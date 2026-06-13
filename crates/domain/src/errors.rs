use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum DomainError {
    #[error("invalid email: {message}")]
    InvalidEmail { message: String },

    #[error("invalid username: {message}")]
    InvalidUsername { message: String },

    #[error("invalid password hash: {message}")]
    InvalidPasswordHash { message: String },

    #[error("invalid display name: {message}")]
    InvalidDisplayName { message: String },

    #[error("invalid phone number: {message}")]
    InvalidPhone { message: String },

    #[error("invalid location: {message}")]
    InvalidLocation { message: String },

    #[error("invalid party type: {message}")]
    InvalidPartyType { message: String },

    #[error("invalid verification status: {message}")]
    InvalidVerificationStatus { message: String },

    #[error("invalid deal role: {message}")]
    InvalidDealRole { message: String },

    #[error("invalid party membership role: {message}")]
    InvalidPartyMembershipRole { message: String },

    #[error("a user with this email already exists")]
    DuplicateEmail,

    #[error("a user with this username already exists")]
    DuplicateUsername,

    #[error("a party with this email already exists")]
    DuplicatePartyEmail,

    #[error("this role is already assigned to the party")]
    DuplicatePartyRole,

    #[error("party not found")]
    PartyNotFound,

    #[error("role not found")]
    RoleNotFound,

    #[error("party role has active deals and cannot be removed")]
    PartyRoleHasActiveDeals,

    #[error("party has active deals and cannot be deleted")]
    PartyHasActiveDeals,

    #[error("invalid search parameters: {message}")]
    InvalidSearchParameters { message: String },

    #[error("repository error: {0}")]
    RepositoryError(String),
}
