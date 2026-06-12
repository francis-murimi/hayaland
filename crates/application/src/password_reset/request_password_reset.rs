use crate::email::{build_password_reset_email, generate_verification_token, EmailSender};
use crate::errors::ApplicationError;
use crate::password_reset::dto::RequestPasswordResetCommand;
use domain::entities::{Email, PasswordResetToken};
use domain::repositories::{PasswordResetRepository, UserRepository};
use std::sync::Arc;
use time::OffsetDateTime;
use tracing::{info, instrument, warn};

#[derive(Clone)]
pub struct RequestPasswordReset {
    user_repo: Arc<dyn UserRepository>,
    reset_repo: Arc<dyn PasswordResetRepository>,
    email_sender: Arc<dyn EmailSender>,
    base_url: String,
    token_expiry_seconds: i64,
}

impl RequestPasswordReset {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        reset_repo: Arc<dyn PasswordResetRepository>,
        email_sender: Arc<dyn EmailSender>,
        base_url: String,
        token_expiry_seconds: i64,
    ) -> Self {
        Self {
            user_repo,
            reset_repo,
            email_sender,
            base_url,
            token_expiry_seconds,
        }
    }

    #[instrument(skip(self, cmd))]
    pub async fn execute(&self, cmd: RequestPasswordResetCommand) -> Result<(), ApplicationError> {
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

        self.reset_repo.invalidate_unused_for_user(user.id).await?;

        let token = generate_verification_token();
        let expires_at =
            OffsetDateTime::now_utc() + time::Duration::seconds(self.token_expiry_seconds);
        self.reset_repo
            .save(&PasswordResetToken::new(token.clone(), user.id, expires_at))
            .await?;

        let expiry_minutes = self.token_expiry_seconds / 60;
        let (subject, body) = build_password_reset_email(&self.base_url, &token, expiry_minutes);

        if let Err(e) = self
            .email_sender
            .send(user.email.as_str(), &subject, &body)
            .await
        {
            warn!(user_id = %user.id, error = %e, "failed to send password reset email");
            return Err(ApplicationError::EmailSendFailed);
        }

        info!(user_id = %user.id, "password reset email sent");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{test_user, FakeEmailSender, FakePasswordResetRepo, FakeRepo};
    use std::sync::Arc;

    fn service(
        user_repo: Arc<dyn UserRepository>,
        reset_repo: Arc<dyn PasswordResetRepository>,
        email_sender: Arc<dyn EmailSender>,
    ) -> RequestPasswordReset {
        RequestPasswordReset::new(
            user_repo,
            reset_repo,
            email_sender,
            "https://app.hayaland.local".to_string(),
            3600,
        )
    }

    #[tokio::test]
    async fn sends_email_for_existing_user() {
        let user = test_user("reset@example.com", "reset", "password123");
        let user_id = user.id;
        let user_repo = Arc::new(FakeRepo {
            users: std::sync::Mutex::new([(user_id, user)].into_iter().collect()),
        });
        let reset_repo = Arc::new(FakePasswordResetRepo::default());
        let sender = Arc::new(FakeEmailSender::default());

        service(user_repo, reset_repo.clone(), sender.clone())
            .execute(RequestPasswordResetCommand {
                email: "reset@example.com".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(sender.sent.lock().unwrap().len(), 1);
        assert_eq!(reset_repo.count_for_user(user_id).await, 1);
    }

    #[tokio::test]
    async fn ignores_unknown_email() {
        let user_repo = Arc::new(FakeRepo {
            users: std::sync::Mutex::new(std::collections::HashMap::new()),
        });
        let reset_repo = Arc::new(FakePasswordResetRepo::default());
        let sender = Arc::new(FakeEmailSender::default());

        service(user_repo, reset_repo, sender.clone())
            .execute(RequestPasswordResetCommand {
                email: "missing@example.com".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(sender.sent.lock().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn returns_error_when_email_sending_fails() {
        let user = test_user("fail@example.com", "fail", "password123");
        let user_id = user.id;
        let user_repo = Arc::new(FakeRepo {
            users: std::sync::Mutex::new([(user_id, user)].into_iter().collect()),
        });
        let reset_repo = Arc::new(FakePasswordResetRepo::default());
        let sender = Arc::new(FakeEmailSender::failing());

        let result = service(user_repo, reset_repo, sender)
            .execute(RequestPasswordResetCommand {
                email: "fail@example.com".to_string(),
            })
            .await;

        assert!(matches!(result, Err(ApplicationError::EmailSendFailed)));
    }
}
