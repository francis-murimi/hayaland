use application::errors::ApplicationError;
use application::ports::{PushNotificationSender, PushResult};
use async_trait::async_trait;

/// No-op push sender that records calls without contacting a provider.
#[derive(Default, Clone)]
pub struct NoOpPushSender {
    _private: (),
}

impl NoOpPushSender {
    pub fn new() -> Self {
        Self { _private: () }
    }
}

#[async_trait]
impl PushNotificationSender for NoOpPushSender {
    async fn send(
        &self,
        device_tokens: &[String],
        _title: &str,
        _body: &str,
        _data: serde_json::Value,
    ) -> Result<Vec<PushResult>, ApplicationError> {
        Ok(device_tokens
            .iter()
            .map(|t| PushResult {
                device_token: t.clone(),
                success: true,
                error: None,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn returns_success_for_each_token() {
        let sender = NoOpPushSender::new();
        let results = sender
            .send(
                &["token-a".into(), "token-b".into()],
                "title",
                "body",
                serde_json::Value::Null,
            )
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.success && r.error.is_none()));
        assert_eq!(results[0].device_token, "token-a");
        assert_eq!(results[1].device_token, "token-b");
    }
}
