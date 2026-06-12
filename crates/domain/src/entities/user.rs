use crate::errors::DomainError;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;
use validator::ValidateEmail;

/// A validated email address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Email(String);

impl Email {
    pub fn new(value: &str) -> Result<Self, DomainError> {
        let trimmed = value.trim().to_lowercase();
        if !trimmed.validate_email() {
            return Err(DomainError::InvalidEmail {
                message: "email format is invalid".to_string(),
            });
        }
        Ok(Self(trimmed))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A validated username.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Username(String);

impl Username {
    pub fn new(value: &str) -> Result<Self, DomainError> {
        let trimmed = value.trim();
        let len = trimmed.chars().count();
        if !(3..=32).contains(&len)
            || !trimmed
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(DomainError::InvalidUsername {
                message: "username must be 3-32 characters and contain only letters, numbers, and underscores".to_string(),
            });
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An opaque password hash. Plaintext passwords must never reach this type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PasswordHash(String);

impl PasswordHash {
    pub fn new(hash: String) -> Result<Self, DomainError> {
        if hash.is_empty() {
            return Err(DomainError::InvalidPasswordHash {
                message: "password hash cannot be empty".to_string(),
            });
        }
        Ok(Self(hash))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The `User` aggregate root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: Email,
    pub username: Username,
    pub password_hash: PasswordHash,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl User {
    pub fn new(id: Uuid, email: Email, username: Username, password_hash: PasswordHash) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id,
            email,
            username,
            password_hash,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_rejects_invalid_input() {
        assert!(Email::new("not-an-email").is_err());
    }

    #[test]
    fn email_normalizes_case() {
        let email = Email::new("  Hello@Example.com  ").unwrap();
        assert_eq!(email.as_str(), "hello@example.com");
    }

    #[test]
    fn username_rejects_too_short() {
        assert!(Username::new("ab").is_err());
    }

    #[test]
    fn username_rejects_invalid_chars() {
        assert!(Username::new("foo bar").is_err());
    }

    #[test]
    fn username_accepts_valid_input() {
        let username = Username::new("valid_user_123").unwrap();
        assert_eq!(username.as_str(), "valid_user_123");
    }

    #[test]
    fn password_hash_rejects_empty() {
        assert!(PasswordHash::new(String::new()).is_err());
    }
}
