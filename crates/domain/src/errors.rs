use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum DomainError {
    #[error("invalid email: {message}")]
    InvalidEmail { message: String },

    #[error("invalid username: {message}")]
    InvalidUsername { message: String },

    #[error("invalid password hash: {message}")]
    InvalidPasswordHash { message: String },

    #[error("a user with this email already exists")]
    DuplicateEmail,

    #[error("a user with this username already exists")]
    DuplicateUsername,

    #[error("repository error: {0}")]
    RepositoryError(String),
}
