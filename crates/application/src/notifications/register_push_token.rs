use crate::errors::ApplicationError;
use domain::entities::PushToken;
use domain::repositories::PushTokenRepository;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

/// Command for registering (or refreshing) a push-notification token for a user device.
#[derive(Debug, Clone)]
pub struct RegisterPushTokenCommand {
    pub device_token: String,
    pub provider: String,
    pub device_type: Option<String>,
}

/// Register a push token for the authenticated user, upserting on
/// (user_id, device_token) and refreshing metadata.
#[derive(Clone)]
pub struct RegisterPushToken {
    repo: Arc<dyn PushTokenRepository>,
}

impl RegisterPushToken {
    pub fn new(repo: Arc<dyn PushTokenRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        user_id: Uuid,
        cmd: RegisterPushTokenCommand,
    ) -> Result<PushToken, ApplicationError> {
        if cmd.device_token.is_empty() {
            return Err(ApplicationError::Validation(vec![
                "device token cannot be empty".to_string(),
            ]));
        }
        if cmd.provider.is_empty() {
            return Err(ApplicationError::Validation(vec![
                "provider cannot be empty".to_string(),
            ]));
        }

        let mut token = PushToken::new(
            Uuid::now_v7(),
            user_id,
            cmd.device_token,
            cmd.provider,
            cmd.device_type,
        );
        token.last_used_at = Some(OffsetDateTime::now_utc());

        self.repo.upsert(&token).await?;
        Ok(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use domain::errors::DomainError;
    use std::sync::Mutex;
    use uuid::Uuid;

    #[derive(Default)]
    struct FakePushTokenRepository {
        tokens: Mutex<Vec<PushToken>>,
    }

    #[async_trait]
    impl PushTokenRepository for FakePushTokenRepository {
        async fn upsert(&self, token: &PushToken) -> Result<(), DomainError> {
            let mut tokens = self.tokens.lock().unwrap();
            if let Some(existing) = tokens
                .iter_mut()
                .find(|t| t.user_id == token.user_id && t.device_token == token.device_token)
            {
                *existing = token.clone();
            } else {
                tokens.push(token.clone());
            }
            Ok(())
        }

        async fn list_by_user(&self, _user_id: Uuid) -> Result<Vec<PushToken>, DomainError> {
            Ok(self.tokens.lock().unwrap().clone())
        }

        async fn find_by_id(&self, _id: Uuid) -> Result<Option<PushToken>, DomainError> {
            Ok(None)
        }

        async fn delete(&self, _id: Uuid) -> Result<bool, DomainError> {
            Ok(false)
        }

        async fn delete_by_user(&self, _user_id: Uuid) -> Result<u64, DomainError> {
            Ok(0)
        }

        async fn touch(&self, _id: Uuid) -> Result<(), DomainError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn registers_new_token() {
        let repo: Arc<dyn PushTokenRepository> = Arc::new(FakePushTokenRepository::default());
        let use_case = RegisterPushToken::new(repo.clone());
        let user_id = Uuid::now_v7();

        let token = use_case
            .execute(
                user_id,
                RegisterPushTokenCommand {
                    device_token: "token-1".to_string(),
                    provider: "FCM".to_string(),
                    device_type: Some("android".to_string()),
                },
            )
            .await
            .unwrap();

        assert_eq!(token.user_id, user_id);
        assert_eq!(token.device_token, "token-1");
        assert_eq!(token.provider, "FCM");
        assert!(token.last_used_at.is_some());
    }

    #[tokio::test]
    async fn rejects_empty_device_token() {
        let repo: Arc<dyn PushTokenRepository> = Arc::new(FakePushTokenRepository::default());
        let use_case = RegisterPushToken::new(repo);

        let err = use_case
            .execute(
                Uuid::now_v7(),
                RegisterPushTokenCommand {
                    device_token: "".to_string(),
                    provider: "FCM".to_string(),
                    device_type: None,
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, ApplicationError::Validation(_)));
    }

    #[tokio::test]
    async fn rejects_empty_provider() {
        let repo: Arc<dyn PushTokenRepository> = Arc::new(FakePushTokenRepository::default());
        let use_case = RegisterPushToken::new(repo);

        let err = use_case
            .execute(
                Uuid::now_v7(),
                RegisterPushTokenCommand {
                    device_token: "token-1".to_string(),
                    provider: "".to_string(),
                    device_type: None,
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, ApplicationError::Validation(_)));
    }
}
