#[cfg(test)]
use crate::email::queue::{EmailQueue, EmailQueueItem};
#[cfg(test)]
use crate::email::EmailSender;
#[cfg(test)]
use crate::errors::ApplicationError;
#[cfg(test)]
use crate::users::create_user::PasswordHasher;
#[cfg(test)]
use crate::users::token::{AuthContext, TokenGenerator, TokenVerifier};
#[cfg(test)]
use async_trait::async_trait;
#[cfg(test)]
use domain::entities::{
    Agreement, ApprovalDecision, Currency, DealRole, DealWallet, Email, EmailVerification,
    Milestone, PasswordHash, PasswordResetToken, PlatformWallet, Review, ReviewRating, Role,
    RoleProfile, Signature, Transaction, TransactionApproval, TransactionStatus, TransactionType,
    User, Username,
};
#[cfg(test)]
use domain::entities::{Party, UserPartyMembership};
use domain::entities::{PartyVerification, PartyVerificationStatus, PartyVerificationType};
#[cfg(test)]
use domain::errors::DomainError;
#[cfg(test)]
use domain::repositories::PartySearchCriteria;
#[cfg(test)]
use domain::repositories::{
    AgreementRepository, EmailVerificationRepository, MilestoneRepository, PartyRepository,
    PartyVerificationRepository, PasswordResetRepository, ReviewListResult, ReviewRepository,
    ReviewSearchCriteria, RoleRepository, TransactionFilters, UserRepository, WalletRepository,
};
#[cfg(test)]
use domain::repositories::{DealAggregate, DealListResult, DealRepository, DealSearchCriteria};
use domain::repositories::{VerificationListFilters, VerificationListResult};
#[cfg(test)]
use rust_decimal::Decimal;
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::sync::{Arc, Mutex};
#[cfg(test)]
use uuid::Uuid;

#[cfg(test)]
pub struct FakeRepo {
    pub users: Mutex<HashMap<Uuid, User>>,
}

#[cfg(test)]
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

#[cfg(test)]
pub struct FakeHasher;

#[cfg(test)]
#[async_trait]
impl PasswordHasher for FakeHasher {
    async fn hash_password(&self, password: &str) -> Result<String, ApplicationError> {
        Ok(format!("hashed:{password}"))
    }

    async fn verify_password(&self, password: &str, hash: &str) -> Result<bool, ApplicationError> {
        Ok(hash == format!("hashed:{password}"))
    }
}

#[cfg(test)]
pub struct FakeTokenGenerator;

#[cfg(test)]
#[async_trait]
impl TokenGenerator for FakeTokenGenerator {
    async fn generate(&self, ctx: &AuthContext) -> Result<String, ApplicationError> {
        Ok(format!("token-{}", ctx.user_id))
    }
}

#[cfg(test)]
pub struct FakeTokenVerifier;

#[cfg(test)]
#[async_trait]
impl TokenVerifier for FakeTokenVerifier {
    async fn verify(&self, token: &str) -> Result<AuthContext, ApplicationError> {
        let user_id = token
            .strip_prefix("token-")
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or(ApplicationError::Unauthorized)?;
        Ok(AuthContext {
            user_id,
            roles: vec!["user".to_string()],
            scopes: vec!["users:read".to_string(), "users:write".to_string()],
        })
    }
}

#[cfg(test)]
pub struct FakeRoleRepo;

#[cfg(test)]
#[async_trait]
impl RoleRepository for FakeRoleRepo {
    async fn find_by_name(&self, name: &str) -> Result<Option<Role>, DomainError> {
        match name {
            "user" => Ok(Some(Role::builtin(
                "user",
                vec!["users:read".to_string(), "users:write".to_string()],
            ))),
            "admin" => Ok(Some(Role::builtin(
                "admin",
                vec![
                    "users:read".to_string(),
                    "users:write".to_string(),
                    "users:admin".to_string(),
                    "users:delete".to_string(),
                    "admin:transactions".to_string(),
                    "admin:milestones".to_string(),
                    "admin:*".to_string(),
                ],
            ))),
            _ => Ok(None),
        }
    }

    async fn list(&self) -> Result<Vec<Role>, DomainError> {
        Ok(vec![
            self.find_by_name("user").await.unwrap().unwrap(),
            self.find_by_name("admin").await.unwrap().unwrap(),
        ])
    }

    async fn save(&self, _role: &Role) -> Result<(), DomainError> {
        Ok(())
    }

    async fn delete(&self, _name: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

#[cfg(test)]
pub fn test_user(email: &str, username: &str, password: &str) -> User {
    User::new(
        Uuid::now_v7(),
        Email::new(email).unwrap(),
        Username::new(username).unwrap(),
        PasswordHash::new(format!("hashed:{password}")).unwrap(),
    )
}

#[cfg(test)]
pub fn test_repo_with(user: User) -> Arc<FakeRepo> {
    let mut map = HashMap::new();
    map.insert(user.id, user);
    Arc::new(FakeRepo {
        users: Mutex::new(map),
    })
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeEmailVerificationRepo {
    verifications: Mutex<HashMap<String, EmailVerification>>,
}

#[cfg(test)]
impl FakeEmailVerificationRepo {
    pub async fn find_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<EmailVerification>, DomainError> {
        Ok(self
            .verifications
            .lock()
            .unwrap()
            .values()
            .filter(|v| v.user_id == user_id)
            .cloned()
            .collect())
    }

    pub async fn count_for_user(&self, user_id: Uuid) -> usize {
        self.find_by_user_id(user_id).await.unwrap().len()
    }
}

#[cfg(test)]
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

#[cfg(test)]
#[derive(Default)]
pub struct FakePasswordResetRepo {
    tokens: Mutex<HashMap<String, PasswordResetToken>>,
}

#[cfg(test)]
impl FakePasswordResetRepo {
    pub async fn count_for_user(&self, user_id: Uuid) -> usize {
        self.tokens
            .lock()
            .unwrap()
            .values()
            .filter(|t| t.user_id == user_id)
            .count()
    }
}

#[cfg(test)]
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

#[cfg(test)]
#[derive(Default)]
pub struct FakeEmailQueue {
    pub items: Mutex<Vec<(String, String, String)>>,
    failing: bool,
}

#[cfg(test)]
impl FakeEmailQueue {
    pub fn failing() -> Self {
        Self {
            items: Default::default(),
            failing: true,
        }
    }
}

#[cfg(test)]
#[async_trait]
impl EmailQueue for FakeEmailQueue {
    async fn enqueue(&self, item: EmailQueueItem) -> Result<(), ApplicationError> {
        if self.failing {
            return Err(ApplicationError::EmailSendFailed);
        }
        self.items
            .lock()
            .unwrap()
            .push((item.to, item.subject, item.body));
        Ok(())
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeEmailSender {
    pub sent: Mutex<Vec<(String, String, String)>>,
    failing: bool,
}

#[cfg(test)]
impl FakeEmailSender {
    pub fn failing() -> Self {
        Self {
            sent: Default::default(),
            failing: true,
        }
    }
}

#[cfg(test)]
#[async_trait]
impl EmailSender for FakeEmailSender {
    async fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), ApplicationError> {
        if self.failing {
            return Err(ApplicationError::EmailSendFailed);
        }
        self.sent
            .lock()
            .unwrap()
            .push((to.to_string(), subject.to_string(), body.to_string()));
        Ok(())
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct FakePartyRepo {
    pub parties: Mutex<HashMap<Uuid, Party>>,
    pub memberships: Mutex<Vec<UserPartyMembership>>,
    pub roles: Mutex<Vec<(Uuid, DealRole, RoleProfile)>>,
}

#[cfg(test)]
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

    async fn list(&self, _criteria: &PartySearchCriteria) -> Result<Vec<Party>, DomainError> {
        Ok(self.parties.lock().unwrap().values().cloned().collect())
    }

    async fn count(&self, _criteria: &PartySearchCriteria) -> Result<i64, DomainError> {
        Ok(self.parties.lock().unwrap().len() as i64)
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

#[cfg(test)]
#[derive(Default)]
pub struct FakeDealRepo {
    pub deals: std::sync::Mutex<HashMap<Uuid, domain::entities::Deal>>,
    pub participations: std::sync::Mutex<Vec<domain::entities::DealParticipation>>,
    pub terms: std::sync::Mutex<Vec<domain::entities::Term>>,
    pub value_distributions: std::sync::Mutex<HashMap<Uuid, domain::entities::ValueDistribution>>,
    pub history: std::sync::Mutex<Vec<(Uuid, String, Option<Uuid>, Option<serde_json::Value>)>>,
    reference_counter: std::sync::Mutex<i64>,
}

#[cfg(test)]
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
        total_value: Decimal,
        platform_fee_percentage: Decimal,
        platform_fee_amount: Decimal,
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

#[cfg(test)]
#[derive(Default)]
pub struct FakeAgreementRepo {
    pub agreements: Mutex<HashMap<Uuid, Agreement>>,
    pub signatures: Mutex<Vec<Signature>>,
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeWalletRepo {
    pub wallets: Mutex<HashMap<Uuid, PlatformWallet>>,
    pub transactions: Mutex<Vec<Transaction>>,
    pub approvals: Mutex<Vec<TransactionApproval>>,
}

#[cfg(test)]
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
        transaction: &Transaction,
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
    ) -> Result<Vec<Transaction>, DomainError> {
        let txns = self.transactions.lock().unwrap();
        let mut results: Vec<Transaction> = txns
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
        transaction: &Transaction,
    ) -> Result<(), DomainError> {
        self.transactions.lock().unwrap().push(transaction.clone());
        Ok(())
    }

    async fn find_transaction_by_id(&self, id: Uuid) -> Result<Option<Transaction>, DomainError> {
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
        transaction: &Transaction,
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
    ) -> Result<Vec<Transaction>, DomainError> {
        let approvals = self.approvals.lock().unwrap();
        let voted: Vec<Uuid> = approvals
            .iter()
            .filter(|a| a.party_id == party_id)
            .map(|a| a.transaction_id)
            .collect();
        drop(approvals);

        let txns = self.transactions.lock().unwrap();
        let mut results: Vec<Transaction> = txns
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

#[cfg(test)]
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
        let agreements = self.agreements.lock().unwrap();
        let version = agreements
            .get(&agreement_id)
            .map(|a| a.version)
            .unwrap_or(0);
        drop(agreements);
        Ok(self.signatures.lock().unwrap().iter().any(|s| {
            s.agreement_id == agreement_id && s.party_id == party_id && s.version == version
        }))
    }

    async fn count_signatures(&self, agreement_id: Uuid) -> Result<i64, DomainError> {
        let agreements = self.agreements.lock().unwrap();
        let version = agreements
            .get(&agreement_id)
            .map(|a| a.version)
            .unwrap_or(0);
        drop(agreements);
        Ok(self
            .signatures
            .lock()
            .unwrap()
            .iter()
            .filter(|s| s.agreement_id == agreement_id && s.version == version)
            .count() as i64)
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeMilestoneRepo {
    pub milestones: Mutex<Vec<Milestone>>,
}

#[cfg(test)]
#[async_trait]
impl MilestoneRepository for FakeMilestoneRepo {
    async fn create(&self, milestone: &Milestone) -> Result<(), DomainError> {
        self.milestones.lock().unwrap().push(milestone.clone());
        Ok(())
    }

    async fn update(&self, milestone: &Milestone) -> Result<(), DomainError> {
        let mut milestones = self.milestones.lock().unwrap();
        if let Some(m) = milestones.iter_mut().find(|m| m.id == milestone.id) {
            *m = milestone.clone();
        }
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        self.milestones.lock().unwrap().retain(|m| m.id != id);
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
        let milestones: Vec<_> = self
            .milestones
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.deal_id == deal_id)
            .skip(offset as usize)
            .take(limit as usize)
            .cloned()
            .collect();
        Ok(milestones)
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
        Ok(self
            .milestones
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.deal_id == deal_id && m.milestone_status.as_str() == "VERIFIED")
            .count() as i64)
    }

    async fn count_by_status(&self, deal_id: Uuid, status: &str) -> Result<i64, DomainError> {
        Ok(self
            .milestones
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.deal_id == deal_id && m.milestone_status.as_str() == status)
            .count() as i64)
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeReviewRepo {
    pub reviews: Mutex<Vec<Review>>,
}

#[cfg(test)]
#[async_trait]
impl ReviewRepository for FakeReviewRepo {
    async fn create(&self, review: &Review) -> Result<(), DomainError> {
        self.reviews.lock().unwrap().push(review.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Review>, DomainError> {
        Ok(self
            .reviews
            .lock()
            .unwrap()
            .iter()
            .find(|r| r.id == id)
            .cloned())
    }

    async fn exists(
        &self,
        deal_id: Uuid,
        reviewer_party_id: Uuid,
        reviewed_party_id: Uuid,
    ) -> Result<bool, DomainError> {
        Ok(self.reviews.lock().unwrap().iter().any(|r| {
            r.deal_id == deal_id
                && r.reviewer_party_id == reviewer_party_id
                && r.reviewed_party_id == reviewed_party_id
        }))
    }

    async fn count_by_deal(&self, deal_id: Uuid) -> Result<i64, DomainError> {
        Ok(self
            .reviews
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.deal_id == deal_id)
            .count() as i64)
    }

    async fn find_missing_review_pairs(
        &self,
        deal_id: Uuid,
        participations: &[(Uuid, DealRole)],
    ) -> Result<Vec<(Uuid, Uuid)>, DomainError> {
        let existing: std::collections::HashSet<(Uuid, Uuid)> = self
            .reviews
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.deal_id == deal_id)
            .map(|r| (r.reviewer_party_id, r.reviewed_party_id))
            .collect();

        let mut missing = Vec::new();
        for (reviewer, _) in participations {
            for (reviewed, _) in participations {
                if reviewer != reviewed && !existing.contains(&(*reviewer, *reviewed)) {
                    missing.push((*reviewer, *reviewed));
                }
            }
        }
        Ok(missing)
    }

    async fn list(&self, criteria: &ReviewSearchCriteria) -> Result<ReviewListResult, DomainError> {
        let reviews: Vec<Review> = self
            .reviews
            .lock()
            .unwrap()
            .iter()
            .filter(|r| criteria.deal_id.is_none_or(|d| r.deal_id == d))
            .filter(|r| {
                criteria
                    .reviewer_party_id
                    .is_none_or(|p| r.reviewer_party_id == p)
            })
            .filter(|r| {
                criteria
                    .reviewed_party_id
                    .is_none_or(|p| r.reviewed_party_id == p)
            })
            .filter(|r| criteria.is_public.is_none_or(|pub_| r.is_public == pub_))
            .cloned()
            .collect();

        let total = reviews.len() as i64;
        let start = criteria.offset as usize;
        let end = (criteria.offset + criteria.limit) as usize;
        let paginated = reviews
            .into_iter()
            .skip(start)
            .take(end.saturating_sub(start))
            .collect();

        Ok(ReviewListResult {
            reviews: paginated,
            total,
            limit: criteria.limit,
            offset: criteria.offset,
        })
    }

    async fn count(&self, criteria: &ReviewSearchCriteria) -> Result<i64, DomainError> {
        let count = self
            .reviews
            .lock()
            .unwrap()
            .iter()
            .filter(|r| criteria.deal_id.is_none_or(|d| r.deal_id == d))
            .filter(|r| {
                criteria
                    .reviewer_party_id
                    .is_none_or(|p| r.reviewer_party_id == p)
            })
            .filter(|r| {
                criteria
                    .reviewed_party_id
                    .is_none_or(|p| r.reviewed_party_id == p)
            })
            .filter(|r| criteria.is_public.is_none_or(|pub_| r.is_public == pub_))
            .count() as i64;
        Ok(count)
    }

    async fn update(&self, review: &Review) -> Result<(), DomainError> {
        let mut reviews = self.reviews.lock().unwrap();
        if let Some(r) = reviews.iter_mut().find(|r| r.id == review.id) {
            *r = review.clone();
        }
        Ok(())
    }

    async fn hide(&self, id: Uuid, platform_response: Option<String>) -> Result<(), DomainError> {
        let mut reviews = self.reviews.lock().unwrap();
        if let Some(r) = reviews.iter_mut().find(|r| r.id == id) {
            r.is_public = false;
            r.review_text = None;
            r.platform_response = platform_response;
        }
        Ok(())
    }
}

#[cfg(test)]
pub fn test_review(
    deal_id: Uuid,
    reviewer_party_id: Uuid,
    reviewed_party_id: Uuid,
    reviewed_role: DealRole,
    overall: i32,
) -> Review {
    Review::new(
        Uuid::now_v7(),
        deal_id,
        reviewer_party_id,
        reviewed_party_id,
        reviewed_role,
        ReviewRating::new(overall).unwrap(),
        None,
        None,
        None,
        None,
        None,
        true,
    )
}

#[cfg(test)]
#[derive(Default)]
pub struct FakePartyVerificationRepo {
    pub verifications: Mutex<Vec<PartyVerification>>,
}

#[cfg(test)]
#[async_trait]
impl PartyVerificationRepository for FakePartyVerificationRepo {
    async fn create(&self, verification: &PartyVerification) -> Result<(), DomainError> {
        self.verifications
            .lock()
            .unwrap()
            .push(verification.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<PartyVerification>, DomainError> {
        Ok(self
            .verifications
            .lock()
            .unwrap()
            .iter()
            .find(|v| v.id == id)
            .cloned())
    }

    async fn find_active_by_party_and_type(
        &self,
        party_id: Uuid,
        verification_type: PartyVerificationType,
    ) -> Result<Option<PartyVerification>, DomainError> {
        Ok(self
            .verifications
            .lock()
            .unwrap()
            .iter()
            .find(|v| {
                v.party_id == party_id
                    && v.verification_type == verification_type
                    && matches!(
                        v.status,
                        PartyVerificationStatus::Pending | PartyVerificationStatus::Approved
                    )
            })
            .cloned())
    }

    async fn list_by_party(&self, party_id: Uuid) -> Result<Vec<PartyVerification>, DomainError> {
        Ok(self
            .verifications
            .lock()
            .unwrap()
            .iter()
            .filter(|v| v.party_id == party_id)
            .cloned()
            .collect())
    }

    async fn list(
        &self,
        filters: &VerificationListFilters,
    ) -> Result<VerificationListResult, DomainError> {
        let all: Vec<PartyVerification> = self
            .verifications
            .lock()
            .unwrap()
            .iter()
            .filter(|v| {
                filters
                    .status
                    .as_ref()
                    .is_none_or(|s| v.status.as_str() == s)
            })
            .filter(|v| {
                filters
                    .verification_type
                    .as_ref()
                    .is_none_or(|t| v.verification_type.as_str() == t)
            })
            .filter(|v| filters.party_id.is_none_or(|p| v.party_id == p))
            .cloned()
            .collect();

        let total = all.len() as i64;
        let start = filters.offset as usize;
        let end = (filters.offset + filters.limit) as usize;
        let paginated = all
            .into_iter()
            .skip(start)
            .take(end.saturating_sub(start))
            .collect();

        Ok(VerificationListResult {
            verifications: paginated,
            total,
            limit: filters.limit,
            offset: filters.offset,
        })
    }

    async fn count(&self, filters: &VerificationListFilters) -> Result<i64, DomainError> {
        let count = self
            .verifications
            .lock()
            .unwrap()
            .iter()
            .filter(|v| {
                filters
                    .status
                    .as_ref()
                    .is_none_or(|s| v.status.as_str() == s)
            })
            .filter(|v| {
                filters
                    .verification_type
                    .as_ref()
                    .is_none_or(|t| v.verification_type.as_str() == t)
            })
            .filter(|v| filters.party_id.is_none_or(|p| v.party_id == p))
            .count() as i64;
        Ok(count)
    }

    async fn approve(
        &self,
        id: Uuid,
        reviewed_by_user_id: Uuid,
        review_notes: Option<String>,
    ) -> Result<(), DomainError> {
        let mut verifications = self.verifications.lock().unwrap();
        let v = verifications
            .iter_mut()
            .find(|v| v.id == id)
            .ok_or(DomainError::VerificationNotFound)?;
        if !matches!(v.status, PartyVerificationStatus::Pending) {
            return Err(DomainError::InvalidVerificationStateTransition {
                from: v.status.as_str().to_string(),
                to: "APPROVED".to_string(),
            });
        }
        v.approve(reviewed_by_user_id, review_notes);
        Ok(())
    }

    async fn reject(
        &self,
        id: Uuid,
        reviewed_by_user_id: Uuid,
        rejection_reason: String,
        review_notes: Option<String>,
    ) -> Result<(), DomainError> {
        let mut verifications = self.verifications.lock().unwrap();
        let v = verifications
            .iter_mut()
            .find(|v| v.id == id)
            .ok_or(DomainError::VerificationNotFound)?;
        if !matches!(v.status, PartyVerificationStatus::Pending) {
            return Err(DomainError::InvalidVerificationStateTransition {
                from: v.status.as_str().to_string(),
                to: "REJECTED".to_string(),
            });
        }
        v.reject(reviewed_by_user_id, rejection_reason, review_notes);
        Ok(())
    }

    async fn revoke(
        &self,
        id: Uuid,
        reviewed_by_user_id: Uuid,
        reason: String,
        review_notes: Option<String>,
    ) -> Result<(), DomainError> {
        let mut verifications = self.verifications.lock().unwrap();
        let v = verifications
            .iter_mut()
            .find(|v| v.id == id)
            .ok_or(DomainError::VerificationNotFound)?;
        if !matches!(v.status, PartyVerificationStatus::Approved) {
            return Err(DomainError::InvalidVerificationStateTransition {
                from: v.status.as_str().to_string(),
                to: "REVOKED".to_string(),
            });
        }
        v.revoke(reviewed_by_user_id, reason, review_notes);
        Ok(())
    }

    async fn sum_approved_points(&self, party_id: Uuid) -> Result<i64, DomainError> {
        let now = time::OffsetDateTime::now_utc();
        let sum: i32 = self
            .verifications
            .lock()
            .unwrap()
            .iter()
            .filter(|v| v.party_id == party_id)
            .filter(|v| matches!(v.status, PartyVerificationStatus::Approved))
            .filter(|v| v.expires_at.is_none_or(|exp| exp > now))
            .map(|v| v.points)
            .sum();
        Ok(sum as i64)
    }

    async fn count_by_status(&self, party_id: Uuid, status: &str) -> Result<i64, DomainError> {
        let status = PartyVerificationStatus::try_from(status)?;
        let count = self
            .verifications
            .lock()
            .unwrap()
            .iter()
            .filter(|v| v.party_id == party_id && v.status == status)
            .count() as i64;
        Ok(count)
    }

    async fn set_provider_reference(
        &self,
        id: Uuid,
        provider_reference: String,
        provider_payload: Option<serde_json::Value>,
    ) -> Result<(), DomainError> {
        let mut verifications = self.verifications.lock().unwrap();
        if let Some(v) = verifications.iter_mut().find(|v| v.id == id) {
            v.provider_reference = Some(provider_reference);
            v.provider_payload = provider_payload;
        }
        Ok(())
    }

    async fn mark_expired(&self, id: Uuid) -> Result<(), DomainError> {
        let mut verifications = self.verifications.lock().unwrap();
        if let Some(v) = verifications.iter_mut().find(|v| v.id == id) {
            if matches!(v.status, PartyVerificationStatus::Approved) {
                v.status = PartyVerificationStatus::Expired;
                v.updated_at = time::OffsetDateTime::now_utc();
            }
        }
        Ok(())
    }

    async fn update_verification_level(
        &self,
        _party_id: Uuid,
        _verification_level: i32,
    ) -> Result<(), DomainError> {
        Ok(())
    }
}

#[cfg(test)]
use crate::ports::{EncryptionService, MessageEvent, RealtimePublisher};
#[cfg(test)]
use domain::entities::{
    ChatRoom, ChatRoomMemberRole, ChatRoomMembership, ChatRoomType, Conversation, ConversationType,
    Message, MessageReaction, MessageRead, ReactionType, RecipientType,
};
#[cfg(test)]
use domain::repositories::{
    ChatRoomListQuery, ChatRoomRepository, ConversationSummary, MessageListQuery,
    MessageRepository, MessageWithMeta,
};

#[cfg(test)]
pub struct FakeEncryptionService;

#[cfg(test)]
#[async_trait]
impl EncryptionService for FakeEncryptionService {
    async fn encrypt(&self, plaintext: &str) -> Result<String, ApplicationError> {
        Ok(format!("enc:{plaintext}"))
    }

    async fn decrypt(&self, ciphertext: &str) -> Result<String, ApplicationError> {
        Ok(ciphertext
            .strip_prefix("enc:")
            .unwrap_or(ciphertext)
            .to_string())
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct RecordingPublisher {
    pub events: Mutex<Vec<MessageEvent>>,
}

#[cfg(test)]
#[async_trait]
impl RealtimePublisher for RecordingPublisher {
    async fn publish(&self, event: MessageEvent) -> Result<(), ApplicationError> {
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeMessageRepo {
    pub conversations: Mutex<HashMap<Uuid, Conversation>>,
    pub messages: Mutex<HashMap<Uuid, Message>>,
    pub reads: Mutex<HashMap<(Uuid, Uuid), MessageRead>>,
    pub reactions: Mutex<Vec<MessageReaction>>,
}

#[cfg(test)]
#[async_trait]
impl MessageRepository for FakeMessageRepo {
    async fn create_conversation(&self, conversation: &Conversation) -> Result<(), DomainError> {
        self.conversations
            .lock()
            .unwrap()
            .insert(conversation.id, conversation.clone());
        Ok(())
    }

    async fn find_conversation_by_id(&self, id: Uuid) -> Result<Option<Conversation>, DomainError> {
        Ok(self.conversations.lock().unwrap().get(&id).cloned())
    }

    async fn find_direct_user_conversation(
        &self,
        user_a_id: Uuid,
        user_b_id: Uuid,
    ) -> Result<Option<Conversation>, DomainError> {
        Ok(self
            .conversations
            .lock()
            .unwrap()
            .values()
            .find(|c| {
                c.conversation_type == ConversationType::DirectUser
                    && ((c.user_a_id == Some(user_a_id) && c.user_b_id == Some(user_b_id))
                        || (c.user_a_id == Some(user_b_id) && c.user_b_id == Some(user_a_id)))
            })
            .cloned())
    }

    async fn find_direct_party_conversation(
        &self,
        party_a_id: Uuid,
        party_b_id: Uuid,
    ) -> Result<Option<Conversation>, DomainError> {
        Ok(self
            .conversations
            .lock()
            .unwrap()
            .values()
            .find(|c| {
                c.conversation_type == ConversationType::DirectParty
                    && ((c.party_a_id == Some(party_a_id) && c.party_b_id == Some(party_b_id))
                        || (c.party_a_id == Some(party_b_id) && c.party_b_id == Some(party_a_id)))
            })
            .cloned())
    }

    async fn find_party_members_conversation(
        &self,
        party_id: Uuid,
    ) -> Result<Option<Conversation>, DomainError> {
        Ok(self
            .conversations
            .lock()
            .unwrap()
            .values()
            .find(|c| {
                c.conversation_type == ConversationType::PartyMembers
                    && c.party_id == Some(party_id)
            })
            .cloned())
    }

    async fn find_deal_conversation(
        &self,
        deal_id: Uuid,
    ) -> Result<Option<Conversation>, DomainError> {
        Ok(self
            .conversations
            .lock()
            .unwrap()
            .values()
            .find(|c| c.conversation_type == ConversationType::Deal && c.deal_id == Some(deal_id))
            .cloned())
    }

    async fn find_room_conversation(
        &self,
        room_id: Uuid,
    ) -> Result<Option<Conversation>, DomainError> {
        Ok(self
            .conversations
            .lock()
            .unwrap()
            .values()
            .find(|c| c.conversation_type == ConversationType::Room && c.room_id == Some(room_id))
            .cloned())
    }

    async fn touch_conversation(
        &self,
        conversation_id: Uuid,
        last_message_at: time::OffsetDateTime,
    ) -> Result<(), DomainError> {
        if let Some(c) = self.conversations.lock().unwrap().get_mut(&conversation_id) {
            c.last_message_at = last_message_at;
        }
        Ok(())
    }

    async fn create_message(&self, message: &Message) -> Result<(), DomainError> {
        self.messages
            .lock()
            .unwrap()
            .insert(message.id, message.clone());
        Ok(())
    }

    async fn find_message_by_id(&self, id: Uuid) -> Result<Option<Message>, DomainError> {
        Ok(self.messages.lock().unwrap().get(&id).cloned())
    }

    async fn list_messages(
        &self,
        conversation_id: Uuid,
        query: &MessageListQuery,
    ) -> Result<Vec<MessageWithMeta>, DomainError> {
        let before = query
            .before_id
            .and_then(|id| self.messages.lock().unwrap().get(&id).map(|m| m.created_at));
        let mut msgs: Vec<Message> = self
            .messages
            .lock()
            .unwrap()
            .values()
            .filter(|m| m.conversation_id == conversation_id)
            .filter(|m| before.is_none_or(|t| m.created_at < t))
            .cloned()
            .collect();
        msgs.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        let limited: Vec<Message> = msgs.into_iter().take(query.limit as usize).collect();
        let reactions = self.reactions.lock().unwrap();
        let result: Vec<MessageWithMeta> = limited
            .into_iter()
            .map(|m| {
                let likes = reactions
                    .iter()
                    .filter(|r| r.message_id == m.id && r.reaction_type == ReactionType::Like)
                    .count() as i64;
                let dislikes = reactions
                    .iter()
                    .filter(|r| r.message_id == m.id && r.reaction_type == ReactionType::Dislike)
                    .count() as i64;
                let reads = self
                    .reads
                    .lock()
                    .unwrap()
                    .values()
                    .filter(|r| r.message_id == m.id)
                    .count() as i64;
                MessageWithMeta {
                    message: m,
                    read_count: reads,
                    likes,
                    dislikes,
                    user_reaction: None,
                }
            })
            .collect();
        Ok(result)
    }

    async fn update_message(&self, message: &Message) -> Result<(), DomainError> {
        self.messages
            .lock()
            .unwrap()
            .insert(message.id, message.clone());
        Ok(())
    }

    async fn soft_delete_message(&self, id: Uuid) -> Result<(), DomainError> {
        if let Some(m) = self.messages.lock().unwrap().get_mut(&id) {
            m.is_deleted = true;
        }
        Ok(())
    }

    async fn set_message_pinned(
        &self,
        message_id: Uuid,
        is_pinned: bool,
        pinned_at: Option<time::OffsetDateTime>,
    ) -> Result<(), DomainError> {
        if let Some(m) = self.messages.lock().unwrap().get_mut(&message_id) {
            m.is_pinned = is_pinned;
            m.pinned_at = pinned_at;
        }
        Ok(())
    }

    async fn list_pinned_messages(
        &self,
        conversation_id: Uuid,
    ) -> Result<Vec<MessageWithMeta>, DomainError> {
        self.list_messages(
            conversation_id,
            &MessageListQuery {
                before_id: None,
                limit: i64::MAX,
            },
        )
        .await
        .map(|v| v.into_iter().filter(|m| m.message.is_pinned).collect())
    }

    async fn mark_read(&self, read: &MessageRead) -> Result<(), DomainError> {
        self.reads
            .lock()
            .unwrap()
            .entry((read.message_id, read.user_id))
            .or_insert_with(|| read.clone());
        Ok(())
    }

    async fn find_read(
        &self,
        message_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<MessageRead>, DomainError> {
        Ok(self
            .reads
            .lock()
            .unwrap()
            .get(&(message_id, user_id))
            .cloned())
    }

    async fn unread_count_for_user(
        &self,
        user_id: Uuid,
        _party_id: Option<Uuid>,
    ) -> Result<i64, DomainError> {
        let reads = self.reads.lock().unwrap();
        let count = self
            .messages
            .lock()
            .unwrap()
            .values()
            .filter(|m| m.sender_user_id != user_id && !m.is_deleted)
            .filter(|m| !reads.contains_key(&(m.id, user_id)))
            .count() as i64;
        Ok(count)
    }

    async fn list_conversations_for_user(
        &self,
        user_id: Uuid,
        party_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ConversationSummary>, DomainError> {
        let all: Vec<Conversation> = self
            .conversations
            .lock()
            .unwrap()
            .values()
            .filter(|c| {
                c.conversation_type == ConversationType::DirectUser
                    && (c.user_a_id == Some(user_id) || c.user_b_id == Some(user_id))
                    || c.conversation_type == ConversationType::DirectParty
                        && (c.party_a_id == party_id || c.party_b_id == party_id)
                    || c.conversation_type == ConversationType::PartyMembers
                        && c.party_id == party_id
                    || c.conversation_type == ConversationType::Deal
                    || c.conversation_type == ConversationType::Room
                    || c.conversation_type == ConversationType::AdminBroadcast
            })
            .cloned()
            .collect();
        let mut sorted = all;
        sorted.sort_by(|a, b| b.last_message_at.cmp(&a.last_message_at));
        let reads = self.reads.lock().unwrap();
        let msgs = self.messages.lock().unwrap();
        let summaries: Vec<ConversationSummary> = sorted
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .map(|c| {
                let unread = msgs
                    .values()
                    .filter(|m| m.conversation_id == c.id && m.sender_user_id != user_id)
                    .filter(|m| !reads.contains_key(&(m.id, user_id)))
                    .count() as i64;
                ConversationSummary {
                    conversation: c,
                    unread_count: unread,
                }
            })
            .collect();
        Ok(summaries)
    }

    async fn toggle_reaction(
        &self,
        reaction: &MessageReaction,
    ) -> Result<Option<MessageReaction>, DomainError> {
        let mut reactions = self.reactions.lock().unwrap();
        let key = (
            reaction.message_id,
            reaction.user_id,
            reaction.party_id,
            reaction.reaction_type,
        );
        if let Some(pos) = reactions
            .iter()
            .position(|r| (r.message_id, r.user_id, r.party_id, r.reaction_type) == key)
        {
            reactions.remove(pos);
            Ok(None)
        } else {
            reactions.push(reaction.clone());
            Ok(Some(reaction.clone()))
        }
    }

    async fn list_reactions_for_message(
        &self,
        message_id: Uuid,
    ) -> Result<Vec<MessageReaction>, DomainError> {
        Ok(self
            .reactions
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.message_id == message_id)
            .cloned()
            .collect())
    }

    async fn count_messages_by_recipient(
        &self,
        recipient_type: RecipientType,
        recipient_id: Uuid,
    ) -> Result<i64, DomainError> {
        let count = self
            .messages
            .lock()
            .unwrap()
            .values()
            .filter(|m| {
                m.recipient_type == recipient_type
                    && match recipient_type {
                        RecipientType::User => m.recipient_user_id == Some(recipient_id),
                        RecipientType::Party => m.recipient_party_id == Some(recipient_id),
                        RecipientType::Deal => m.recipient_deal_id == Some(recipient_id),
                        RecipientType::Room => m.recipient_room_id == Some(recipient_id),
                        _ => false,
                    }
            })
            .count() as i64;
        Ok(count)
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeChatRoomRepo {
    pub rooms: Mutex<HashMap<Uuid, ChatRoom>>,
    pub memberships: Mutex<Vec<ChatRoomMembership>>,
}

#[cfg(test)]
#[async_trait]
impl ChatRoomRepository for FakeChatRoomRepo {
    async fn create_room(&self, room: &ChatRoom) -> Result<(), DomainError> {
        self.rooms.lock().unwrap().insert(room.id, room.clone());
        Ok(())
    }

    async fn find_room_by_id(&self, id: Uuid) -> Result<Option<ChatRoom>, DomainError> {
        Ok(self.rooms.lock().unwrap().get(&id).cloned())
    }

    async fn find_room_by_name(&self, name: &str) -> Result<Option<ChatRoom>, DomainError> {
        Ok(self
            .rooms
            .lock()
            .unwrap()
            .values()
            .find(|r| r.name.as_str() == name)
            .cloned())
    }

    async fn update_room(&self, room: &ChatRoom) -> Result<(), DomainError> {
        self.rooms.lock().unwrap().insert(room.id, room.clone());
        Ok(())
    }

    async fn soft_delete_room(&self, id: Uuid) -> Result<(), DomainError> {
        if let Some(r) = self.rooms.lock().unwrap().get_mut(&id) {
            r.soft_delete();
        }
        Ok(())
    }

    async fn list_rooms(
        &self,
        query: &ChatRoomListQuery,
        visible_room_ids: &[Uuid],
    ) -> Result<Vec<ChatRoom>, DomainError> {
        let visible: std::collections::HashSet<Uuid> = visible_room_ids.iter().copied().collect();
        let mut rooms: Vec<ChatRoom> = self
            .rooms
            .lock()
            .unwrap()
            .values()
            .filter(|r| query.include_deleted || !r.is_deleted)
            .filter(|r| query.room_type.is_none_or(|t| r.room_type == t))
            .filter(|r| r.room_type == ChatRoomType::Public || visible.contains(&r.id))
            .cloned()
            .collect();
        rooms.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(rooms
            .into_iter()
            .skip(query.offset as usize)
            .take(query.limit as usize)
            .collect())
    }

    async fn count_rooms(
        &self,
        query: &ChatRoomListQuery,
        visible_room_ids: &[Uuid],
    ) -> Result<i64, DomainError> {
        let list = self.list_rooms(query, visible_room_ids).await?;
        Ok(list.len() as i64)
    }

    async fn add_membership(&self, membership: &ChatRoomMembership) -> Result<(), DomainError> {
        self.memberships.lock().unwrap().push(membership.clone());
        Ok(())
    }

    async fn remove_membership(&self, membership_id: Uuid) -> Result<(), DomainError> {
        self.memberships
            .lock()
            .unwrap()
            .retain(|m| m.id != membership_id);
        Ok(())
    }

    async fn find_membership_by_id(
        &self,
        membership_id: Uuid,
    ) -> Result<Option<ChatRoomMembership>, DomainError> {
        Ok(self
            .memberships
            .lock()
            .unwrap()
            .iter()
            .find(|m| m.id == membership_id)
            .cloned())
    }

    async fn find_membership_for_user(
        &self,
        room_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<ChatRoomMembership>, DomainError> {
        Ok(self
            .memberships
            .lock()
            .unwrap()
            .iter()
            .find(|m| m.room_id == room_id && m.user_id == Some(user_id))
            .cloned())
    }

    async fn find_membership_for_party(
        &self,
        room_id: Uuid,
        party_id: Uuid,
    ) -> Result<Option<ChatRoomMembership>, DomainError> {
        Ok(self
            .memberships
            .lock()
            .unwrap()
            .iter()
            .find(|m| m.room_id == room_id && m.party_id == Some(party_id))
            .cloned())
    }

    async fn update_membership_role(
        &self,
        membership_id: Uuid,
        role: ChatRoomMemberRole,
    ) -> Result<(), DomainError> {
        if let Some(m) = self
            .memberships
            .lock()
            .unwrap()
            .iter_mut()
            .find(|m| m.id == membership_id)
        {
            m.member_role = role;
        }
        Ok(())
    }

    async fn list_memberships_for_room(
        &self,
        room_id: Uuid,
    ) -> Result<Vec<ChatRoomMembership>, DomainError> {
        Ok(self
            .memberships
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.room_id == room_id)
            .cloned()
            .collect())
    }

    async fn list_room_ids_for_user(
        &self,
        user_id: Uuid,
        party_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, DomainError> {
        let party_set: std::collections::HashSet<Uuid> = party_ids.iter().copied().collect();
        let ids: std::collections::HashSet<Uuid> = self
            .memberships
            .lock()
            .unwrap()
            .iter()
            .filter(|m| {
                m.user_id == Some(user_id) || m.party_id.is_some_and(|p| party_set.contains(&p))
            })
            .map(|m| m.room_id)
            .collect();
        Ok(ids.into_iter().collect())
    }

    async fn is_user_in_room(
        &self,
        room_id: Uuid,
        user_id: Uuid,
        party_ids: &[Uuid],
    ) -> Result<bool, DomainError> {
        let party_set: std::collections::HashSet<Uuid> = party_ids.iter().copied().collect();
        Ok(self.memberships.lock().unwrap().iter().any(|m| {
            m.room_id == room_id
                && (m.user_id == Some(user_id)
                    || m.party_id.is_some_and(|p| party_set.contains(&p)))
        }))
    }

    async fn is_party_in_room(
        &self,
        room_id: Uuid,
        party_ids: &[Uuid],
    ) -> Result<bool, DomainError> {
        let party_set: std::collections::HashSet<Uuid> = party_ids.iter().copied().collect();
        Ok(self
            .memberships
            .lock()
            .unwrap()
            .iter()
            .any(|m| m.room_id == room_id && m.party_id.is_some_and(|p| party_set.contains(&p))))
    }

    async fn list_rooms_for_user(
        &self,
        user_id: Uuid,
        party_ids: &[Uuid],
    ) -> Result<Vec<ChatRoom>, DomainError> {
        let room_ids = self.list_room_ids_for_user(user_id, party_ids).await?;
        let set: std::collections::HashSet<Uuid> = room_ids.iter().copied().collect();
        Ok(self
            .rooms
            .lock()
            .unwrap()
            .values()
            .filter(|r| set.contains(&r.id))
            .cloned()
            .collect())
    }
}
