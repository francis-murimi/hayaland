use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// A single-use token that authorizes one password reset for a user account.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PasswordResetToken {
    pub token: String,
    pub user_id: Uuid,
    pub expires_at: OffsetDateTime,
    pub used: bool,
}

impl PasswordResetToken {
    pub fn new(token: impl Into<String>, user_id: Uuid, expires_at: OffsetDateTime) -> Self {
        Self {
            token: token.into(),
            user_id,
            expires_at,
            used: false,
        }
    }

    pub fn is_expired(&self, now: OffsetDateTime) -> bool {
        now >= self.expires_at
    }

    pub fn is_valid(&self, now: OffsetDateTime) -> bool {
        !self.used && !self.is_expired(now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_token_is_unused() {
        let t = PasswordResetToken::new("token", Uuid::nil(), OffsetDateTime::now_utc());
        assert!(!t.used);
    }

    #[test]
    fn expired_token_is_invalid() {
        let now = OffsetDateTime::now_utc();
        let t = PasswordResetToken::new("token", Uuid::nil(), now - time::Duration::seconds(1));
        assert!(t.is_expired(now));
        assert!(!t.is_valid(now));
    }

    #[test]
    fn unused_future_token_is_valid() {
        let now = OffsetDateTime::now_utc();
        let t = PasswordResetToken::new("token", Uuid::nil(), now + time::Duration::hours(1));
        assert!(!t.is_expired(now));
        assert!(t.is_valid(now));
    }

    #[test]
    fn used_token_is_invalid() {
        let now = OffsetDateTime::now_utc();
        let mut t = PasswordResetToken::new("token", Uuid::nil(), now + time::Duration::hours(1));
        t.used = true;
        assert!(!t.is_valid(now));
    }
}
