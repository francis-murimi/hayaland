use domain::errors::DomainError;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ApplicationError {
    #[error("validation failed: {0:?}")]
    Validation(Vec<String>),

    #[error("user not found")]
    NotFound,

    #[error("party not found")]
    PartyNotFound,

    #[error("role not found")]
    RoleNotFound,

    #[error("a user with this email already exists")]
    DuplicateEmail,

    #[error("a user with this username already exists")]
    DuplicateUsername,

    #[error("a party with this email already exists")]
    DuplicatePartyEmail,

    #[error("this role is already assigned to the party")]
    DuplicatePartyRole,

    #[error("party role has active deals and cannot be removed")]
    PartyRoleHasActiveDeals,

    #[error("party has active deals and cannot be deleted")]
    PartyHasActiveDeals,

    #[error("deal not found")]
    DealNotFound,

    #[error("deal participation not found")]
    DealParticipationNotFound,

    #[error("invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("invalid value distribution: {message}")]
    InvalidValueDistribution { message: String },

    #[error("win-win-win validation failed")]
    WinWinWinValidationFailed { violations: Vec<String> },

    #[error("deal access denied")]
    DealAccessDenied,

    #[error("weak password: {message}")]
    WeakPassword { message: String },

    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("account is inactive")]
    AccountInactive,

    #[error("unauthorized")]
    Unauthorized,

    #[error("forbidden")]
    Forbidden,

    #[error("admin users cannot be deactivated")]
    CannotDeactivateAdmin,

    #[error("cannot remove admin role from the first admin")]
    CannotRemoveFirstAdmin,

    #[error("failed to send email")]
    EmailSendFailed,

    #[error("invalid or expired verification token")]
    InvalidOrExpiredVerificationToken,

    #[error("invalid or expired password reset token")]
    InvalidOrExpiredPasswordResetToken,

    #[error("account is already verified")]
    AlreadyVerified,

    #[error("infrastructure error: {0}")]
    Infrastructure(String),
}

impl From<DomainError> for ApplicationError {
    fn from(err: DomainError) -> Self {
        match err {
            DomainError::InvalidEmail { message }
            | DomainError::InvalidUsername { message }
            | DomainError::InvalidPasswordHash { message }
            | DomainError::InvalidDisplayName { message }
            | DomainError::InvalidPhone { message }
            | DomainError::InvalidLocation { message }
            | DomainError::InvalidPartyType { message }
            | DomainError::InvalidVerificationStatus { message }
            | DomainError::InvalidDealRole { message }
            | DomainError::InvalidPartyMembershipRole { message }
            | DomainError::InvalidSearchParameters { message } => {
                ApplicationError::Validation(vec![message])
            }
            DomainError::DuplicateEmail => ApplicationError::DuplicateEmail,
            DomainError::DuplicateUsername => ApplicationError::DuplicateUsername,
            DomainError::DuplicatePartyEmail => ApplicationError::DuplicatePartyEmail,
            DomainError::DuplicatePartyRole => ApplicationError::DuplicatePartyRole,
            DomainError::PartyNotFound => ApplicationError::PartyNotFound,
            DomainError::RoleNotFound => ApplicationError::RoleNotFound,
            DomainError::PartyRoleHasActiveDeals => ApplicationError::PartyRoleHasActiveDeals,
            DomainError::PartyHasActiveDeals => ApplicationError::PartyHasActiveDeals,
            DomainError::InvalidDealStatus { message }
            | DomainError::InvalidParticipationStatus { message }
            | DomainError::InvalidDealTitle { message }
            | DomainError::InvalidValueDistribution { message } => {
                ApplicationError::Validation(vec![message])
            }
            DomainError::InvalidStateTransition { from, to } => {
                ApplicationError::InvalidStateTransition { from, to }
            }
            DomainError::DealNotFound => ApplicationError::DealNotFound,
            DomainError::DealParticipationNotFound => ApplicationError::DealParticipationNotFound,
            DomainError::TermNotFound
            | DomainError::MilestoneNotFound
            | DomainError::AgreementNotFound
            | DomainError::TransactionNotFound
            | DomainError::WalletNotFound
            | DomainError::MatchNotFound => ApplicationError::NotFound,
            DomainError::InsufficientPermissions => ApplicationError::DealAccessDenied,
            DomainError::WinWinWinValidationFailed { violations } => {
                ApplicationError::WinWinWinValidationFailed { violations }
            }
            DomainError::RepositoryError(msg) => ApplicationError::Infrastructure(msg),
        }
    }
}
