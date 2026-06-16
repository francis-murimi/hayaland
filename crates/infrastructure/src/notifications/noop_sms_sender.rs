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
