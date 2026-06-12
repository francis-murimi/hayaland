use crate::errors::ApplicationError;
use crate::users::create_user::PasswordHasher;
use crate::users::dto::{AuthenticateUserCommand, AuthenticateUserResult};
use crate::users::token::TokenGenerator;
use domain::entities::Email;
use domain::repositories::UserRepository;
use std::sync::Arc;

pub struct AuthenticateUser {
    repo: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
    token_generator: Arc<dyn TokenGenerator>,
}

impl AuthenticateUser {
    pub fn new(
        repo: Arc<dyn UserRepository>,
        hasher: Arc<dyn PasswordHasher>,
        token_generator: Arc<dyn TokenGenerator>,
    ) -> Self {
        Self {
            repo,
            hasher,
            token_generator,
        }
    }

    pub async fn execute(
        &self,
        cmd: AuthenticateUserCommand,
    ) -> Result<AuthenticateUserResult, ApplicationError> {
        let email = Email::new(&cmd.email).map_err(ApplicationError::from)?;

        let user = self
            .repo
            .find_by_email(&email)
            .await?
            .ok_or(ApplicationError::InvalidCredentials)?;

        let valid = self
            .hasher
            .verify_password(&cmd.password, user.password_hash.as_str())
            .await?;
        if !valid {
            return Err(ApplicationError::InvalidCredentials);
        }

        if !user.is_active {
            return Err(ApplicationError::AccountInactive);
        }

        let token = self.token_generator.generate(user.id).await?;
        Ok(AuthenticateUserResult {
            user_id: user.id,
            token,
        })
    }
}
