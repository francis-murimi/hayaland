use actix_web::http::StatusCode;
use actix_web::{http::header, test, web::Data};
use api::routes;
use api::AppState;
use application::agreements::{
    AdminUpdateAgreement, GenerateAgreement, GetAgreement, SignAgreement,
};
use application::deals::dto::SetValueDistributionCommand;
use application::deals::{
    AcceptTerm, CounterTerm, CreateDeal, ExecuteTransition, GetDeal, GetValueDistribution,
    ListDeals, ListTerms, ProposeTerm, RejectTerm, SetValueDistribution, SubmitDeal, UpdateDeal,
    ValidateDeal, WithdrawTerm,
};
use application::email::dto::VerifyEmailCommand;
use application::email::queue::{EmailQueue, EmailQueueItem};
use application::email::resend_verification::ResendVerificationEmail;
use application::email::verify_email::VerifyEmail;
use application::milestones::{
    CompleteMilestone, CreateMilestone, DeleteMilestone, GetDealProgress, ListMilestones,
    StartMilestone, UpdateMilestone, VerifyMilestone,
};
use application::parties::{
    AddPartyRole, CreateParty, GetParty, ListMyParties, ListPartyRoles, RemovePartyRole,
    SearchParties, SoftDeleteParty, UpdateParty,
};
use application::password_reset::request_password_reset::RequestPasswordReset;
use application::password_reset::reset_password::ResetPassword;
use application::payments::{
    ApproveTransaction, CreateWallet, DeductFee, DepositPoints, GetDealWallet, GetTransaction,
    GetWallet, HoldEscrow, ListDealTransactions, ListPendingApprovals, ListWalletTransactions,
    RecordAdjustment, ReleaseEscrow, WithdrawPoints,
};
use application::roles::assign_user_roles::AssignUserRoles;
use application::roles::list_roles::ListRoles;
use application::roles::update_role_scopes::UpdateRoleScopes;
use application::users::authenticate_user::AuthenticateUser;
use application::users::create_user::{CreateUser, PasswordHasher};
use application::users::deactivate_user::DeactivateUser;
use application::users::get_user::GetUser;
use application::users::list_users::ListUsers;
use application::users::token::{AuthContext, TokenGenerator, TokenVerifier};
use application::users::update_user::UpdateUser;
use async_trait::async_trait;
use domain::entities::{
    Agreement, ApprovalDecision, Currency, DealRole, DealStatus, DealWallet, DistributionModel,
    Email, EmailVerification, Milestone, PasswordHash, PasswordResetToken, PlatformWallet, Role,
    RoleProfile, Signature, TransactionApproval, TransactionStatus, TransactionType, User,
    Username,
};
use domain::entities::{Party, PartyType, UserPartyMembership};
use domain::errors::DomainError;
use domain::repositories::PartySearchCriteria;
use domain::repositories::{
    AgreementRepository, DealAggregate, DealListResult, DealRepository, DealSearchCriteria,
    EmailVerificationRepository, MilestoneRepository, PartyRepository, PasswordResetRepository,
    RoleRepository, TransactionFilters, UserRepository, WalletRepository,
};
use domain::services::ValidationConfig;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Once};
use uuid::Uuid;

static INIT_TRACING: Once = Once::new();

fn init_test_tracing() {
    INIT_TRACING.call_once(|| {
        let _ = tracing_subscriber::fmt()
            .with_env_filter("info")
            .with_test_writer()
            .try_init();
    });
}

struct FakeRepo {
    users: Mutex<HashMap<Uuid, User>>,
}

#[async_trait]
impl UserRepository for FakeRepo {
    async fn create(&self, user: &User) -> Result<(), DomainError> {
        self.users.lock().unwrap().insert(user.id, user.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, DomainError> {
        Ok(self.users.lock().unwrap().get(&id).cloned())
    }

    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, DomainError> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .values()
            .find(|u| u.email == *email)
            .cloned())
    }

    async fn find_by_username(&self, username: &Username) -> Result<Option<User>, DomainError> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .values()
            .find(|u| u.username == *username)
            .cloned())
    }

    async fn list(
        &self,
        limit: i64,
        offset: i64,
        active_only: Option<bool>,
    ) -> Result<Vec<User>, DomainError> {
        let users: Vec<User> = self.users.lock().unwrap().values().cloned().collect();
        let filtered = match active_only {
            Some(true) => users
                .into_iter()
                .filter(|u| u.is_active)
                .collect::<Vec<_>>(),
            Some(false) => users
                .into_iter()
                .filter(|u| !u.is_active)
                .collect::<Vec<_>>(),
            None => users,
        };
        let start = offset as usize;
        let end = (offset + limit) as usize;
        Ok(filtered.into_iter().skip(start).take(end - start).collect())
    }

    async fn update(&self, user: &User) -> Result<(), DomainError> {
        self.users.lock().unwrap().insert(user.id, user.clone());
        Ok(())
    }

    async fn count(&self) -> Result<i64, DomainError> {
        Ok(self.users.lock().unwrap().len() as i64)
    }
}

struct FakeHasher;

#[async_trait]
impl PasswordHasher for FakeHasher {
    async fn hash_password(
        &self,
        password: &str,
    ) -> Result<String, application::errors::ApplicationError> {
        Ok(format!("hashed:{password}"))
    }

    async fn verify_password(
        &self,
        password: &str,
        hash: &str,
    ) -> Result<bool, application::errors::ApplicationError> {
        Ok(hash == format!("hashed:{password}"))
    }
}

struct FakeTokenService {
    repo: Arc<dyn UserRepository>,
    role_repo: Arc<dyn RoleRepository>,
}

#[async_trait]
impl TokenGenerator for FakeTokenService {
    async fn generate(
        &self,
        ctx: &AuthContext,
    ) -> Result<String, application::errors::ApplicationError> {
        Ok(format!("token-{}", ctx.user_id))
    }
}

#[async_trait]
impl TokenVerifier for FakeTokenService {
    async fn verify(
        &self,
        token: &str,
    ) -> Result<AuthContext, application::errors::ApplicationError> {
        let user_id = token
            .strip_prefix("token-")
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or(application::errors::ApplicationError::Unauthorized)?;

        let user = self
            .repo
            .find_by_id(user_id)
            .await
            .map_err(|_| application::errors::ApplicationError::Unauthorized)?
            .ok_or(application::errors::ApplicationError::Unauthorized)?;

        let mut scopes = std::collections::HashSet::new();
        for role in &user.roles {
            if let Ok(Some(def)) = self.role_repo.find_by_name(role).await {
                scopes.extend(def.scopes);
            }
        }
        let mut scopes: Vec<_> = scopes.into_iter().collect();
        scopes.sort();

        Ok(AuthContext {
            user_id,
            roles: user.roles,
            scopes,
        })
    }
}

struct FakeRoleRepo {
    roles: Mutex<HashMap<String, Role>>,
}

#[async_trait]
impl RoleRepository for FakeRoleRepo {
    async fn find_by_name(&self, name: &str) -> Result<Option<Role>, DomainError> {
        Ok(self.roles.lock().unwrap().get(name).cloned())
    }

    async fn list(&self) -> Result<Vec<Role>, DomainError> {
        Ok(self.roles.lock().unwrap().values().cloned().collect())
    }

    async fn save(&self, role: &Role) -> Result<(), DomainError> {
        self.roles
            .lock()
            .unwrap()
            .insert(role.name.clone(), role.clone());
        Ok(())
    }

    async fn delete(&self, _name: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

#[derive(Default)]
struct FakeEmailVerificationRepo {
    verifications: Mutex<HashMap<String, EmailVerification>>,
}

#[async_trait]
impl EmailVerificationRepository for FakeEmailVerificationRepo {
    async fn save(&self, verification: &EmailVerification) -> Result<(), DomainError> {
        self.verifications
            .lock()
            .unwrap()
            .insert(verification.token.clone(), verification.clone());
        Ok(())
    }

    async fn find_by_token(&self, token: &str) -> Result<Option<EmailVerification>, DomainError> {
        Ok(self.verifications.lock().unwrap().get(token).cloned())
    }

    async fn mark_used(&self, token: &str) -> Result<(), DomainError> {
        if let Some(v) = self.verifications.lock().unwrap().get_mut(token) {
            v.used = true;
        }
        Ok(())
    }

    async fn invalidate_unused_for_user(&self, user_id: Uuid) -> Result<(), DomainError> {
        for v in self.verifications.lock().unwrap().values_mut() {
            if v.user_id == user_id && !v.used {
                v.used = true;
            }
        }
        Ok(())
    }
}

#[derive(Default)]
struct FakePasswordResetRepo {
    tokens: Mutex<HashMap<String, PasswordResetToken>>,
}

#[async_trait]
impl PasswordResetRepository for FakePasswordResetRepo {
    async fn save(&self, token: &PasswordResetToken) -> Result<(), DomainError> {
        self.tokens
            .lock()
            .unwrap()
            .insert(token.token.clone(), token.clone());
        Ok(())
    }

    async fn find_by_token(&self, token: &str) -> Result<Option<PasswordResetToken>, DomainError> {
        Ok(self.tokens.lock().unwrap().get(token).cloned())
    }

    async fn mark_used(&self, token: &str) -> Result<(), DomainError> {
        if let Some(t) = self.tokens.lock().unwrap().get_mut(token) {
            t.used = true;
        }
        Ok(())
    }

    async fn invalidate_unused_for_user(&self, user_id: Uuid) -> Result<(), DomainError> {
        for t in self.tokens.lock().unwrap().values_mut() {
            if t.user_id == user_id && !t.used {
                t.used = true;
            }
        }
        Ok(())
    }
}

#[derive(Default)]
struct FakeEmailQueue {
    items: Mutex<Vec<(String, String, String)>>,
}

#[derive(Default)]
struct FakePartyRepo {
    parties: Mutex<HashMap<Uuid, Party>>,
    memberships: Mutex<Vec<UserPartyMembership>>,
    roles: Mutex<Vec<(Uuid, DealRole, RoleProfile)>>,
}

fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let earth_radius_km = 6371.0;
    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lon / 2.0).sin().powi(2);
    2.0 * earth_radius_km * a.sqrt().atan2((1.0 - a).sqrt())
}

#[async_trait]
impl PartyRepository for FakePartyRepo {
    async fn create(&self, party: &Party) -> Result<(), DomainError> {
        self.parties.lock().unwrap().insert(party.id, party.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Party>, DomainError> {
        Ok(self.parties.lock().unwrap().get(&id).cloned())
    }

    async fn find_by_email(&self, email: &Email) -> Result<Option<Party>, DomainError> {
        Ok(self
            .parties
            .lock()
            .unwrap()
            .values()
            .find(|p| p.email == *email)
            .cloned())
    }

    async fn update(&self, party: &Party) -> Result<(), DomainError> {
        self.parties.lock().unwrap().insert(party.id, party.clone());
        Ok(())
    }

    async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError> {
        if let Some(p) = self.parties.lock().unwrap().get_mut(&id) {
            p.is_active = false;
        }
        Ok(())
    }

    async fn list(&self, criteria: &PartySearchCriteria) -> Result<Vec<Party>, DomainError> {
        let mut parties: Vec<Party> = self.parties.lock().unwrap().values().cloned().collect();
        if let Some(q) = &criteria.query {
            let q = q.to_lowercase();
            parties.retain(|p| {
                p.display_name.as_str().to_lowercase().contains(&q)
                    || p.email.as_str().to_lowercase().contains(&q)
            });
        }
        if let Some(true) = criteria.active_only {
            parties.retain(|p| p.is_active);
        }
        if let Some(min) = criteria.min_trust_score {
            parties.retain(|p| p.trust_score >= min);
        }
        if let Some(max) = criteria.max_trust_score {
            parties.retain(|p| p.trust_score <= max);
        }
        if let (Some(lat), Some(lng), Some(radius)) =
            (criteria.latitude, criteria.longitude, criteria.radius_km)
        {
            parties.retain(|p| {
                p.location
                    .map(|loc| haversine_km(loc.latitude, loc.longitude, lat, lng) <= radius)
                    .unwrap_or(false)
            });
        }
        Ok(parties)
    }

    async fn count(&self, criteria: &PartySearchCriteria) -> Result<i64, DomainError> {
        self.list(criteria).await.map(|p| p.len() as i64)
    }

    async fn add_role(
        &self,
        party_id: Uuid,
        role: DealRole,
        profile: RoleProfile,
    ) -> Result<(), DomainError> {
        let mut roles = self.roles.lock().unwrap();
        if let Some(entry) = roles
            .iter_mut()
            .find(|(pid, r, _)| *pid == party_id && *r == role)
        {
            entry.2 = profile;
        } else {
            roles.push((party_id, role, profile));
        }
        Ok(())
    }

    async fn remove_role(&self, party_id: Uuid, role: DealRole) -> Result<(), DomainError> {
        self.roles
            .lock()
            .unwrap()
            .retain(|(pid, r, _)| !(*pid == party_id && *r == role));
        Ok(())
    }

    async fn list_roles(
        &self,
        party_id: Uuid,
    ) -> Result<Vec<(DealRole, RoleProfile)>, DomainError> {
        Ok(self
            .roles
            .lock()
            .unwrap()
            .iter()
            .filter(|(pid, _, _)| *pid == party_id)
            .map(|(_, r, p)| (*r, p.clone()))
            .collect())
    }

    async fn has_role(&self, party_id: Uuid, role: DealRole) -> Result<bool, DomainError> {
        Ok(self
            .roles
            .lock()
            .unwrap()
            .iter()
            .any(|(pid, r, _)| *pid == party_id && *r == role))
    }

    async fn count_active_deals_for_role(
        &self,
        _party_id: Uuid,
        _role: DealRole,
    ) -> Result<i64, DomainError> {
        Ok(0)
    }

    async fn count_active_deals(&self, _party_id: Uuid) -> Result<i64, DomainError> {
        Ok(0)
    }

    async fn add_membership(&self, membership: &UserPartyMembership) -> Result<(), DomainError> {
        self.memberships.lock().unwrap().push(membership.clone());
        Ok(())
    }

    async fn list_memberships_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<(UserPartyMembership, Party)>, DomainError> {
        let parties = self.parties.lock().unwrap();
        Ok(self
            .memberships
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.user_id == user_id)
            .map(|m| {
                (
                    m.clone(),
                    parties.get(&m.party_id).cloned().expect("party exists"),
                )
            })
            .collect())
    }

    async fn find_membership(
        &self,
        user_id: Uuid,
        party_id: Uuid,
    ) -> Result<Option<UserPartyMembership>, DomainError> {
        Ok(self
            .memberships
            .lock()
            .unwrap()
            .iter()
            .find(|m| m.user_id == user_id && m.party_id == party_id)
            .cloned())
    }

    async fn touch(&self, _id: Uuid, _updated_at: time::OffsetDateTime) -> Result<(), DomainError> {
        Ok(())
    }

    async fn is_user_member_of_party(
        &self,
        user_id: Uuid,
        party_id: Uuid,
    ) -> Result<bool, DomainError> {
        Ok(self
            .memberships
            .lock()
            .unwrap()
            .iter()
            .any(|m| m.user_id == user_id && m.party_id == party_id && m.is_active))
    }
}

#[derive(Default)]
#[allow(clippy::type_complexity)]
struct FakeDealRepo {
    deals: Mutex<HashMap<Uuid, domain::entities::Deal>>,
    participations: Mutex<Vec<domain::entities::DealParticipation>>,
    terms: Mutex<Vec<domain::entities::Term>>,
    value_distributions: Mutex<HashMap<Uuid, domain::entities::ValueDistribution>>,
    history: Mutex<Vec<(Uuid, String, Option<Uuid>, Option<serde_json::Value>)>>,
    reference_counter: Mutex<i64>,
}

#[async_trait]
impl DealRepository for FakeDealRepo {
    async fn create(&self, aggregate: &DealAggregate) -> Result<(), DomainError> {
        self.deals
            .lock()
            .unwrap()
            .insert(aggregate.deal.id, aggregate.deal.clone());
        for p in &aggregate.participations {
            self.participations.lock().unwrap().push(p.clone());
        }
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<domain::entities::Deal>, DomainError> {
        Ok(self.deals.lock().unwrap().get(&id).cloned())
    }

    async fn find_aggregate_by_id(&self, id: Uuid) -> Result<Option<DealAggregate>, DomainError> {
        let deal = self.find_by_id(id).await?;
        match deal {
            Some(d) => {
                let participations = self
                    .participations
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|p| p.deal_id == id)
                    .cloned()
                    .collect();
                Ok(Some(DealAggregate {
                    deal: d,
                    participations,
                }))
            }
            None => Ok(None),
        }
    }

    async fn find_deals_by_status(
        &self,
        status: domain::entities::DealStatus,
        entered_before: time::OffsetDateTime,
        limit: i64,
    ) -> Result<Vec<domain::entities::Deal>, DomainError> {
        let deals: Vec<_> = self
            .deals
            .lock()
            .unwrap()
            .values()
            .filter(|d| d.deal_status == status && d.current_state_entered_at < entered_before)
            .cloned()
            .collect();
        let mut deals: Vec<_> = deals.into_iter().take(limit as usize).collect();
        deals.sort_by(|a, b| a.current_state_entered_at.cmp(&b.current_state_entered_at));
        Ok(deals)
    }

    async fn find_participations_by_deal(
        &self,
        deal_id: Uuid,
    ) -> Result<Vec<domain::entities::DealParticipation>, DomainError> {
        Ok(self
            .participations
            .lock()
            .unwrap()
            .iter()
            .filter(|p| p.deal_id == deal_id)
            .cloned()
            .collect())
    }

    async fn update(&self, deal: &domain::entities::Deal) -> Result<(), DomainError> {
        self.deals.lock().unwrap().insert(deal.id, deal.clone());
        Ok(())
    }

    async fn update_participation(
        &self,
        participation: &domain::entities::DealParticipation,
    ) -> Result<(), DomainError> {
        let mut participations = self.participations.lock().unwrap();
        if let Some(p) = participations.iter_mut().find(|p| p.id == participation.id) {
            *p = participation.clone();
        }
        Ok(())
    }

    async fn list(&self, _criteria: &DealSearchCriteria) -> Result<DealListResult, DomainError> {
        let deals: Vec<domain::entities::Deal> =
            self.deals.lock().unwrap().values().cloned().collect();
        let total = deals.len() as i64;
        Ok(DealListResult {
            deals,
            total,
            limit: 20,
            offset: 0,
        })
    }

    async fn count_active_deals_for_party(&self, _party_id: Uuid) -> Result<i64, DomainError> {
        Ok(0)
    }

    async fn count_active_deals_for_party_role(
        &self,
        _party_id: Uuid,
        _role: domain::entities::DealRole,
    ) -> Result<i64, DomainError> {
        Ok(0)
    }

    async fn record_history(
        &self,
        deal_id: Uuid,
        event_type: &str,
        actor_party_id: Option<Uuid>,
        details: Option<serde_json::Value>,
    ) -> Result<(), DomainError> {
        self.history.lock().unwrap().push((
            deal_id,
            event_type.to_string(),
            actor_party_id,
            details,
        ));
        Ok(())
    }

    async fn is_party_participant(
        &self,
        deal_id: Uuid,
        party_id: Uuid,
    ) -> Result<bool, DomainError> {
        Ok(self
            .participations
            .lock()
            .unwrap()
            .iter()
            .any(|p| p.deal_id == deal_id && p.party_id == party_id))
    }

    async fn next_deal_reference(&self) -> Result<String, DomainError> {
        let mut counter = self.reference_counter.lock().unwrap();
        *counter += 1;
        Ok(format!("DL-2026-{:04}", *counter))
    }

    async fn update_value_totals(
        &self,
        deal_id: Uuid,
        total_value: rust_decimal::Decimal,
        platform_fee_percentage: rust_decimal::Decimal,
        platform_fee_amount: rust_decimal::Decimal,
    ) -> Result<(), DomainError> {
        if let Some(deal) = self.deals.lock().unwrap().get_mut(&deal_id) {
            deal.total_deal_value = Some(total_value);
            deal.platform_fee_percentage = platform_fee_percentage;
            deal.platform_fee_amount = platform_fee_amount;
        }
        Ok(())
    }

    async fn create_term(&self, term: &domain::entities::Term) -> Result<(), DomainError> {
        self.terms.lock().unwrap().push(term.clone());
        Ok(())
    }

    async fn update_term(&self, term: &domain::entities::Term) -> Result<(), DomainError> {
        let mut terms = self.terms.lock().unwrap();
        if let Some(t) = terms.iter_mut().find(|t| t.id == term.id) {
            *t = term.clone();
        }
        Ok(())
    }

    async fn find_term_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<domain::entities::Term>, DomainError> {
        Ok(self
            .terms
            .lock()
            .unwrap()
            .iter()
            .find(|t| t.id == id)
            .cloned())
    }

    async fn find_terms_by_deal(
        &self,
        deal_id: Uuid,
    ) -> Result<Vec<domain::entities::Term>, DomainError> {
        Ok(self
            .terms
            .lock()
            .unwrap()
            .iter()
            .filter(|t| t.deal_id == deal_id)
            .cloned()
            .collect())
    }

    async fn set_value_distribution(
        &self,
        distribution: &domain::entities::ValueDistribution,
    ) -> Result<(), DomainError> {
        self.value_distributions
            .lock()
            .unwrap()
            .insert(distribution.deal_id, distribution.clone());
        Ok(())
    }

    async fn find_value_distribution_by_deal(
        &self,
        deal_id: Uuid,
    ) -> Result<Option<domain::entities::ValueDistribution>, DomainError> {
        Ok(self
            .value_distributions
            .lock()
            .unwrap()
            .get(&deal_id)
            .cloned())
    }
}

struct FakeAgreementRepo {
    agreements: Mutex<HashMap<Uuid, Agreement>>,
    signatures: Mutex<Vec<Signature>>,
}

#[async_trait]
impl AgreementRepository for FakeAgreementRepo {
    async fn create(&self, agreement: &Agreement) -> Result<(), DomainError> {
        self.agreements
            .lock()
            .unwrap()
            .insert(agreement.id, agreement.clone());
        Ok(())
    }

    async fn find_by_deal_id(&self, deal_id: Uuid) -> Result<Option<Agreement>, DomainError> {
        Ok(self
            .agreements
            .lock()
            .unwrap()
            .values()
            .find(|a| a.deal_id == deal_id)
            .cloned())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Agreement>, DomainError> {
        Ok(self.agreements.lock().unwrap().get(&id).cloned())
    }

    async fn update(&self, agreement: &Agreement) -> Result<(), DomainError> {
        self.agreements
            .lock()
            .unwrap()
            .insert(agreement.id, agreement.clone());
        Ok(())
    }

    async fn create_signature(&self, signature: &Signature) -> Result<(), DomainError> {
        self.signatures.lock().unwrap().push(signature.clone());
        Ok(())
    }

    async fn find_signatures_by_agreement(
        &self,
        agreement_id: Uuid,
    ) -> Result<Vec<Signature>, DomainError> {
        Ok(self
            .signatures
            .lock()
            .unwrap()
            .iter()
            .filter(|s| s.agreement_id == agreement_id)
            .cloned()
            .collect())
    }

    async fn has_party_signed(
        &self,
        agreement_id: Uuid,
        party_id: Uuid,
    ) -> Result<bool, DomainError> {
        let version = self
            .agreements
            .lock()
            .unwrap()
            .get(&agreement_id)
            .map(|a| a.version)
            .unwrap_or(0);
        Ok(self.signatures.lock().unwrap().iter().any(|s| {
            s.agreement_id == agreement_id && s.party_id == party_id && s.version == version
        }))
    }

    async fn count_signatures(&self, agreement_id: Uuid) -> Result<i64, DomainError> {
        let version = self
            .agreements
            .lock()
            .unwrap()
            .get(&agreement_id)
            .map(|a| a.version)
            .unwrap_or(0);
        Ok(self
            .signatures
            .lock()
            .unwrap()
            .iter()
            .filter(|s| s.agreement_id == agreement_id && s.version == version)
            .count() as i64)
    }
}

#[async_trait]
impl EmailQueue for FakeEmailQueue {
    async fn enqueue(
        &self,
        item: EmailQueueItem,
    ) -> Result<(), application::errors::ApplicationError> {
        self.items
            .lock()
            .unwrap()
            .push((item.to, item.subject, item.body));
        Ok(())
    }
}

#[derive(Default)]
struct FakeWalletRepo {
    wallets: Mutex<HashMap<Uuid, PlatformWallet>>,
    transactions: Mutex<Vec<domain::entities::Transaction>>,
    approvals: Mutex<Vec<TransactionApproval>>,
}

#[async_trait]
impl WalletRepository for FakeWalletRepo {
    async fn create(&self, wallet: &PlatformWallet) -> Result<(), DomainError> {
        self.wallets
            .lock()
            .unwrap()
            .insert(wallet.party_id, wallet.clone());
        Ok(())
    }

    async fn find_by_party_id(
        &self,
        party_id: Uuid,
    ) -> Result<Option<PlatformWallet>, DomainError> {
        Ok(self.wallets.lock().unwrap().get(&party_id).cloned())
    }

    async fn update(&self, wallet: &PlatformWallet) -> Result<(), DomainError> {
        self.wallets
            .lock()
            .unwrap()
            .insert(wallet.party_id, wallet.clone());
        Ok(())
    }

    async fn record_transaction(
        &self,
        wallet: &PlatformWallet,
        transaction: &domain::entities::Transaction,
    ) -> Result<(), DomainError> {
        self.wallets
            .lock()
            .unwrap()
            .insert(wallet.party_id, wallet.clone());
        self.transactions.lock().unwrap().push(transaction.clone());
        Ok(())
    }

    async fn find_transactions(
        &self,
        party_id: Uuid,
        filters: &TransactionFilters,
    ) -> Result<Vec<domain::entities::Transaction>, DomainError> {
        let txns = self.transactions.lock().unwrap();
        let mut results: Vec<domain::entities::Transaction> = txns
            .iter()
            .filter(|t| t.from_party_id == Some(party_id) || t.to_party_id == Some(party_id))
            .filter(|t| filters.deal_id.is_none_or(|d| t.deal_id == d))
            .filter(|t| {
                filters
                    .status
                    .as_ref()
                    .is_none_or(|s| t.status.as_str() == s)
            })
            .filter(|t| {
                filters
                    .transaction_type
                    .as_ref()
                    .is_none_or(|tt| t.transaction_type.as_str() == tt)
            })
            .cloned()
            .collect();
        results.sort_by_key(|a| a.created_at);
        let start = filters.offset as usize;
        let end = (filters.offset + filters.limit) as usize;
        Ok(results
            .into_iter()
            .skip(start)
            .take(end.saturating_sub(start))
            .collect())
    }

    async fn count_transactions(
        &self,
        party_id: Uuid,
        filters: &TransactionFilters,
    ) -> Result<i64, DomainError> {
        let count = self
            .transactions
            .lock()
            .unwrap()
            .iter()
            .filter(|t| t.from_party_id == Some(party_id) || t.to_party_id == Some(party_id))
            .filter(|t| filters.deal_id.is_none_or(|d| t.deal_id == d))
            .filter(|t| {
                filters
                    .status
                    .as_ref()
                    .is_none_or(|s| t.status.as_str() == s)
            })
            .filter(|t| {
                filters
                    .transaction_type
                    .as_ref()
                    .is_none_or(|tt| t.transaction_type.as_str() == tt)
            })
            .count() as i64;
        Ok(count)
    }

    async fn compute_deal_wallet(
        &self,
        party_id: Uuid,
        deal_id: Uuid,
    ) -> Result<Option<DealWallet>, DomainError> {
        let txns = self.transactions.lock().unwrap();
        let has_activity = txns.iter().any(|t| {
            t.deal_id == deal_id
                && (t.from_party_id == Some(party_id) || t.to_party_id == Some(party_id))
        });
        if !has_activity {
            return Ok(None);
        }

        let mut dw = DealWallet::new(party_id, deal_id, Currency::Points);
        for t in txns.iter() {
            if t.deal_id != deal_id {
                continue;
            }
            match t.transaction_type {
                TransactionType::Deposit if t.to_party_id == Some(party_id) => {
                    dw.deposited += t.amount;
                    dw.contributed += t.amount;
                }
                TransactionType::Withdrawal if t.from_party_id == Some(party_id) => {
                    dw.withdrawn += t.amount;
                    dw.contributed -= t.amount;
                }
                TransactionType::EscrowHold if t.from_party_id == Some(party_id) => {
                    dw.held_in_escrow += t.amount;
                    dw.contributed += t.amount;
                }
                TransactionType::EscrowRelease if t.to_party_id == Some(party_id) => {
                    dw.released += t.amount;
                    dw.held_in_escrow -= t.amount;
                }
                TransactionType::Fee if t.from_party_id == Some(party_id) => {
                    dw.fees_paid += t.amount;
                }
                TransactionType::Adjustment if t.to_party_id == Some(party_id) => {
                    dw.released += t.amount;
                }
                TransactionType::Adjustment if t.from_party_id == Some(party_id) => {
                    dw.contributed += t.amount;
                }
                _ => {}
            }
        }
        dw.net_position = dw.released + dw.withdrawn - dw.fees_paid - dw.contributed;
        Ok(Some(dw))
    }

    async fn record_pending_transaction(
        &self,
        transaction: &domain::entities::Transaction,
    ) -> Result<(), DomainError> {
        self.transactions.lock().unwrap().push(transaction.clone());
        Ok(())
    }

    async fn find_transaction_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<domain::entities::Transaction>, DomainError> {
        Ok(self
            .transactions
            .lock()
            .unwrap()
            .iter()
            .find(|t| t.id == id)
            .cloned())
    }

    async fn find_approvals_for_transaction(
        &self,
        transaction_id: Uuid,
    ) -> Result<Vec<TransactionApproval>, DomainError> {
        Ok(self
            .approvals
            .lock()
            .unwrap()
            .iter()
            .filter(|a| a.transaction_id == transaction_id)
            .cloned()
            .collect())
    }

    async fn record_approval_and_finalise(
        &self,
        transaction: &domain::entities::Transaction,
        approval: &TransactionApproval,
        wallet_mutations: &[(Uuid, PlatformWallet)],
    ) -> Result<(), DomainError> {
        self.approvals.lock().unwrap().push(approval.clone());

        let mut txns = self.transactions.lock().unwrap();
        let stored = txns.iter_mut().find(|t| t.id == transaction.id);
        if let Some(t) = stored {
            t.approvals_received = transaction.approvals_received + 1;
            match approval.decision {
                ApprovalDecision::Rejected => {
                    t.status = TransactionStatus::Rejected;
                }
                ApprovalDecision::Approved
                    if t.approvals_received >= transaction.approvals_required =>
                {
                    t.status = TransactionStatus::Verified;
                    t.executed_at = Some(time::OffsetDateTime::now_utc());
                }
                ApprovalDecision::Approved => {}
            }
        }
        drop(txns);

        let mut wallets = self.wallets.lock().unwrap();
        for (party_id, wallet) in wallet_mutations {
            wallets.insert(*party_id, wallet.clone());
        }
        Ok(())
    }

    async fn find_pending_transactions_for_party(
        &self,
        party_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<domain::entities::Transaction>, DomainError> {
        let approvals = self.approvals.lock().unwrap();
        let voted: Vec<Uuid> = approvals
            .iter()
            .filter(|a| a.party_id == party_id)
            .map(|a| a.transaction_id)
            .collect();
        drop(approvals);

        let txns = self.transactions.lock().unwrap();
        let mut results: Vec<domain::entities::Transaction> = txns
            .iter()
            .filter(|t| t.status == TransactionStatus::Pending && t.requires_approval)
            .filter(|t| t.involved_party_ids.contains(&party_id))
            .filter(|t| !voted.contains(&t.id))
            .cloned()
            .collect();
        results.sort_by_key(|a| a.created_at);
        let start = offset as usize;
        let end = (offset + limit) as usize;
        Ok(results
            .into_iter()
            .skip(start)
            .take(end.saturating_sub(start))
            .collect())
    }

    async fn count_pending_transactions_for_party(
        &self,
        party_id: Uuid,
    ) -> Result<i64, DomainError> {
        let approvals = self.approvals.lock().unwrap();
        let voted: Vec<Uuid> = approvals
            .iter()
            .filter(|a| a.party_id == party_id)
            .map(|a| a.transaction_id)
            .collect();
        drop(approvals);

        let count = self
            .transactions
            .lock()
            .unwrap()
            .iter()
            .filter(|t| t.status == TransactionStatus::Pending && t.requires_approval)
            .filter(|t| t.involved_party_ids.contains(&party_id))
            .filter(|t| !voted.contains(&t.id))
            .count() as i64;
        Ok(count)
    }
}

#[derive(Default)]
struct FakeMilestoneRepo {
    milestones: Mutex<Vec<Milestone>>,
}

#[async_trait]
impl MilestoneRepository for FakeMilestoneRepo {
    async fn create(&self, milestone: &Milestone) -> Result<(), DomainError> {
        self.milestones.lock().unwrap().push(milestone.clone());
        Ok(())
    }

    async fn update(&self, milestone: &Milestone) -> Result<(), DomainError> {
        let mut milestones = self.milestones.lock().unwrap();
        let idx = milestones
            .iter()
            .position(|m| m.id == milestone.id)
            .ok_or(DomainError::MilestoneNotFound)?;
        milestones[idx] = milestone.clone();
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let mut milestones = self.milestones.lock().unwrap();
        let idx = milestones
            .iter()
            .position(|m| m.id == id)
            .ok_or(DomainError::MilestoneNotFound)?;
        milestones.remove(idx);
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Milestone>, DomainError> {
        Ok(self
            .milestones
            .lock()
            .unwrap()
            .iter()
            .find(|m| m.id == id)
            .cloned())
    }

    async fn find_by_deal(
        &self,
        deal_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Milestone>, DomainError> {
        let milestones = self.milestones.lock().unwrap();
        let mut matched: Vec<Milestone> = milestones
            .iter()
            .filter(|m| m.deal_id == deal_id)
            .cloned()
            .collect();
        matched.sort_by(|a, b| {
            a.display_order
                .cmp(&b.display_order)
                .then_with(|| a.created_at.cmp(&b.created_at))
        });
        let start = offset as usize;
        let end = (offset + limit) as usize;
        Ok(matched
            .into_iter()
            .skip(start)
            .take(end.saturating_sub(start))
            .collect())
    }

    async fn count_by_deal(&self, deal_id: Uuid) -> Result<i64, DomainError> {
        Ok(self
            .milestones
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.deal_id == deal_id)
            .count() as i64)
    }

    async fn count_verified_by_deal(&self, deal_id: Uuid) -> Result<i64, DomainError> {
        use domain::entities::MilestoneStatus;
        Ok(self
            .milestones
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.deal_id == deal_id && m.milestone_status == MilestoneStatus::Verified)
            .count() as i64)
    }

    async fn count_by_status(&self, deal_id: Uuid, status: &str) -> Result<i64, DomainError> {
        let target = domain::entities::MilestoneStatus::try_from(status)?;
        Ok(self
            .milestones
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.deal_id == deal_id && m.milestone_status == target)
            .count() as i64)
    }
}

struct TestFixtures {
    state: AppState,
    repo: Arc<FakeRepo>,
    queue: Arc<FakeEmailQueue>,
    deal_repo: Arc<FakeDealRepo>,
}

fn seeded_role_repo() -> Arc<FakeRoleRepo> {
    Arc::new(FakeRoleRepo {
        roles: Mutex::new(HashMap::from([
            (
                "user".to_string(),
                Role::builtin(
                    "user",
                    vec![
                        "users:read".to_string(),
                        "users:write".to_string(),
                        "parties:read".to_string(),
                        "parties:write".to_string(),
                        "deals:read".to_string(),
                        "deals:write".to_string(),
                        "payments:read".to_string(),
                        "payments:write".to_string(),
                    ],
                ),
            ),
            (
                "admin".to_string(),
                Role::builtin(
                    "admin",
                    vec![
                        "users:read".to_string(),
                        "users:write".to_string(),
                        "users:admin".to_string(),
                        "users:delete".to_string(),
                        "admin:deals".to_string(),
                        "admin:transactions".to_string(),
                        "admin:milestones".to_string(),
                        "admin:*".to_string(),
                    ],
                ),
            ),
        ])),
    })
}

fn test_fixtures() -> TestFixtures {
    let repo: Arc<FakeRepo> = Arc::new(FakeRepo {
        users: Mutex::new(HashMap::new()),
    });
    let verification_repo: Arc<FakeEmailVerificationRepo> =
        Arc::new(FakeEmailVerificationRepo::default());
    let password_reset_repo: Arc<FakePasswordResetRepo> =
        Arc::new(FakePasswordResetRepo::default());
    let role_repo: Arc<dyn RoleRepository> = seeded_role_repo();
    let hasher: Arc<dyn PasswordHasher> = Arc::new(FakeHasher);
    let queue: Arc<FakeEmailQueue> = Arc::new(FakeEmailQueue::default());
    let party_repo: Arc<FakePartyRepo> = Arc::new(FakePartyRepo::default());
    let deal_repo: Arc<FakeDealRepo> = Arc::new(FakeDealRepo::default());
    let agreement_repo: Arc<FakeAgreementRepo> = Arc::new(FakeAgreementRepo {
        agreements: Mutex::new(HashMap::new()),
        signatures: Mutex::new(Vec::new()),
    });
    let wallet_repo: Arc<dyn WalletRepository> = Arc::new(FakeWalletRepo::default());
    let milestone_repo: Arc<dyn MilestoneRepository> = Arc::new(FakeMilestoneRepo::default());
    let token: Arc<FakeTokenService> = Arc::new(FakeTokenService {
        repo: repo.clone(),
        role_repo: role_repo.clone(),
    });

    let state = AppState {
        create_user: CreateUser::new(
            repo.clone(),
            verification_repo.clone(),
            queue.clone(),
            hasher.clone(),
            "https://app.hayaland.local".to_string(),
            86400,
        ),
        get_user: GetUser::new(repo.clone()),
        list_users: ListUsers::new(repo.clone()),
        update_user: UpdateUser::new(repo.clone()),
        assign_user_roles: AssignUserRoles::new(repo.clone()),
        deactivate_user: DeactivateUser::new(repo.clone()),
        authenticate_user: AuthenticateUser::new(
            repo.clone(),
            role_repo.clone(),
            hasher.clone(),
            token.clone(),
        ),
        verify_email: VerifyEmail::new(repo.clone(), verification_repo.clone()),
        resend_verification_email: ResendVerificationEmail::new(
            repo.clone(),
            verification_repo.clone(),
            queue.clone(),
            "https://app.hayaland.local".to_string(),
            86400,
        ),
        request_password_reset: RequestPasswordReset::new(
            repo.clone(),
            password_reset_repo.clone(),
            queue.clone(),
            "https://app.hayaland.local".to_string(),
            3600,
        ),
        reset_password: ResetPassword::new(repo.clone(), password_reset_repo, hasher.clone()),
        list_roles: ListRoles::new(role_repo.clone()),
        update_role_scopes: UpdateRoleScopes::new(role_repo),
        create_party: CreateParty::new_with_wallet(party_repo.clone(), wallet_repo.clone()),
        get_party: GetParty::new(party_repo.clone()),
        list_my_parties: ListMyParties::new(party_repo.clone()),
        search_parties: SearchParties::new(party_repo.clone()),
        update_party: UpdateParty::new(party_repo.clone()),
        delete_party: SoftDeleteParty::new(party_repo.clone()),
        add_party_role: AddPartyRole::new(party_repo.clone()),
        remove_party_role: RemovePartyRole::new(party_repo.clone()),
        list_party_roles: ListPartyRoles::new(party_repo.clone()),
        create_deal: CreateDeal::new(deal_repo.clone(), party_repo.clone()),
        get_deal: GetDeal::new(deal_repo.clone(), party_repo.clone()),
        list_deals: ListDeals::new(deal_repo.clone(), party_repo.clone()),
        update_deal: UpdateDeal::new(deal_repo.clone(), party_repo.clone()),
        submit_deal: SubmitDeal::new(
            deal_repo.clone(),
            party_repo.clone(),
            ValidationConfig::default(),
        ),
        execute_transition: ExecuteTransition::new_with_milestones(
            deal_repo.clone(),
            party_repo.clone(),
            agreement_repo.clone(),
            milestone_repo.clone(),
            ValidationConfig::default(),
        ),
        propose_term: ProposeTerm::new(deal_repo.clone(), party_repo.clone()),
        counter_term: CounterTerm::new(deal_repo.clone(), party_repo.clone()),
        accept_term: AcceptTerm::new(deal_repo.clone(), party_repo.clone()),
        reject_term: RejectTerm::new(deal_repo.clone(), party_repo.clone()),
        withdraw_term: WithdrawTerm::new(deal_repo.clone(), party_repo.clone()),
        list_terms: ListTerms::new(deal_repo.clone(), party_repo.clone()),
        set_value_distribution: SetValueDistribution::new(deal_repo.clone(), party_repo.clone()),
        get_value_distribution: GetValueDistribution::new(deal_repo.clone(), party_repo.clone()),
        validate_deal: ValidateDeal::new(
            deal_repo.clone(),
            party_repo.clone(),
            ValidationConfig::default(),
        ),
        generate_agreement: GenerateAgreement::new(
            deal_repo.clone(),
            party_repo.clone(),
            agreement_repo.clone(),
        ),
        get_agreement: GetAgreement::new(
            deal_repo.clone(),
            party_repo.clone(),
            agreement_repo.clone(),
        ),
        sign_agreement: SignAgreement::new(
            deal_repo.clone(),
            party_repo.clone(),
            agreement_repo.clone(),
        ),
        admin_update_agreement: AdminUpdateAgreement::new(deal_repo.clone(), agreement_repo),
        create_wallet: CreateWallet::new(wallet_repo.clone()),
        deposit_points: DepositPoints::new(
            party_repo.clone(),
            deal_repo.clone(),
            wallet_repo.clone(),
        ),
        withdraw_points: WithdrawPoints::new(
            party_repo.clone(),
            deal_repo.clone(),
            wallet_repo.clone(),
        ),
        get_wallet: GetWallet::new(party_repo.clone(), wallet_repo.clone()),
        get_deal_wallet: GetDealWallet::new(
            party_repo.clone(),
            deal_repo.clone(),
            wallet_repo.clone(),
        ),
        hold_escrow: HoldEscrow::new(party_repo.clone(), deal_repo.clone(), wallet_repo.clone()),
        release_escrow: ReleaseEscrow::new(
            party_repo.clone(),
            deal_repo.clone(),
            wallet_repo.clone(),
        ),
        deduct_fee: DeductFee::new(party_repo.clone(), deal_repo.clone(), wallet_repo.clone()),
        record_adjustment: RecordAdjustment::new(
            party_repo.clone(),
            deal_repo.clone(),
            wallet_repo.clone(),
        ),
        list_wallet_transactions: ListWalletTransactions::new(
            party_repo.clone(),
            wallet_repo.clone(),
        ),
        list_deal_transactions: ListDealTransactions::new(
            party_repo.clone(),
            deal_repo.clone(),
            wallet_repo.clone(),
        ),
        approve_transaction: ApproveTransaction::new(party_repo.clone(), wallet_repo.clone()),
        list_pending_approvals: ListPendingApprovals::new(party_repo.clone(), wallet_repo.clone()),
        get_transaction: GetTransaction::new(party_repo.clone(), wallet_repo.clone()),
        create_milestone: CreateMilestone::new(
            party_repo.clone(),
            deal_repo.clone(),
            milestone_repo.clone(),
        ),
        list_milestones: ListMilestones::new(
            party_repo.clone(),
            deal_repo.clone(),
            milestone_repo.clone(),
        ),
        get_deal_progress: GetDealProgress::new(
            party_repo.clone(),
            deal_repo.clone(),
            milestone_repo.clone(),
        ),
        update_milestone: UpdateMilestone::new(
            party_repo.clone(),
            deal_repo.clone(),
            milestone_repo.clone(),
        ),
        delete_milestone: DeleteMilestone::new(
            party_repo.clone(),
            deal_repo.clone(),
            milestone_repo.clone(),
        ),
        start_milestone: StartMilestone::new(
            party_repo.clone(),
            deal_repo.clone(),
            milestone_repo.clone(),
        ),
        complete_milestone: CompleteMilestone::new(
            party_repo.clone(),
            deal_repo.clone(),
            milestone_repo.clone(),
        ),
        verify_milestone: VerifyMilestone::new(
            party_repo.clone(),
            deal_repo.clone(),
            milestone_repo.clone(),
            wallet_repo.clone(),
        ),
        token_validator: token,
    };

    TestFixtures {
        state,
        repo,
        queue,
        deal_repo,
    }
}

fn extract_token_for_email(queue: &FakeEmailQueue, email: &str) -> String {
    extract_token_for_email_with_path(queue, email, "/auth/verify-email?token=")
}

fn extract_reset_token_for_email(queue: &FakeEmailQueue, email: &str) -> String {
    extract_token_for_email_with_path(queue, email, "/auth/reset-password?token=")
}

fn extract_token_for_email_with_path(queue: &FakeEmailQueue, email: &str, path: &str) -> String {
    let items = queue.items.lock().unwrap();
    let (_, _, body) = items
        .iter()
        .rev()
        .find(|(to, _, body)| to == email && body.contains(path))
        .expect("email not sent");
    body.split("token=")
        .nth(1)
        .unwrap()
        .split('\n')
        .next()
        .unwrap()
        .to_string()
}

async fn login(fixtures: &TestFixtures, email: &str) -> String {
    let email_obj = Email::new(email).unwrap();
    if fixtures
        .repo
        .find_by_email(&email_obj)
        .await
        .unwrap()
        .is_none()
    {
        fixtures
            .state
            .create_user
            .execute(application::users::dto::CreateUserCommand {
                email: email.to_string(),
                username: email.split('@').next().unwrap().to_string(),
                password: "password123".to_string(),
            })
            .await
            .unwrap();
    }

    let user = fixtures
        .repo
        .find_by_email(&email_obj)
        .await
        .unwrap()
        .unwrap();
    if !user.is_active {
        let token = extract_token_for_email(&fixtures.queue, email);
        fixtures
            .state
            .verify_email
            .execute(VerifyEmailCommand { token })
            .await
            .unwrap();
    }

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state.clone()))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({ "email": email, "password": "password123" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    body["token"].as_str().unwrap().to_string()
}

#[actix_rt::test]
async fn health_returns_ok() {
    init_test_tracing();
    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(test_fixtures().state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get().uri("/api/v1/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_rt::test]
async fn create_user_returns_201() {
    init_test_tracing();
    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(test_fixtures().state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/users")
        .set_json(serde_json::json!({
            "email": "test@example.com",
            "username": "testuser",
            "password": "password123"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("id").is_some());
}

#[actix_rt::test]
async fn create_user_returns_400_for_invalid_input() {
    init_test_tracing();
    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(test_fixtures().state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/users")
        .set_json(serde_json::json!({
            "email": "not-an-email",
            "username": "ab",
            "password": "short"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn get_user_returns_401_when_unauthenticated() {
    init_test_tracing();
    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(test_fixtures().state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/users/{}", Uuid::nil()))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn get_user_returns_401_for_invalid_token() {
    init_test_tracing();
    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(test_fixtures().state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/users/{}", Uuid::nil()))
        .insert_header((header::AUTHORIZATION, "Bearer not-a-token"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn get_user_returns_200_when_authenticated() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let created = fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "get@example.com".to_string(),
            username: "getuser".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&fixtures, "get@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/users/{}", created.id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_rt::test]
async fn get_user_returns_404_when_missing() {
    init_test_tracing();
    let fixtures = test_fixtures();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "missing@example.com".to_string(),
            username: "missing".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&fixtures, "missing@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/users/{}", Uuid::nil()))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_rt::test]
async fn list_users_returns_401_when_unauthenticated() {
    init_test_tracing();
    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(test_fixtures().state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/users?page=1&per_page=10")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn list_users_returns_200_when_authenticated() {
    init_test_tracing();
    let fixtures = test_fixtures();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "list@example.com".to_string(),
            username: "listuser".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&fixtures, "list@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/users?page=1&per_page=10")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_rt::test]
async fn update_user_returns_200_for_owner() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let created = fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "update@example.com".to_string(),
            username: "updateuser".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&fixtures, "update@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/users/{}", created.id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(serde_json::json!({ "username": "updateduser" }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_rt::test]
async fn update_user_returns_403_for_non_owner() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner = fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "owner@example.com".to_string(),
            username: "owner".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "other@example.com".to_string(),
            username: "other".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&fixtures, "other@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/users/{}", owner.id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(serde_json::json!({ "username": "hacked" }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn deactivate_user_returns_200_and_blocks_login() {
    init_test_tracing();
    let fixtures = test_fixtures();
    // First user becomes the protected admin.
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "admin@example.com".to_string(),
            username: "admin".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let created = fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "inactive@example.com".to_string(),
            username: "inactive".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&fixtures, "inactive@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let deactivate = test::TestRequest::delete()
        .uri(&format!("/api/v1/users/{}", created.id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, deactivate).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let login = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "inactive@example.com",
            "password": "password123"
        }))
        .to_request();
    let resp = test::call_service(&app, login).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn deactivate_admin_returns_403() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let admin = fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "admin@example.com".to_string(),
            username: "admin".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&fixtures, "admin@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/users/{}", admin.id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn login_returns_401_for_unverified_user() {
    init_test_tracing();
    let fixtures = test_fixtures();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "unverified@example.com".to_string(),
            username: "unverified".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "unverified@example.com",
            "password": "password123"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn login_returns_200_for_verified_user() {
    init_test_tracing();
    let fixtures = test_fixtures();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "login@example.com".to_string(),
            username: "loginuser".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();

    let token = login(&fixtures, "login@example.com").await;
    assert!(token.starts_with("token-"));
}

#[actix_rt::test]
async fn login_returns_401_for_invalid_credentials() {
    init_test_tracing();
    let fixtures = test_fixtures();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "bad@example.com".to_string(),
            username: "baduser".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let _token = login(&fixtures, "bad@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "bad@example.com",
            "password": "wrongpassword"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn verify_email_activates_user() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let created = fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "verify@example.com".to_string(),
            username: "verify".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();

    let token = extract_token_for_email(&fixtures.queue, "verify@example.com");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/auth/verify-email?token={token}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "verified");
    assert_eq!(body["user_id"], created.id.to_string());

    let user = fixtures.repo.find_by_id(created.id).await.unwrap().unwrap();
    assert!(user.is_active);
}

#[actix_rt::test]
async fn verify_email_rejects_invalid_token() {
    init_test_tracing();
    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(test_fixtures().state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/verify-email?token=not-a-token")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn resend_verification_returns_202() {
    init_test_tracing();
    let fixtures = test_fixtures();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "resend@example.com".to_string(),
            username: "resend".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/resend-verification")
        .set_json(serde_json::json!({ "email": "resend@example.com" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
}

#[actix_rt::test]
async fn admin_can_list_roles() {
    init_test_tracing();
    let fixtures = test_fixtures();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "admin@example.com".to_string(),
            username: "admin".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&fixtures, "admin@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/roles")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let roles = body["roles"].as_array().unwrap();
    assert!(roles.iter().any(|r| r["name"] == "admin"));
}

#[actix_rt::test]
async fn admin_can_update_role_scopes() {
    init_test_tracing();
    let fixtures = test_fixtures();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "admin@example.com".to_string(),
            username: "admin".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&fixtures, "admin@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::put()
        .uri("/api/v1/roles/moderator")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(serde_json::json!({ "scopes": ["users:read", "users:write"] }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["name"], "moderator");
    assert!(body["scopes"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("users:read")));
}

#[actix_rt::test]
async fn admin_can_assign_roles_to_user() {
    init_test_tracing();
    let fixtures = test_fixtures();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "admin@example.com".to_string(),
            username: "admin".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let target = fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "target@example.com".to_string(),
            username: "target".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&fixtures, "admin@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/users/{}/roles", target.id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(serde_json::json!({ "roles": ["admin"] }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["roles"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("admin")));
}

#[actix_rt::test]
async fn non_admin_cannot_assign_roles() {
    init_test_tracing();
    let fixtures = test_fixtures();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "admin@example.com".to_string(),
            username: "admin".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let target = fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "target@example.com".to_string(),
            username: "target".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&fixtures, "target@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/users/{}/roles", target.id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(serde_json::json!({ "roles": ["admin"] }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn forgot_password_returns_202_for_existing_user() {
    init_test_tracing();
    let fixtures = test_fixtures();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "forgot@example.com".to_string(),
            username: "forgot".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/forgot-password")
        .set_json(serde_json::json!({ "email": "forgot@example.com" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
}

#[actix_rt::test]
async fn forgot_password_returns_202_for_unknown_email() {
    init_test_tracing();
    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(test_fixtures().state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/forgot-password")
        .set_json(serde_json::json!({ "email": "unknown@example.com" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
}

#[actix_rt::test]
async fn reset_password_changes_password_and_allows_login() {
    init_test_tracing();
    let fixtures = test_fixtures();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "reset@example.com".to_string(),
            username: "reset".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state.clone()))
            .configure(routes::configure),
    )
    .await;

    let forgot = test::TestRequest::post()
        .uri("/api/v1/auth/forgot-password")
        .set_json(serde_json::json!({ "email": "reset@example.com" }))
        .to_request();
    let resp = test::call_service(&app, forgot).await;
    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    let token = extract_reset_token_for_email(&fixtures.queue, "reset@example.com");

    let reset = test::TestRequest::post()
        .uri("/api/v1/auth/reset-password")
        .set_json(serde_json::json!({ "token": token, "password": "newpassword123" }))
        .to_request();
    let resp = test::call_service(&app, reset).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let login = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "reset@example.com",
            "password": "newpassword123"
        }))
        .to_request();
    let resp = test::call_service(&app, login).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_rt::test]
async fn reset_password_returns_400_for_invalid_token() {
    init_test_tracing();
    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(test_fixtures().state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/reset-password")
        .set_json(serde_json::json!({ "token": "not-a-token", "password": "newpassword123" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn reset_password_returns_400_for_short_password() {
    init_test_tracing();
    let fixtures = test_fixtures();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "short@example.com".to_string(),
            username: "short".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let forgot = test::TestRequest::post()
        .uri("/api/v1/auth/forgot-password")
        .set_json(serde_json::json!({ "email": "short@example.com" }))
        .to_request();
    let resp = test::call_service(&app, forgot).await;
    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    let token = extract_reset_token_for_email(&fixtures.queue, "short@example.com");

    let reset = test::TestRequest::post()
        .uri("/api/v1/auth/reset-password")
        .set_json(serde_json::json!({ "token": token, "password": "short" }))
        .to_request();
    let resp = test::call_service(&app, reset).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ============================================================================
// Party handler tests
// ============================================================================

fn seed_user(fixtures: &TestFixtures, email: &str, role: &str) -> Uuid {
    let id = Uuid::now_v7();
    let username = email.split('@').next().unwrap();
    let mut user = User::new(
        id,
        Email::new(email).unwrap(),
        Username::new(username).unwrap(),
        PasswordHash::new("hash".to_string()).unwrap(),
    );
    user.is_active = true;
    user.roles = vec![role.to_string()];
    fixtures.repo.users.lock().unwrap().insert(id, user);
    id
}

fn bearer(id: Uuid) -> String {
    format!("Bearer token-{id}")
}

macro_rules! create_party {
    ($app:expr, $token:expr, $email:expr) => {{
        let req = test::TestRequest::post()
            .uri("/api/v1/parties")
            .insert_header((header::AUTHORIZATION, $token.to_string()))
            .set_json(serde_json::json!({
                "display_name": "Green Acres Farm",
                "email": $email,
                "party_type": "ORGANIZATION",
                "roles": []
            }))
            .to_request();
        let resp = test::call_service($app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body: serde_json::Value = test::read_body_json(resp).await;
        Uuid::parse_str(body["id"].as_str().unwrap()).unwrap()
    }};
}

#[actix_rt::test]
async fn create_party_returns_201() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let id = create_party!(&app, &bearer(owner_id), "farm@example.com");
    assert!(!id.to_string().is_empty());
}

#[actix_rt::test]
async fn get_party_returns_200_for_owner() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner2@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let party_id = create_party!(&app, &bearer(owner_id), "farm2@example.com");

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/parties/{party_id}"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["email"], "farm2@example.com");
}

#[actix_rt::test]
async fn list_my_parties_returns_owned_parties() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner3@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    create_party!(&app, &bearer(owner_id), "farm3a@example.com");
    create_party!(&app, &bearer(owner_id), "farm3b@example.com");

    let req = test::TestRequest::get()
        .uri("/api/v1/parties/me")
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["parties"].as_array().unwrap().len(), 2);
}

#[actix_rt::test]
async fn list_parties_requires_admin() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let user_id = seed_user(&fixtures, "regular@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/parties")
        .insert_header((header::AUTHORIZATION, bearer(user_id)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn search_parties_returns_results_for_admin() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let admin_id = seed_user(&fixtures, "admin@example.com", "admin");
    let owner_id = seed_user(&fixtures, "owner4@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    create_party!(&app, &bearer(owner_id), "searchable@example.com");

    let req = test::TestRequest::get()
        .uri("/api/v1/parties/search?q=searchable")
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["parties"].as_array().unwrap().len(), 1);
}

#[actix_rt::test]
async fn update_party_returns_200_for_owner() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner5@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let party_id = create_party!(&app, &bearer(owner_id), "farm5@example.com");

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/parties/{party_id}"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .set_json(serde_json::json!({ "display_name": "Updated Farm" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["display_name"], "Updated Farm");
}

#[actix_rt::test]
async fn add_and_remove_party_role() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner6@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let party_id = create_party!(&app, &bearer(owner_id), "farm6@example.com");

    let add = test::TestRequest::post()
        .uri(&format!("/api/v1/parties/{party_id}/roles"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .set_json(serde_json::json!({
            "role_type": "SUPPLIER",
            "profile": {
                "type": "SUPPLIER",
                "resource_type_ids": [],
                "preferred_compensation": [],
                "insurance_verified": false
            }
        }))
        .to_request();
    let resp = test::call_service(&app, add).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let list = test::TestRequest::get()
        .uri(&format!("/api/v1/parties/{party_id}/roles"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .to_request();
    let resp = test::call_service(&app, list).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["roles"].as_array().unwrap().len(), 1);

    let remove = test::TestRequest::delete()
        .uri(&format!("/api/v1/parties/{party_id}/roles/SUPPLIER"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .to_request();
    let resp = test::call_service(&app, remove).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[actix_rt::test]
async fn delete_party_returns_204_for_owner() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner7@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let party_id = create_party!(&app, &bearer(owner_id), "farm7@example.com");

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/parties/{party_id}"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

async fn seed_party_with_location(
    state: &AppState,
    owner_id: Uuid,
    email: &str,
    lat: f64,
    lng: f64,
) -> Uuid {
    let result = state
        .create_party
        .execute(application::parties::dto::CreatePartyCommand {
            actor_user_id: owner_id,
            party_type: PartyType::Organization,
            display_name: "Located Farm".to_string(),
            email: email.to_string(),
            phone: None,
            tax_id: None,
            primary_domain_id: None,
            latitude: Some(lat),
            longitude: Some(lng),
            service_radius_km: Some(10.0),
            roles: vec![],
        })
        .await
        .unwrap();
    result.id
}

#[actix_rt::test]
async fn search_parties_with_radius_returns_results() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let admin_id = seed_user(&fixtures, "admin_radius@example.com", "admin");
    let owner_id = seed_user(&fixtures, "owner_radius@example.com", "user");

    seed_party_with_location(
        &fixtures.state,
        owner_id,
        "within@example.com",
        37.7749,
        -122.4194,
    )
    .await;
    seed_party_with_location(
        &fixtures.state,
        owner_id,
        "outside@example.com",
        40.7128,
        -74.0060,
    )
    .await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/parties/search?lat=37.7749&lng=-122.4194&radiusKm=10")
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["parties"].as_array().unwrap().len(), 1);
    assert_eq!(body["total"], 1);
}

#[actix_rt::test]
async fn nearby_parties_returns_parties_within_radius() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner_nearby@example.com", "user");

    seed_party_with_location(
        &fixtures.state,
        owner_id,
        "nearby_a@example.com",
        37.7750,
        -122.4195,
    )
    .await;
    seed_party_with_location(
        &fixtures.state,
        owner_id,
        "nearby_b@example.com",
        37.7760,
        -122.4200,
    )
    .await;
    seed_party_with_location(
        &fixtures.state,
        owner_id,
        "far_away@example.com",
        48.8566,
        2.3522,
    )
    .await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/parties/nearby?lat=37.7749&lng=-122.4194&radiusKm=1")
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["parties"].as_array().unwrap().len(), 2);
    assert_eq!(body["total"], 2);
}

#[actix_rt::test]
async fn nearby_parties_requires_radius_and_coordinates() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner_badgeo@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/parties/nearby?lat=37.7749")
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

fn role_profile(role: &str) -> serde_json::Value {
    match role {
        "SUPPLIER" => serde_json::json!({
            "type": "SUPPLIER",
            "resource_type_ids": [],
            "preferred_compensation": [],
            "insurance_verified": false
        }),
        "CONSUMER" => serde_json::json!({
            "type": "CONSUMER",
            "need_category_ids": [],
            "preferred_payment_terms": []
        }),
        "ENHANCER" => serde_json::json!({
            "type": "ENHANCER",
            "enhancement_type_ids": [],
            "skills": [],
            "equipment_owned": []
        }),
        _ => panic!("unsupported test role: {role}"),
    }
}

macro_rules! create_party_with_role {
    ($app:expr, $owner_id:expr, $email:expr, $role:expr) => {{
        let party_id = create_party!($app, &bearer($owner_id), $email);

        let add = test::TestRequest::post()
            .uri(&format!("/api/v1/parties/{party_id}/roles"))
            .insert_header((header::AUTHORIZATION, bearer($owner_id)))
            .set_json(serde_json::json!({
                "role_type": $role,
                "profile": role_profile($role)
            }))
            .to_request();
        let resp = test::call_service($app, add).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        party_id
    }};
}

macro_rules! create_three_party_deal {
    ($app:expr, $owner_id:expr, $supplier:expr, $consumer:expr, $enhancer:expr) => {{
        let category_id = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
        let req = test::TestRequest::post()
            .uri("/api/v1/deals")
            .insert_header((header::AUTHORIZATION, bearer($owner_id)))
            .insert_header(("X-Party-ID", $supplier.to_string()))
            .set_json(serde_json::json!({
                "title": "API Negotiation Deal",
                "domain_category_id": category_id,
                "consumer_party_id": $consumer,
                "enhancer_party_id": $enhancer
            }))
            .to_request();
        let resp = test::call_service($app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body: serde_json::Value = test::read_body_json(resp).await;
        Uuid::parse_str(body["id"].as_str().unwrap()).unwrap()
    }};
}

#[actix_rt::test]
async fn propose_and_list_terms_via_api() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "termowner@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let supplier = create_party_with_role!(&app, owner_id, "supplier-term@example.com", "SUPPLIER");
    let consumer = create_party_with_role!(&app, owner_id, "consumer-term@example.com", "CONSUMER");
    let enhancer = create_party_with_role!(&app, owner_id, "enhancer-term@example.com", "ENHANCER");
    let deal_id = create_three_party_deal!(&app, owner_id, supplier, consumer, enhancer);

    let propose = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/terms"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(serde_json::json!({
            "term_type": "PRICE",
            "term_name": "Unit price",
            "description": "100 points",
            "is_mandatory": true
        }))
        .to_request();
    let resp = test::call_service(&app, propose).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["term_name"], "Unit price");

    let list = test::TestRequest::get()
        .uri(&format!("/api/v1/deals/{deal_id}/terms"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .to_request();
    let resp = test::call_service(&app, list).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[actix_rt::test]
async fn set_and_get_value_distribution_via_api() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "valueowner@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let supplier =
        create_party_with_role!(&app, owner_id, "supplier-value@example.com", "SUPPLIER");
    let consumer =
        create_party_with_role!(&app, owner_id, "consumer-value@example.com", "CONSUMER");
    let enhancer =
        create_party_with_role!(&app, owner_id, "enhancer-value@example.com", "ENHANCER");
    let deal_id = create_three_party_deal!(&app, owner_id, supplier, consumer, enhancer);

    let set = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/value-distribution"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(serde_json::json!({
            "total_value": "10000",
            "distribution_model": "FIXED_PRICE",
            "supplier_share_percentage": "60",
            "enhancer_share_percentage": "30",
            "platform_fee_percentage": "10",
            "consumer_cost_percentage": "100",
            "payment_schedule": []
        }))
        .to_request();
    let resp = test::call_service(&app, set).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["total_value"], "10000");

    let get = test::TestRequest::get()
        .uri(&format!("/api/v1/deals/{deal_id}/value-distribution"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .to_request();
    let resp = test::call_service(&app, get).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["supplier_share_amount"], "6000");
}

#[actix_rt::test]
async fn validate_deal_via_api_returns_good() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "validateowner@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let supplier =
        create_party_with_role!(&app, owner_id, "supplier-validate@example.com", "SUPPLIER");
    let consumer =
        create_party_with_role!(&app, owner_id, "consumer-validate@example.com", "CONSUMER");
    let enhancer =
        create_party_with_role!(&app, owner_id, "enhancer-validate@example.com", "ENHANCER");
    let deal_id = create_three_party_deal!(&app, owner_id, supplier, consumer, enhancer);

    let set = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/value-distribution"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(serde_json::json!({
            "total_value": "10000",
            "distribution_model": "FIXED_PRICE",
            "supplier_share_percentage": "60",
            "enhancer_share_percentage": "30",
            "platform_fee_percentage": "10",
            "consumer_cost_percentage": "100",
            "payment_schedule": []
        }))
        .to_request();
    let resp = test::call_service(&app, set).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let validate = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/validate"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .to_request();
    let resp = test::call_service(&app, validate).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "GOOD");
    assert_eq!(body["blocked"], false);
}

#[actix_rt::test]
async fn submit_deal_without_value_distribution_returns_conflict() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "submitowner@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let supplier =
        create_party_with_role!(&app, owner_id, "supplier-submit@example.com", "SUPPLIER");
    let consumer =
        create_party_with_role!(&app, owner_id, "consumer-submit@example.com", "CONSUMER");
    let enhancer =
        create_party_with_role!(&app, owner_id, "enhancer-submit@example.com", "ENHANCER");
    let deal_id = create_three_party_deal!(&app, owner_id, supplier, consumer, enhancer);

    let submit = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/submit"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .to_request();
    let resp = test::call_service(&app, submit).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

// ============================================================================
// Agreement handler tests
// ============================================================================

async fn prepare_deal_for_agreement(
    fixtures: &TestFixtures,
    deal_id: Uuid,
    actor_user_id: Uuid,
    actor_party_id: Uuid,
) {
    fixtures
        .state
        .set_value_distribution
        .execute(SetValueDistributionCommand {
            actor_user_id,
            actor_party_id,
            is_admin: false,
            deal_id,
            total_value: Decimal::from(10000),
            distribution_model: DistributionModel::FixedPrice,
            supplier_share_percentage: Decimal::from(60),
            enhancer_share_percentage: Decimal::from(30),
            platform_fee_percentage: Decimal::from(10),
            consumer_cost_percentage: Decimal::from(100),
            payment_schedule: vec![],
        })
        .await
        .unwrap();

    let mut deals = fixtures.deal_repo.deals.lock().unwrap();
    let deal = deals.get_mut(&deal_id).expect("deal exists");
    deal.deal_status = DealStatus::TermsLocked;
}

#[actix_rt::test]
async fn get_agreement_visible_to_participant() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "agreementowner@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state.clone()))
            .configure(routes::configure),
    )
    .await;

    let supplier =
        create_party_with_role!(&app, owner_id, "supplier-agree@example.com", "SUPPLIER");
    let consumer =
        create_party_with_role!(&app, owner_id, "consumer-agree@example.com", "CONSUMER");
    let enhancer =
        create_party_with_role!(&app, owner_id, "enhancer-agree@example.com", "ENHANCER");
    let deal_id = create_three_party_deal!(&app, owner_id, supplier, consumer, enhancer);
    prepare_deal_for_agreement(&fixtures, deal_id, owner_id, supplier).await;

    fixtures
        .state
        .generate_agreement
        .execute(application::agreements::dto::GenerateAgreementCommand {
            actor_user_id: owner_id,
            actor_party_id: supplier,
            deal_id,
        })
        .await
        .unwrap();

    let get = test::TestRequest::get()
        .uri(&format!("/api/v1/deals/{deal_id}/agreement"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .to_request();
    let resp = test::call_service(&app, get).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["deal_id"].as_str().unwrap(), deal_id.to_string());
    assert!(body["agreement_text"]
        .as_str()
        .unwrap()
        .contains("API Negotiation Deal"));
}

#[actix_rt::test]
async fn get_agreement_hidden_from_outsider() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "agreementowner2@example.com", "user");
    let outsider_id = seed_user(&fixtures, "outsider@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state.clone()))
            .configure(routes::configure),
    )
    .await;

    let supplier =
        create_party_with_role!(&app, owner_id, "supplier-agree2@example.com", "SUPPLIER");
    let consumer =
        create_party_with_role!(&app, owner_id, "consumer-agree2@example.com", "CONSUMER");
    let enhancer =
        create_party_with_role!(&app, owner_id, "enhancer-agree2@example.com", "ENHANCER");
    let deal_id = create_three_party_deal!(&app, owner_id, supplier, consumer, enhancer);
    prepare_deal_for_agreement(&fixtures, deal_id, owner_id, supplier).await;

    fixtures
        .state
        .generate_agreement
        .execute(application::agreements::dto::GenerateAgreementCommand {
            actor_user_id: owner_id,
            actor_party_id: supplier,
            deal_id,
        })
        .await
        .unwrap();

    let get = test::TestRequest::get()
        .uri(&format!("/api/v1/deals/{deal_id}/agreement"))
        .insert_header((header::AUTHORIZATION, bearer(outsider_id)))
        .to_request();
    let resp = test::call_service(&app, get).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_rt::test]
async fn sign_agreement_records_signature_via_api() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "signowner@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state.clone()))
            .configure(routes::configure),
    )
    .await;

    let supplier = create_party_with_role!(&app, owner_id, "supplier-sign@example.com", "SUPPLIER");
    let consumer = create_party_with_role!(&app, owner_id, "consumer-sign@example.com", "CONSUMER");
    let enhancer = create_party_with_role!(&app, owner_id, "enhancer-sign@example.com", "ENHANCER");
    let deal_id = create_three_party_deal!(&app, owner_id, supplier, consumer, enhancer);
    prepare_deal_for_agreement(&fixtures, deal_id, owner_id, supplier).await;

    fixtures
        .state
        .generate_agreement
        .execute(application::agreements::dto::GenerateAgreementCommand {
            actor_user_id: owner_id,
            actor_party_id: supplier,
            deal_id,
        })
        .await
        .unwrap();

    let sign = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/agreement/sign"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(serde_json::json!({}))
        .to_request();
    let resp = test::call_service(&app, sign).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["signatures"].as_array().unwrap().len(), 1);
    assert_eq!(
        body["signatures"][0]["party_id"].as_str().unwrap(),
        supplier.to_string()
    );
}

#[actix_rt::test]
async fn admin_can_get_and_update_agreement() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "admindealowner@example.com", "user");
    let admin_id = seed_user(&fixtures, "platformadmin@example.com", "admin");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state.clone()))
            .configure(routes::configure),
    )
    .await;

    let supplier =
        create_party_with_role!(&app, owner_id, "supplier-admin@example.com", "SUPPLIER");
    let consumer =
        create_party_with_role!(&app, owner_id, "consumer-admin@example.com", "CONSUMER");
    let enhancer =
        create_party_with_role!(&app, owner_id, "enhancer-admin@example.com", "ENHANCER");
    let deal_id = create_three_party_deal!(&app, owner_id, supplier, consumer, enhancer);
    prepare_deal_for_agreement(&fixtures, deal_id, owner_id, supplier).await;

    fixtures
        .state
        .generate_agreement
        .execute(application::agreements::dto::GenerateAgreementCommand {
            actor_user_id: owner_id,
            actor_party_id: supplier,
            deal_id,
        })
        .await
        .unwrap();

    let get = test::TestRequest::get()
        .uri(&format!("/api/v1/admin/deals/{deal_id}/agreement"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, get).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let patch = test::TestRequest::patch()
        .uri(&format!("/api/v1/admin/deals/{deal_id}/agreement"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .set_json(serde_json::json!({
            "governing_law": "California",
            "dispute_resolution": "Arbitration",
            "auto_renew": true
        }))
        .to_request();
    let resp = test::call_service(&app, patch).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["governing_law"].as_str().unwrap(), "California");
    assert_eq!(body["dispute_resolution"].as_str().unwrap(), "Arbitration");
    assert_eq!(body["auto_renew"].as_bool().unwrap(), true);
}

async fn seed_party(state: &AppState, owner_id: Uuid, email: &str, role: DealRole) -> Uuid {
    state
        .create_party
        .execute(application::parties::dto::CreatePartyCommand {
            actor_user_id: owner_id,
            party_type: PartyType::Organization,
            display_name: "Wallet Party".to_string(),
            email: email.to_string(),
            phone: None,
            tax_id: None,
            primary_domain_id: None,
            latitude: None,
            longitude: None,
            service_radius_km: None,
            roles: vec![role],
        })
        .await
        .unwrap()
        .id
}

async fn seed_deal(
    state: &AppState,
    actor_user_id: Uuid,
    actor_party_id: Uuid,
    consumer_party_id: Uuid,
    enhancer_party_id: Uuid,
) -> Uuid {
    let category_id = Uuid::parse_str("a1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap();
    state
        .create_deal
        .execute(application::deals::dto::CreateDealCommand {
            actor_user_id,
            actor_party_id,
            is_admin: false,
            title: "Wallet Deal".to_string(),
            description: None,
            domain_category_id: category_id,
            consumer_party_id,
            enhancer_party_id,
            expected_start_date: None,
            expected_end_date: None,
            timeline: None,
            latitude: None,
            longitude: None,
            timeout_overrides: None,
        })
        .await
        .unwrap()
        .id
}

#[actix_rt::test]
async fn wallet_endpoints_manage_points_for_party() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "wallet_owner@example.com", "user");
    let party_id = seed_party(
        &fixtures.state,
        owner_id,
        "wallet_party@example.com",
        DealRole::Supplier,
    )
    .await;
    let consumer_id = seed_party(
        &fixtures.state,
        owner_id,
        "consumer_wallet@example.com",
        DealRole::Consumer,
    )
    .await;
    let enhancer_id = seed_party(
        &fixtures.state,
        owner_id,
        "enhancer_wallet@example.com",
        DealRole::Enhancer,
    )
    .await;
    let deal_id = seed_deal(
        &fixtures.state,
        owner_id,
        party_id,
        consumer_id,
        enhancer_id,
    )
    .await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let get_wallet = test::TestRequest::get()
        .uri(&format!("/api/v1/parties/{party_id}/wallet"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .to_request();
    let resp = test::call_service(&app, get_wallet).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["partyId"], party_id.to_string());
    assert_eq!(body["balance"], "0");

    let deposit = test::TestRequest::post()
        .uri(&format!("/api/v1/parties/{party_id}/wallet/deposits"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .set_json(serde_json::json!({
            "dealId": deal_id,
            "amount": "150.00"
        }))
        .to_request();
    let resp = test::call_service(&app, deposit).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["amount"], "150.00");

    let get_wallet = test::TestRequest::get()
        .uri(&format!("/api/v1/parties/{party_id}/wallet"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .to_request();
    let resp = test::call_service(&app, get_wallet).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["balance"], "150.00");

    let get_deal_wallet = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/parties/{party_id}/deals/{deal_id}/wallet"
        ))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .to_request();
    let resp = test::call_service(&app, get_deal_wallet).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["deposited"], "150.00");

    let list_transactions = test::TestRequest::get()
        .uri(&format!("/api/v1/parties/{party_id}/wallet/transactions"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .to_request();
    let resp = test::call_service(&app, list_transactions).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["total"], 1);
    assert_eq!(body["transactions"].as_array().unwrap().len(), 1);
}

#[actix_rt::test]
async fn admin_can_get_party_and_roles() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner_admin_party@example.com", "user");
    let admin_id = seed_user(&fixtures, "admin_party@example.com", "admin");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let party_id = create_party!(&app, &bearer(owner_id), "farm-admin@example.com");

    let get = test::TestRequest::get()
        .uri(&format!("/api/v1/parties/{party_id}"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, get).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["email"], "farm-admin@example.com");

    let roles = test::TestRequest::get()
        .uri(&format!("/api/v1/parties/{party_id}/roles"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, roles).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_rt::test]
async fn admin_can_update_party() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner_admin_update_party@example.com", "user");
    let admin_id = seed_user(&fixtures, "admin_update_party@example.com", "admin");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let party_id = create_party!(&app, &bearer(owner_id), "farm-admin-update@example.com");

    let update = test::TestRequest::patch()
        .uri(&format!("/api/v1/admin/parties/{party_id}"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .set_json(serde_json::json!({
            "verification_status": "VERIFIED",
            "is_active": false
        }))
        .to_request();
    let resp = test::call_service(&app, update).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["verification_status"], "VERIFIED");
    assert_eq!(body["is_active"], false);
}

#[actix_rt::test]
async fn admin_can_delete_party_with_active_deals() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner_admin_delete_party@example.com", "user");
    let admin_id = seed_user(&fixtures, "admin_delete_party@example.com", "admin");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let supplier = create_party_with_role!(
        &app,
        owner_id,
        "supplier-admin-delete@example.com",
        "SUPPLIER"
    );
    let consumer = create_party_with_role!(
        &app,
        owner_id,
        "consumer-admin-delete@example.com",
        "CONSUMER"
    );
    let enhancer = create_party_with_role!(
        &app,
        owner_id,
        "enhancer-admin-delete@example.com",
        "ENHANCER"
    );
    create_three_party_deal!(&app, owner_id, supplier, consumer, enhancer);

    let delete = test::TestRequest::delete()
        .uri(&format!("/api/v1/parties/{supplier}"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, delete).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[actix_rt::test]
async fn admin_can_validate_deal() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner_admin_validate@example.com", "user");
    let admin_id = seed_user(&fixtures, "admin_validate@example.com", "admin");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let supplier = create_party_with_role!(
        &app,
        owner_id,
        "supplier-admin-validate@example.com",
        "SUPPLIER"
    );
    let consumer = create_party_with_role!(
        &app,
        owner_id,
        "consumer-admin-validate@example.com",
        "CONSUMER"
    );
    let enhancer = create_party_with_role!(
        &app,
        owner_id,
        "enhancer-admin-validate@example.com",
        "ENHANCER"
    );
    let deal_id = create_three_party_deal!(&app, owner_id, supplier, consumer, enhancer);

    let set = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/value-distribution"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(serde_json::json!({
            "total_value": "10000",
            "distribution_model": "FIXED_PRICE",
            "supplier_share_percentage": "60",
            "enhancer_share_percentage": "30",
            "platform_fee_percentage": "10",
            "consumer_cost_percentage": "100",
            "payment_schedule": []
        }))
        .to_request();
    let resp = test::call_service(&app, set).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let validate = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/validate"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, validate).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "GOOD");
}

#[actix_rt::test]
async fn admin_can_list_terms_and_value_distribution() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner_admin_terms@example.com", "user");
    let admin_id = seed_user(&fixtures, "admin_terms@example.com", "admin");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let supplier = create_party_with_role!(
        &app,
        owner_id,
        "supplier-admin-terms@example.com",
        "SUPPLIER"
    );
    let consumer = create_party_with_role!(
        &app,
        owner_id,
        "consumer-admin-terms@example.com",
        "CONSUMER"
    );
    let enhancer = create_party_with_role!(
        &app,
        owner_id,
        "enhancer-admin-terms@example.com",
        "ENHANCER"
    );
    let deal_id = create_three_party_deal!(&app, owner_id, supplier, consumer, enhancer);

    let propose = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/terms"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(serde_json::json!({
            "term_type": "PRICE",
            "term_name": "Admin visible term",
            "description": "100 points",
            "is_mandatory": true
        }))
        .to_request();
    let resp = test::call_service(&app, propose).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let set = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/value-distribution"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(serde_json::json!({
            "total_value": "10000",
            "distribution_model": "FIXED_PRICE",
            "supplier_share_percentage": "60",
            "enhancer_share_percentage": "30",
            "platform_fee_percentage": "10",
            "consumer_cost_percentage": "100",
            "payment_schedule": []
        }))
        .to_request();
    let resp = test::call_service(&app, set).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let list_terms = test::TestRequest::get()
        .uri(&format!("/api/v1/deals/{deal_id}/terms"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, list_terms).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 1);

    let get_value = test::TestRequest::get()
        .uri(&format!("/api/v1/deals/{deal_id}/value-distribution"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, get_value).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["total_value"], "10000");
}

#[actix_rt::test]
async fn admin_can_manage_deal_without_membership() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner_admin_deal@example.com", "user");
    let admin_id = seed_user(&fixtures, "admin_deal@example.com", "admin");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let supplier = create_party_with_role!(
        &app,
        owner_id,
        "supplier-admin-deal@example.com",
        "SUPPLIER"
    );
    let consumer = create_party_with_role!(
        &app,
        owner_id,
        "consumer-admin-deal@example.com",
        "CONSUMER"
    );
    let enhancer = create_party_with_role!(
        &app,
        owner_id,
        "enhancer-admin-deal@example.com",
        "ENHANCER"
    );

    let category_id = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
    let create = test::TestRequest::post()
        .uri("/api/v1/deals")
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(serde_json::json!({
            "title": "Admin Managed Deal",
            "domain_category_id": category_id,
            "consumer_party_id": consumer,
            "enhancer_party_id": enhancer
        }))
        .to_request();
    let resp = test::call_service(&app, create).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let deal_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();

    let update = test::TestRequest::patch()
        .uri(&format!("/api/v1/deals/{deal_id}"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(serde_json::json!({
            "title": "Updated by admin"
        }))
        .to_request();
    let resp = test::call_service(&app, update).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["title"], "Updated by admin");

    let set = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/value-distribution"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(serde_json::json!({
            "total_value": "10000",
            "distribution_model": "FIXED_PRICE",
            "supplier_share_percentage": "60",
            "enhancer_share_percentage": "30",
            "platform_fee_percentage": "10",
            "consumer_cost_percentage": "100",
            "payment_schedule": []
        }))
        .to_request();
    let resp = test::call_service(&app, set).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let propose = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/terms"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(serde_json::json!({
            "term_type": "PRICE",
            "term_name": "Admin term",
            "description": "100 points",
            "is_mandatory": true
        }))
        .to_request();
    let resp = test::call_service(&app, propose).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let term_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();

    let accept = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/terms/{term_id}/accept"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .to_request();
    let resp = test::call_service(&app, accept).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let submit = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/submit"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .to_request();
    let resp = test::call_service(&app, submit).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["deal_status"], "SUGGESTED");
}

#[actix_rt::test]
async fn admin_can_read_wallet_and_transactions() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner_admin_wallet@example.com", "user");
    let admin_id = seed_user(&fixtures, "admin_wallet@example.com", "admin");

    let party_id = seed_party(
        &fixtures.state,
        owner_id,
        "wallet_admin_party@example.com",
        DealRole::Supplier,
    )
    .await;
    let consumer_id = seed_party(
        &fixtures.state,
        owner_id,
        "consumer_admin_wallet@example.com",
        DealRole::Consumer,
    )
    .await;
    let enhancer_id = seed_party(
        &fixtures.state,
        owner_id,
        "enhancer_admin_wallet@example.com",
        DealRole::Enhancer,
    )
    .await;
    let deal_id = seed_deal(
        &fixtures.state,
        owner_id,
        party_id,
        consumer_id,
        enhancer_id,
    )
    .await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let deposit = test::TestRequest::post()
        .uri(&format!("/api/v1/parties/{party_id}/wallet/deposits"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .set_json(serde_json::json!({
            "dealId": deal_id,
            "amount": "200.00"
        }))
        .to_request();
    let resp = test::call_service(&app, deposit).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let get_wallet = test::TestRequest::get()
        .uri(&format!("/api/v1/parties/{party_id}/wallet"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, get_wallet).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["balance"], "200.00");

    let get_deal_wallet = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/parties/{party_id}/deals/{deal_id}/wallet"
        ))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, get_deal_wallet).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["deposited"], "200.00");

    let list_transactions = test::TestRequest::get()
        .uri(&format!("/api/v1/parties/{party_id}/wallet/transactions"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, list_transactions).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["total"], 1);

    let list_deal_transactions = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/parties/{party_id}/deals/{deal_id}/transactions"
        ))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, list_deal_transactions).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["total"], 1);
}

#[actix_rt::test]
async fn admin_can_deposit_and_withdraw_for_party() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner_admin_payment@example.com", "user");
    let admin_id = seed_user(&fixtures, "admin_payment@example.com", "admin");

    let party_id = seed_party(
        &fixtures.state,
        owner_id,
        "payment_admin_party@example.com",
        DealRole::Supplier,
    )
    .await;
    let consumer_id = seed_party(
        &fixtures.state,
        owner_id,
        "consumer_admin_payment@example.com",
        DealRole::Consumer,
    )
    .await;
    let enhancer_id = seed_party(
        &fixtures.state,
        owner_id,
        "enhancer_admin_payment@example.com",
        DealRole::Enhancer,
    )
    .await;
    let deal_id = seed_deal(
        &fixtures.state,
        owner_id,
        party_id,
        consumer_id,
        enhancer_id,
    )
    .await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let deposit = test::TestRequest::post()
        .uri(&format!("/api/v1/parties/{party_id}/wallet/deposits"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .set_json(serde_json::json!({
            "dealId": deal_id,
            "amount": "300.00"
        }))
        .to_request();
    let resp = test::call_service(&app, deposit).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["amount"], "300.00");

    let withdraw = test::TestRequest::post()
        .uri(&format!("/api/v1/parties/{party_id}/wallet/withdrawals"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .set_json(serde_json::json!({
            "dealId": deal_id,
            "amount": "100.00"
        }))
        .to_request();
    let resp = test::call_service(&app, withdraw).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["amount"], "100.00");

    let get_wallet = test::TestRequest::get()
        .uri(&format!("/api/v1/parties/{party_id}/wallet"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, get_wallet).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["balance"], "200.00");
}
