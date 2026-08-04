use application::errors::ApplicationError;
use application::ports::SmsSender;
use async_trait::async_trait;

/// No-op SMS sender for tests and local development.
#[derive(Default, Clone)]
pub struct NoOpSmsSender {
    _private: (),
}

impl NoOpSmsSender {
    pub fn new() -> Self {
        Self { _private: () }
    }
}

#[async_trait]
impl SmsSender for NoOpSmsSender {
    async fn send(&self, _phone: &str, _body: &str) -> Result<(), ApplicationError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn send_returns_ok() {
        let sender = NoOpSmsSender::new();
        sender.send("+1234567890", "hello").await.unwrap();
    }
}
