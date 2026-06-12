use domain::errors::DomainError;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ApplicationError {
    #[error("validation failed: {0:?}")]
    Validation(Vec<String>),

    #[error("user not found")]
    NotFound,

    #[error("a user with this email already exists")]
    DuplicateEmail,

    #[error("a user with this username already exists")]
    DuplicateUsername,

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
            | DomainError::InvalidPasswordHash { message } => {
                ApplicationError::Validation(vec![message])
            }
            DomainError::DuplicateEmail => ApplicationError::DuplicateEmail,
            DomainError::DuplicateUsername => ApplicationError::DuplicateUsername,
            DomainError::RepositoryError(msg) => ApplicationError::Infrastructure(msg),
        }
    }
}
