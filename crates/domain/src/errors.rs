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

    #[error("invalid deal status: {message}")]
    InvalidDealStatus { message: String },

    #[error("invalid participation status: {message}")]
    InvalidParticipationStatus { message: String },

    #[error("invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("invalid deal title: {message}")]
    InvalidDealTitle { message: String },

    #[error("deal not found")]
    DealNotFound,

    #[error("deal participation not found")]
    DealParticipationNotFound,

    #[error("term not found")]
    TermNotFound,

    #[error("milestone not found")]
    MilestoneNotFound,

    #[error("agreement not found")]
    AgreementNotFound,

    #[error("transaction not found")]
    TransactionNotFound,

    #[error("wallet not found")]
    WalletNotFound,

    #[error("match suggestion not found")]
    MatchNotFound,

    #[error("insufficient permissions")]
    InsufficientPermissions,

    #[error("validation failed: {0:?}")]
    Validation(Vec<String>),

    #[error("invalid value distribution: {message}")]
    InvalidValueDistribution { message: String },

    #[error("win-win-win validation failed")]
    WinWinWinValidationFailed { violations: Vec<String> },

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
