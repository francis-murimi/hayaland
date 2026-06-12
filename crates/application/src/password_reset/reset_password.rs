use crate::errors::ApplicationError;
use crate::password_reset::dto::{ResetPasswordCommand, ResetPasswordResult};
use crate::users::create_user::PasswordHasher;
use domain::repositories::{PasswordResetRepository, UserRepository};
use std::sync::Arc;
use time::OffsetDateTime;
use tracing::{info, instrument};

#[derive(Clone)]
pub struct ResetPassword {
    user_repo: Arc<dyn UserRepository>,
    reset_repo: Arc<dyn PasswordResetRepository>,
    hasher: Arc<dyn PasswordHasher>,
}

impl ResetPassword {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        reset_repo: Arc<dyn PasswordResetRepository>,
        hasher: Arc<dyn PasswordHasher>,
    ) -> Self {
        Self {
            user_repo,
            reset_repo,
            hasher,
        }
    }

    #[instrument(skip(self, cmd))]
    pub async fn execute(
        &self,
        cmd: ResetPasswordCommand,
    ) -> Result<ResetPasswordResult, ApplicationError> {
        validate_password(&cmd.password)?;

        let now = OffsetDateTime::now_utc();
        let token = self
            .reset_repo
            .find_by_token(&cmd.token)
            .await?
            .filter(|t| t.is_valid(now))
            .ok_or(ApplicationError::InvalidOrExpiredPasswordResetToken)?;

        let mut user = self
            .user_repo
            .find_by_id(token.user_id)
            .await?
            .ok_or(ApplicationError::NotFound)?;

        let hash = self.hasher.hash_password(&cmd.password).await?;
        user.password_hash =
            domain::entities::PasswordHash::new(hash).map_err(ApplicationError::from)?;
        user.is_active = true;
        user.updated_at = now;

        self.user_repo.update(&user).await?;
        self.reset_repo.mark_used(&cmd.token).await?;

        info!(user_id = %user.id, "password reset completed");
        Ok(ResetPasswordResult { user_id: user.id })
    }
}

fn validate_password(password: &str) -> Result<(), ApplicationError> {
    if password.len() < 8 {
        return Err(ApplicationError::WeakPassword {
            message: "password must be at least 8 characters".to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{test_user, FakeHasher, FakePasswordResetRepo, FakeRepo};
    use domain::entities::PasswordResetToken;
    use std::sync::Arc;

    fn service(
        user_repo: Arc<dyn UserRepository>,
        reset_repo: Arc<dyn PasswordResetRepository>,
    ) -> ResetPassword {
        ResetPassword::new(user_repo, reset_repo, Arc::new(FakeHasher))
    }

    #[tokio::test]
    async fn resets_password_and_marks_token_used() {
        let mut user = test_user("reset@example.com", "reset", "password123");
        user.is_active = true;
        let user_id = user.id;
        let user_repo = Arc::new(FakeRepo {
            users: std::sync::Mutex::new([(user_id, user)].into_iter().collect()),
        });
        let reset_repo = Arc::new(FakePasswordResetRepo::default());
        let token = "valid-token".to_string();
        reset_repo
            .save(&PasswordResetToken::new(
                token.clone(),
                user_id,
                OffsetDateTime::now_utc() + time::Duration::hours(1),
            ))
            .await
            .unwrap();

        let result = service(user_repo.clone(), reset_repo.clone())
            .execute(ResetPasswordCommand {
                token,
                password: "newpassword123".to_string(),
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().user_id, user_id);

        let updated = user_repo.find_by_id(user_id).await.unwrap().unwrap();
        assert!(updated.is_active);
        let valid = FakeHasher
            .verify_password("newpassword123", updated.password_hash.as_str())
            .await
            .unwrap();
        assert!(valid);
        assert!(
            reset_repo
                .find_by_token("valid-token")
                .await
                .unwrap()
                .unwrap()
                .used
        );
    }

    #[tokio::test]
    async fn rejects_expired_token() {
        let user = test_user("reset@example.com", "reset", "password123");
        let user_id = user.id;
        let user_repo = Arc::new(FakeRepo {
            users: std::sync::Mutex::new([(user_id, user)].into_iter().collect()),
        });
        let reset_repo = Arc::new(FakePasswordResetRepo::default());
        let token = "expired-token".to_string();
        reset_repo
            .save(&PasswordResetToken::new(
                token.clone(),
                user_id,
                OffsetDateTime::now_utc() - time::Duration::seconds(1),
            ))
            .await
            .unwrap();

        let result = service(user_repo, reset_repo)
            .execute(ResetPasswordCommand {
                token,
                password: "newpassword123".to_string(),
            })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::InvalidOrExpiredPasswordResetToken)
        ));
    }

    #[tokio::test]
    async fn rejects_short_password() {
        let user = test_user("reset@example.com", "reset", "password123");
        let user_id = user.id;
        let user_repo = Arc::new(FakeRepo {
            users: std::sync::Mutex::new([(user_id, user)].into_iter().collect()),
        });
        let reset_repo = Arc::new(FakePasswordResetRepo::default());
        let token = "token".to_string();
        reset_repo
            .save(&PasswordResetToken::new(
                token.clone(),
                user_id,
                OffsetDateTime::now_utc() + time::Duration::hours(1),
            ))
            .await
            .unwrap();

        let result = service(user_repo, reset_repo)
            .execute(ResetPasswordCommand {
                token,
                password: "short".to_string(),
            })
            .await;

        assert!(matches!(result, Err(ApplicationError::WeakPassword { .. })));
    }
}
