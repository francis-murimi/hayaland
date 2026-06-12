use crate::errors::ApplicationError;
use async_trait::async_trait;

/// A single item waiting to be sent by the email worker.
#[derive(Debug, Clone)]
pub struct EmailQueueItem {
    pub to: String,
    pub subject: String,
    pub body: String,
}

/// Outbound port for queueing transactional emails.
#[async_trait]
pub trait EmailQueue: Send + Sync {
    async fn enqueue(&self, item: EmailQueueItem) -> Result<(), ApplicationError>;
}
