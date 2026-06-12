use crate::email::dto::ResendVerificationCommand;
use crate::email::{build_verification_email, generate_verification_token, EmailSender};
use crate::errors::ApplicationError;
use domain::entities::{Email, EmailVerification};
use domain::repositories::{EmailVerificationRepository, UserRepository};
use std::sync::Arc;
use time::OffsetDateTime;
use tracing::{info, instrument, warn};

#[derive(Clone)]
pub struct ResendVerificationEmail {
    user_repo: Arc<dyn UserRepository>,
    verification_repo: Arc<dyn EmailVerificationRepository>,
    email_sender: Arc<dyn EmailSender>,
    base_url: String,
    token_expiry_seconds: i64,
}

impl ResendVerificationEmail {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        verification_repo: Arc<dyn EmailVerificationRepository>,
        email_sender: Arc<dyn EmailSender>,
        base_url: String,
        token_expiry_seconds: i64,
    ) -> Self {
        Self {
            user_repo,
            verification_repo,
            email_sender,
            base_url,
            token_expiry_seconds,
        }
    }

    #[instrument(skip(self, cmd))]
    pub async fn execute(&self, cmd: ResendVerificationCommand) -> Result<(), ApplicationError> {
        let email = match Email::new(&cmd.email) {
            Ok(e) => e,
            Err(_) => {
                // Silently ignore malformed emails to prevent user enumeration.
                return Ok(());
            }
        };

        let user = match self.user_repo.find_by_email(&email).await? {
            Some(u) => u,
            None => return Ok(()),
        };

        if user.is_active {
            return Ok(());
        }

        self.verification_repo
            .invalidate_unused_for_user(user.id)
            .await?;

        let token = generate_verification_token();
        let expires_at =
            OffsetDateTime::now_utc() + time::Duration::seconds(self.token_expiry_seconds);
        self.verification_repo
            .save(&EmailVerification::new(token.clone(), user.id, expires_at))
            .await?;

        let expiry_hours = self.token_expiry_seconds / 3600;
        let (subject, body) = build_verification_email(&self.base_url, &token, expiry_hours);

        if let Err(e) = self
            .email_sender
            .send(user.email.as_str(), &subject, &body)
            .await
        {
            warn!(user_id = %user.id, error = %e, "failed to resend verification email");
            return Err(e);
        }

        info!(user_id = %user.id, "verification email resent");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{test_user, FakeEmailSender, FakeEmailVerificationRepo, FakeRepo};
    use std::sync::Arc;

    fn service(
        user_repo: Arc<dyn UserRepository>,
        verification_repo: Arc<dyn EmailVerificationRepository>,
        email_sender: Arc<dyn EmailSender>,
    ) -> ResendVerificationEmail {
        ResendVerificationEmail::new(
            user_repo,
            verification_repo,
            email_sender,
            "https://app.hayaland.local".to_string(),
            86400,
        )
    }

    #[tokio::test]
    async fn sends_email_for_inactive_user() {
        let mut user = test_user("resend@example.com", "resend", "password123");
        user.is_active = false;
        let user_id = user.id;
        let user_repo = Arc::new(FakeRepo {
            users: std::sync::Mutex::new([(user_id, user)].into_iter().collect()),
        });
        let verification_repo = Arc::new(FakeEmailVerificationRepo::default());
        let sender = Arc::new(FakeEmailSender::default());

        service(user_repo, verification_repo.clone(), sender.clone())
            .execute(ResendVerificationCommand {
                email: "resend@example.com".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(sender.sent.lock().unwrap().len(), 1);
        assert_eq!(verification_repo.count_for_user(user_id).await, 1);
    }

    #[tokio::test]
    async fn ignores_active_user() {
        let user = test_user("active@example.com", "active", "password123");
        let user_id = user.id;
        let user_repo = Arc::new(FakeRepo {
            users: std::sync::Mutex::new([(user_id, user)].into_iter().collect()),
        });
        let verification_repo = Arc::new(FakeEmailVerificationRepo::default());
        let sender = Arc::new(FakeEmailSender::default());

        service(user_repo, verification_repo.clone(), sender.clone())
            .execute(ResendVerificationCommand {
                email: "active@example.com".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(sender.sent.lock().unwrap().len(), 0);
        assert_eq!(verification_repo.count_for_user(user_id).await, 0);
    }

    #[tokio::test]
    async fn ignores_unknown_email() {
        let user_repo = Arc::new(FakeRepo {
            users: std::sync::Mutex::new(std::collections::HashMap::new()),
        });
        let verification_repo = Arc::new(FakeEmailVerificationRepo::default());
        let sender = Arc::new(FakeEmailSender::default());

        service(user_repo, verification_repo, sender.clone())
            .execute(ResendVerificationCommand {
                email: "missing@example.com".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(sender.sent.lock().unwrap().len(), 0);
    }
}
