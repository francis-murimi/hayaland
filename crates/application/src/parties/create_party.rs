use crate::errors::ApplicationError;
use crate::parties::dto::{CreatePartyCommand, PartyResult};
use crate::payments::CreateWallet;
use domain::entities::{
    DisplayName, Email, GeoPoint, Party, PartyMembershipRole, RoleProfile, UserPartyMembership,
};
use domain::repositories::{PartyRepository, WalletRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Create a new party and assign the actor as owner.
#[derive(Clone)]
pub struct CreateParty {
    repo: Arc<dyn PartyRepository>,
    wallet_repo: Option<Arc<dyn WalletRepository>>,
}

impl CreateParty {
    /// Basic constructor used by tests that do not exercise the wallet.
    pub fn new(repo: Arc<dyn PartyRepository>) -> Self {
        Self {
            repo,
            wallet_repo: None,
        }
    }

    /// Full constructor that also creates the party's wallet container.
    pub fn new_with_wallet(
        repo: Arc<dyn PartyRepository>,
        wallet_repo: Arc<dyn WalletRepository>,
    ) -> Self {
        Self {
            repo,
            wallet_repo: Some(wallet_repo),
        }
    }

    #[instrument(skip(self, cmd), fields(email = %cmd.email, display_name = %cmd.display_name))]
    pub async fn execute(&self, cmd: CreatePartyCommand) -> Result<PartyResult, ApplicationError> {
        let email = Email::new(&cmd.email).map_err(ApplicationError::from)?;
        let display_name = DisplayName::new(&cmd.display_name).map_err(ApplicationError::from)?;

        if self.repo.find_by_email(&email).await?.is_some() {
            return Err(ApplicationError::DuplicatePartyEmail);
        }

        let id = Uuid::now_v7();
        let mut party = Party::new(id, cmd.party_type, display_name, email);

        if let Some(phone) = cmd.phone {
            party.phone =
                Some(domain::entities::Phone::new(&phone).map_err(ApplicationError::from)?);
        }

        party.tax_id = cmd.tax_id;
        party.primary_domain_id = cmd.primary_domain_id;

        if let (Some(lat), Some(lng)) = (cmd.latitude, cmd.longitude) {
            party.location = Some(GeoPoint::new(lat, lng).map_err(ApplicationError::from)?);
        }

        party.service_radius_km = cmd.service_radius_km;

        self.repo.create(&party).await?;

        if let Some(wallet_repo) = &self.wallet_repo {
            CreateWallet::new(wallet_repo.clone()).execute(id).await?;
        }

        let membership = UserPartyMembership::new(
            Uuid::now_v7(),
            cmd.actor_user_id,
            id,
            PartyMembershipRole::Owner,
        );
        self.repo.add_membership(&membership).await?;

        for role in cmd.roles {
            self.repo
                .add_role(id, role, RoleProfile::for_role(role))
                .await?;
        }

        info!(%id, actor = %cmd.actor_user_id, "created party");
        Ok(map_party_to_result(party))
    }
}

pub(crate) fn map_party_to_result(party: Party) -> PartyResult {
    PartyResult {
        id: party.id,
        party_type: party.party_type,
        display_name: party.display_name.as_str().to_owned(),
        email: party.email.as_str().to_owned(),
        phone: party.phone.as_ref().map(|p| p.as_str().to_owned()),
        tax_id: party.tax_id,
        verification_status: party.verification_status,
        primary_domain_id: party.primary_domain_id,
        latitude: party.location.map(|l| l.latitude),
        longitude: party.location.map(|l| l.longitude),
        service_radius_km: party.service_radius_km,
        trust_score: party.trust_score,
        total_deals_completed: party.total_deals_completed,
        total_deals_initiated: party.total_deals_initiated,
        is_active: party.is_active,
        created_at: party.created_at,
        updated_at: party.updated_at,
    }
}
