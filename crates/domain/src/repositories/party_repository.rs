use crate::entities::{
    DealRole, Email, Party, PartyType, RoleProfile, UserPartyMembership, VerificationStatus,
};
use crate::errors::DomainError;
use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

/// Criteria for searching and filtering parties.
#[derive(Debug, Clone, Default)]
pub struct PartySearchCriteria {
    pub query: Option<String>,
    pub roles: Vec<DealRole>,
    pub party_types: Vec<PartyType>,
    pub verification_statuses: Vec<VerificationStatus>,
    pub min_trust_score: Option<f64>,
    pub max_trust_score: Option<f64>,
    pub primary_domain_id: Option<Uuid>,
    pub active_only: Option<bool>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub radius_km: Option<f64>,
    pub limit: i64,
    pub offset: i64,
}

/// Outbound port for persisting and retrieving parties and party roles.
#[async_trait]
pub trait PartyRepository: Send + Sync {
    /// Create a new party.
    async fn create(&self, party: &Party) -> Result<(), DomainError>;

    /// Find a party by its unique id.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Party>, DomainError>;

    /// Find a party by its unique email address.
    async fn find_by_email(&self, email: &Email) -> Result<Option<Party>, DomainError>;

    /// Update party core fields.
    async fn update(&self, party: &Party) -> Result<(), DomainError>;

    /// Soft-delete a party by setting `is_active = false`.
    async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError>;

    /// List parties with optional filtering.
    async fn list(&self, criteria: &PartySearchCriteria) -> Result<Vec<Party>, DomainError>;

    /// Count parties matching the criteria.
    async fn count(&self, criteria: &PartySearchCriteria) -> Result<i64, DomainError>;

    /// Add a role to a party.
    async fn add_role(
        &self,
        party_id: Uuid,
        role: DealRole,
        profile: RoleProfile,
    ) -> Result<(), DomainError>;

    /// Remove a role from a party. Fails if the role has active deals.
    async fn remove_role(&self, party_id: Uuid, role: DealRole) -> Result<(), DomainError>;

    /// List active roles for a party.
    async fn list_roles(&self, party_id: Uuid)
        -> Result<Vec<(DealRole, RoleProfile)>, DomainError>;

    /// Check whether a party has the given active role.
    async fn has_role(&self, party_id: Uuid, role: DealRole) -> Result<bool, DomainError>;

    /// Count active deals for a party in a specific role.
    async fn count_active_deals_for_role(
        &self,
        party_id: Uuid,
        role: DealRole,
    ) -> Result<i64, DomainError>;

    /// Count all active deals for a party regardless of role.
    async fn count_active_deals(&self, party_id: Uuid) -> Result<i64, DomainError>;

    /// Create a user-party membership.
    async fn add_membership(&self, membership: &UserPartyMembership) -> Result<(), DomainError>;

    /// List memberships for a user.
    async fn list_memberships_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<(UserPartyMembership, Party)>, DomainError>;

    /// Find membership by user and party.
    async fn find_membership(
        &self,
        user_id: Uuid,
        party_id: Uuid,
    ) -> Result<Option<UserPartyMembership>, DomainError>;

    /// Update the `updated_at` timestamp of a party.
    async fn touch(&self, id: Uuid, updated_at: OffsetDateTime) -> Result<(), DomainError>;

    /// Check whether a user is an active member of a party.
    async fn is_user_member_of_party(
        &self,
        user_id: Uuid,
        party_id: Uuid,
    ) -> Result<bool, DomainError>;
}
