use application::email::queue::{EmailQueue, EmailQueueItem};
use application::email::EmailSender;
use application::errors::ApplicationError;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::time::sleep;
use tracing::{debug, error, warn};

#[derive(Clone)]
pub struct InMemoryEmailQueue {
    sender: UnboundedSender<EmailQueueItem>,
}

impl InMemoryEmailQueue {
    pub fn new() -> (Self, UnboundedReceiver<EmailQueueItem>) {
        let (sender, receiver) = unbounded_channel();
        (Self { sender }, receiver)
    }
}

#[async_trait]
impl EmailQueue for InMemoryEmailQueue {
    async fn enqueue(&self, item: EmailQueueItem) -> Result<(), ApplicationError> {
        self.sender
            .send(item)
            .map_err(|_| ApplicationError::EmailSendFailed)?;
        Ok(())
    }
}

/// Background worker that drains the queue and sends emails via the configured transport.
pub async fn run_worker(
    mut receiver: UnboundedReceiver<EmailQueueItem>,
    sender: Arc<dyn EmailSender>,
    max_retries: u32,
    retry_base_delay_ms: u64,
    retry_max_delay_ms: u64,
) {
    while let Some(item) = receiver.recv().await {
        debug!(to = %item.to, subject = %item.subject, "sending queued email");

        let mut last_error = None;
        for attempt in 0..=max_retries {
            match sender.send(&item.to, &item.subject, &item.body).await {
                Ok(()) => {
                    debug!(to = %item.to, "queued email sent");
                    last_error = None;
                    break;
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries {
                        let delay =
                            (retry_base_delay_ms * 2_u64.pow(attempt)).min(retry_max_delay_ms);
                        warn!(
                            to = %item.to,
                            attempt = attempt + 1,
                            max_retries = max_retries,
                            delay_ms = delay,
                            "email send failed, retrying"
                        );
                        sleep(Duration::from_millis(delay)).await;
                    }
                }
            }
        }

        if let Some(e) = last_error {
            error!(
                error = %e,
                to = %item.to,
                attempts = max_retries + 1,
                "permanently failed to send queued email"
            );
        }
    }
    debug!("email worker shutting down");
}
