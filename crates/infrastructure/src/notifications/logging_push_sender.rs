use application::errors::ApplicationError;
use application::ports::{PushNotificationSender, PushResult};
use async_trait::async_trait;
use domain::repositories::PushTokenRepository;
use std::sync::Arc;
use uuid::Uuid;

/// A concrete push sender that resolves device tokens from the user_push_tokens registry,
/// logs each delivery attempt, and records per-token outcomes.
///
/// This adapter does not contact an external provider (FCM/APNs). In production it
/// would typically be replaced by a provider-specific sender that uses the same
/// token lookup; the logging here lets the platform test the full flow locally.
#[derive(Clone)]
pub struct LoggingPushSender {
    repo: Arc<dyn PushTokenRepository>,
}

impl LoggingPushSender {
    pub fn new(repo: Arc<dyn PushTokenRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl PushNotificationSender for LoggingPushSender {
    async fn send(
        &self,
        device_tokens: &[String],
        title: &str,
        body: &str,
        data: serde_json::Value,
    ) -> Result<Vec<PushResult>, ApplicationError> {
        tracing::info!(
            token_count = device_tokens.len(),
            title = %title,
            body = %body,
            data = %data,
            "sending push notification"
        );

        Ok(device_tokens
            .iter()
            .map(|token| PushResult {
                device_token: token.clone(),
                success: true,
                error: None,
            })
            .collect())
    }

    async fn send_to_user(
        &self,
        user_id: Uuid,
        title: &str,
        body: &str,
        data: serde_json::Value,
    ) -> Result<Vec<PushResult>, ApplicationError> {
        let tokens = self.repo.list_by_user(user_id).await.map_err(|e| {
            ApplicationError::Infrastructure(format!("push token lookup failed: {e}"))
        })?;

        let token_strings: Vec<String> = tokens.into_iter().map(|t| t.device_token).collect();
        tracing::info!(user_id = %user_id, token_count = token_strings.len(), "looked up push tokens for user");

        self.send(&token_strings, title, body, data).await
    }
}
