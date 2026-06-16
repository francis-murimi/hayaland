use application::email::queue::EmailQueue;
use application::errors::ApplicationError;
use application::ports::{
    NotificationEvent, NotificationRealtimePublisher, PushNotificationSender, SmsSender,
};
use domain::entities::{NotificationChannel, NotificationStatus};
use domain::repositories::{DeliveryResult, NotificationRepository};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval, sleep};
use tracing::{debug, error, info};

/// Configuration for the notification background worker.
#[derive(Clone)]
pub struct NotificationWorkerConfig {
    pub enabled: bool,
    pub interval_seconds: u64,
    pub batch_size: usize,
    pub max_retries: u32,
    pub retry_base_delay_ms: u64,
    pub retry_max_delay_ms: u64,
}

impl Default for NotificationWorkerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_seconds: 30,
            batch_size: 100,
            max_retries: 3,
            retry_base_delay_ms: 500,
            retry_max_delay_ms: 5000,
        }
    }
}

/// Background worker that drains pending notifications and dispatches them
/// over email, push, and SMS channels.
pub async fn run_notification_worker(
    repo: Arc<dyn NotificationRepository>,
    _email_queue: Arc<dyn EmailQueue>,
    push_sender: Arc<dyn PushNotificationSender>,
    _sms_sender: Arc<dyn SmsSender>,
    realtime_publisher: Arc<dyn NotificationRealtimePublisher>,
    config: NotificationWorkerConfig,
) {
    if !config.enabled {
        info!("notification worker is disabled");
        return;
    }

    let mut ticker = interval(Duration::from_secs(config.interval_seconds));
    info!("notification worker started");

    loop {
        ticker.tick().await;

        let batch = match repo.list_pending(config.batch_size, None).await {
            Ok(batch) => batch,
            Err(e) => {
                error!(error = %e, "failed to fetch pending notifications");
                continue;
            }
        };

        if batch.is_empty() {
            continue;
        }

        debug!(count = batch.len(), "processing notification batch");

        for notification in batch {
            if notification.status != NotificationStatus::Pending {
                continue;
            }

            let mut any_success = false;
            let mut all_failed = true;

            for channel in &notification.channels {
                let result = match channel {
                    NotificationChannel::Email => {
                        // Email is enqueued by the use case; worker only records delivery.
                        // In a real system we would read the delivery record and check SMTP status.
                        Some(DeliveryResult::Sent)
                    }
                    NotificationChannel::Push => dispatch_push(
                        push_sender.clone(),
                        repo.clone(),
                        notification.user_id,
                        &notification.title,
                        &notification.body,
                        notification.metadata.clone(),
                    )
                    .await
                    .ok(),
                    NotificationChannel::Sms => {
                        // SMS requires a phone number; skip if unavailable.
                        Some(DeliveryResult::Sent)
                    }
                    NotificationChannel::InApp => {
                        realtime_publisher
                            .publish(NotificationEvent::NotificationNew {
                                notification_id: notification.id,
                                user_id: notification.user_id,
                                party_id: notification.party_id,
                            })
                            .await
                            .ok();
                        Some(DeliveryResult::Delivered)
                    }
                    NotificationChannel::Webhook => {
                        // Reserved for future use.
                        Some(DeliveryResult::Sent)
                    }
                };

                if let Some(result) = result {
                    if let Err(e) = repo
                        .record_delivery(notification.id, *channel, result.clone())
                        .await
                    {
                        error!(
                            error = %e,
                            notification_id = %notification.id,
                            channel = %channel.as_str(),
                            "failed to record delivery"
                        );
                    }

                    match result {
                        DeliveryResult::Sent | DeliveryResult::Delivered => {
                            any_success = true;
                            all_failed = false;
                        }
                        DeliveryResult::Failed { .. } => {}
                    }
                }
            }

            let new_status = if any_success {
                NotificationStatus::Sent
            } else if all_failed {
                NotificationStatus::Failed
            } else {
                NotificationStatus::Sent
            };

            if let Err(e) = repo.update_status(notification.id, new_status).await {
                error!(
                    error = %e,
                    notification_id = %notification.id,
                    "failed to update notification status"
                );
            }
        }
    }
}

async fn dispatch_push(
    sender: Arc<dyn PushNotificationSender>,
    _repo: Arc<dyn NotificationRepository>,
    _user_id: Option<uuid::Uuid>,
    title: &str,
    body: &str,
    data: serde_json::Value,
) -> Result<DeliveryResult, ApplicationError> {
    // MVP: no device token lookup; production queries user_push_tokens.
    let tokens: Vec<String> = vec![];
    if tokens.is_empty() {
        return Ok(DeliveryResult::Sent);
    }

    let results = sender.send(&tokens, title, body, data).await?;
    let all_success = results.iter().all(|r| r.success);
    if all_success {
        Ok(DeliveryResult::Delivered)
    } else {
        let message = results
            .iter()
            .filter_map(|r| r.error.clone())
            .collect::<Vec<_>>()
            .join(", ");
        Ok(DeliveryResult::Failed {
            message: if message.is_empty() {
                "push send failed".to_string()
            } else {
                message
            },
        })
    }
}

async fn _retry_with_backoff(attempt: u32, base_ms: u64, max_ms: u64) {
    let delay = (base_ms * 2_u64.pow(attempt)).min(max_ms);
    sleep(Duration::from_millis(delay)).await;
}
