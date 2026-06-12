use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// A single-use email verification token bound to a user account.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmailVerification {
    pub token: String,
    pub user_id: Uuid,
    pub expires_at: OffsetDateTime,
    pub used: bool,
}

impl EmailVerification {
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
    fn new_verification_is_unused() {
        let v = EmailVerification::new("token", Uuid::nil(), OffsetDateTime::now_utc());
        assert!(!v.used);
    }

    #[test]
    fn expired_token_is_invalid() {
        let now = OffsetDateTime::now_utc();
        let v = EmailVerification::new("token", Uuid::nil(), now - time::Duration::seconds(1));
        assert!(v.is_expired(now));
        assert!(!v.is_valid(now));
    }

    #[test]
    fn unused_future_token_is_valid() {
        let now = OffsetDateTime::now_utc();
        let v = EmailVerification::new("token", Uuid::nil(), now + time::Duration::hours(24));
        assert!(!v.is_expired(now));
        assert!(v.is_valid(now));
    }

    #[test]
    fn used_token_is_invalid() {
        let now = OffsetDateTime::now_utc();
        let mut v = EmailVerification::new("token", Uuid::nil(), now + time::Duration::hours(24));
        v.used = true;
        assert!(!v.is_valid(now));
    }
}
