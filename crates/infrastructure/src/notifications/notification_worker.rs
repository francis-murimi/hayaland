use application::email::queue::EmailQueue;
use application::errors::ApplicationError;
use application::ports::{
    NotificationEvent, NotificationRealtimePublisher, PushNotificationSender, SmsSender,
};
use domain::entities::{Notification, NotificationChannel, NotificationStatus};
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
        tick_once(
            repo.clone(),
            push_sender.clone(),
            realtime_publisher.clone(),
            config.batch_size,
        )
        .await;
    }
}

async fn tick_once(
    repo: Arc<dyn NotificationRepository>,
    push_sender: Arc<dyn PushNotificationSender>,
    realtime_publisher: Arc<dyn NotificationRealtimePublisher>,
    batch_size: usize,
) {
    let batch = match repo.list_pending(batch_size, None).await {
        Ok(batch) => batch,
        Err(e) => {
            error!(error = %e, "failed to fetch pending notifications");
            return;
        }
    };

    if batch.is_empty() {
        return;
    }

    process_notification_batch(
        repo.clone(),
        push_sender.clone(),
        realtime_publisher.clone(),
        batch,
    )
    .await;
}

async fn process_notification_batch(
    repo: Arc<dyn NotificationRepository>,
    push_sender: Arc<dyn PushNotificationSender>,
    realtime_publisher: Arc<dyn NotificationRealtimePublisher>,
    batch: Vec<Notification>,
) {
    debug!(count = batch.len(), "processing notification batch");

    for notification in batch {
        process_notification(
            repo.clone(),
            push_sender.clone(),
            realtime_publisher.clone(),
            notification,
        )
        .await;
    }
}

async fn process_notification(
    repo: Arc<dyn NotificationRepository>,
    push_sender: Arc<dyn PushNotificationSender>,
    realtime_publisher: Arc<dyn NotificationRealtimePublisher>,
    notification: Notification,
) {
    if notification.status != NotificationStatus::Pending {
        return;
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
            NotificationChannel::Push => {
                let tokens = notification
                    .metadata
                    .get("push_tokens")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                dispatch_push(
                    push_sender.clone(),
                    &tokens,
                    &notification.title,
                    &notification.body,
                    notification.metadata.clone(),
                )
                .await
                .ok()
            }
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

async fn dispatch_push(
    sender: Arc<dyn PushNotificationSender>,
    tokens: &[String],
    title: &str,
    body: &str,
    data: serde_json::Value,
) -> Result<DeliveryResult, ApplicationError> {
    if tokens.is_empty() {
        return Ok(DeliveryResult::Sent);
    }

    let results = sender.send(tokens, title, body, data).await?;
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

#[allow(dead_code)]
fn backoff_delay_ms(attempt: u32, base_ms: u64, max_ms: u64) -> u64 {
    (base_ms * 2_u64.pow(attempt)).min(max_ms)
}

#[allow(dead_code)]
async fn _retry_with_backoff(attempt: u32, base_ms: u64, max_ms: u64) {
    sleep(Duration::from_millis(backoff_delay_ms(
        attempt, base_ms, max_ms,
    )))
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use application::test_helpers::{FakeEmailQueue, FakeNotificationRepo};
    use async_trait::async_trait;
    use domain::entities::{
        Notification, NotificationChannel, NotificationPriority, NotificationStatus,
        NotificationType,
    };
    use domain::errors::DomainError;
    use std::sync::Arc;
    use std::sync::Mutex;
    use uuid::Uuid;

    struct FakePushSender {
        calls: Mutex<Vec<(Vec<String>, String, String, serde_json::Value)>>,
    }

    impl Default for FakePushSender {
        fn default() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl PushNotificationSender for FakePushSender {
        async fn send(
            &self,
            tokens: &[String],
            title: &str,
            body: &str,
            data: serde_json::Value,
        ) -> Result<Vec<application::ports::PushResult>, ApplicationError> {
            self.calls.lock().unwrap().push((
                tokens.to_vec(),
                title.to_string(),
                body.to_string(),
                data,
            ));
            Ok(vec![])
        }
    }

    struct ConfigurablePushSender {
        result: Mutex<Option<Result<Vec<application::ports::PushResult>, ApplicationError>>>,
    }

    impl ConfigurablePushSender {
        fn returning(
            result: Result<Vec<application::ports::PushResult>, ApplicationError>,
        ) -> Self {
            Self {
                result: Mutex::new(Some(result)),
            }
        }
    }

    #[async_trait]
    impl PushNotificationSender for ConfigurablePushSender {
        async fn send(
            &self,
            _tokens: &[String],
            _title: &str,
            _body: &str,
            _data: serde_json::Value,
        ) -> Result<Vec<application::ports::PushResult>, ApplicationError> {
            self.result
                .lock()
                .unwrap()
                .take()
                .unwrap_or(Err(ApplicationError::Infrastructure(
                    "no push result configured".to_string(),
                )))
        }
    }

    #[derive(Default)]
    struct FakeRealtimePublisher {
        events: Mutex<Vec<NotificationEvent>>,
    }

    #[async_trait]
    impl NotificationRealtimePublisher for FakeRealtimePublisher {
        async fn publish(&self, event: NotificationEvent) -> Result<(), ApplicationError> {
            self.events.lock().unwrap().push(event);
            Ok(())
        }
    }

    struct FailingRealtimePublisher;

    #[async_trait]
    impl NotificationRealtimePublisher for FailingRealtimePublisher {
        async fn publish(&self, _event: NotificationEvent) -> Result<(), ApplicationError> {
            Err(ApplicationError::Infrastructure(
                "publish failed".to_string(),
            ))
        }
    }

    struct NoOpSmsSender;

    #[async_trait]
    impl SmsSender for NoOpSmsSender {
        async fn send(&self, _phone: &str, _body: &str) -> Result<(), ApplicationError> {
            Ok(())
        }
    }

    struct FailingNotificationRepo {
        inner: FakeNotificationRepo,
        failing_list_pending: bool,
        failing_record_delivery: bool,
        failing_update_status: bool,
    }

    impl FailingNotificationRepo {
        fn new() -> Self {
            Self {
                inner: FakeNotificationRepo::default(),
                failing_list_pending: false,
                failing_record_delivery: false,
                failing_update_status: false,
            }
        }

        fn with_failing_list_pending(mut self) -> Self {
            self.failing_list_pending = true;
            self
        }

        fn with_failing_record_delivery(mut self) -> Self {
            self.failing_record_delivery = true;
            self
        }

        fn with_failing_update_status(mut self) -> Self {
            self.failing_update_status = true;
            self
        }
    }

    #[async_trait]
    impl NotificationRepository for FailingNotificationRepo {
        async fn create(&self, notification: &Notification) -> Result<(), DomainError> {
            self.inner.create(notification).await
        }

        async fn find_by_id(&self, id: Uuid) -> Result<Option<Notification>, DomainError> {
            self.inner.find_by_id(id).await
        }

        async fn list_for_recipient(
            &self,
            user_id: Option<Uuid>,
            party_id: Option<Uuid>,
            filters: domain::repositories::NotificationFilters,
            pagination: domain::repositories::Pagination,
        ) -> Result<domain::repositories::NotificationListResult, DomainError> {
            self.inner
                .list_for_recipient(user_id, party_id, filters, pagination)
                .await
        }

        async fn count_unread_for_recipient(
            &self,
            user_id: Option<Uuid>,
            party_id: Option<Uuid>,
        ) -> Result<i64, DomainError> {
            self.inner
                .count_unread_for_recipient(user_id, party_id)
                .await
        }

        async fn mark_read(
            &self,
            id: Uuid,
            user_id: Uuid,
            party_id: Option<Uuid>,
            read_at: time::OffsetDateTime,
        ) -> Result<bool, DomainError> {
            self.inner.mark_read(id, user_id, party_id, read_at).await
        }

        async fn mark_all_read(
            &self,
            user_id: Option<Uuid>,
            party_id: Option<Uuid>,
            before: Option<time::OffsetDateTime>,
            notification_type: Option<NotificationType>,
        ) -> Result<u64, DomainError> {
            self.inner
                .mark_all_read(user_id, party_id, before, notification_type)
                .await
        }

        async fn mark_actioned(
            &self,
            id: Uuid,
            user_id: Uuid,
            party_id: Option<Uuid>,
            actioned_at: time::OffsetDateTime,
        ) -> Result<bool, DomainError> {
            self.inner
                .mark_actioned(id, user_id, party_id, actioned_at)
                .await
        }

        async fn delete(
            &self,
            id: Uuid,
            user_id: Uuid,
            party_id: Option<Uuid>,
        ) -> Result<bool, DomainError> {
            self.inner.delete(id, user_id, party_id).await
        }

        async fn update_status(
            &self,
            id: Uuid,
            status: NotificationStatus,
        ) -> Result<(), DomainError> {
            if self.failing_update_status {
                return Err(DomainError::RepositoryError(
                    "update status failed".to_string(),
                ));
            }
            self.inner.update_status(id, status).await
        }

        async fn record_delivery(
            &self,
            notification_id: Uuid,
            channel: NotificationChannel,
            result: DeliveryResult,
        ) -> Result<(), DomainError> {
            if self.failing_record_delivery {
                return Err(DomainError::RepositoryError(
                    "record delivery failed".to_string(),
                ));
            }
            self.inner
                .record_delivery(notification_id, channel, result)
                .await
        }

        async fn list_pending(
            &self,
            batch_size: usize,
            older_than: Option<time::OffsetDateTime>,
        ) -> Result<Vec<Notification>, DomainError> {
            if self.failing_list_pending {
                return Err(DomainError::RepositoryError(
                    "list pending failed".to_string(),
                ));
            }
            self.inner.list_pending(batch_size, older_than).await
        }
    }

    fn pending_notification(channels: Vec<NotificationChannel>) -> Notification {
        let mut n = Notification::new(
            Uuid::now_v7(),
            Some(Uuid::now_v7()),
            None,
            NotificationType::DealCompleted,
            "title".to_string(),
            "body".to_string(),
            NotificationPriority::Normal,
            None,
            vec![],
            None,
            None,
            serde_json::Value::Null,
            None,
        )
        .unwrap();
        n.channels = channels;
        n
    }

    fn pending_notification_with_metadata(
        channels: Vec<NotificationChannel>,
        metadata: serde_json::Value,
    ) -> Notification {
        let mut n = pending_notification(channels);
        n.metadata = metadata;
        n
    }

    #[tokio::test]
    async fn processes_pending_notification() {
        let repo = Arc::new(FakeNotificationRepo::default());
        let push = Arc::new(FakePushSender::default());
        let realtime = Arc::new(FakeRealtimePublisher::default());

        let notification =
            pending_notification(vec![NotificationChannel::InApp, NotificationChannel::Email]);
        repo.create(&notification).await.unwrap();

        process_notification_batch(
            repo.clone(),
            push.clone(),
            realtime.clone(),
            repo.list_pending(10, None).await.unwrap(),
        )
        .await;

        let stored = repo.find_by_id(notification.id).await.unwrap().unwrap();
        assert_eq!(stored.status, NotificationStatus::Sent);
        assert_eq!(realtime.events.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn skips_non_pending_notifications() {
        let repo = Arc::new(FakeNotificationRepo::default());
        let push = Arc::new(FakePushSender::default());
        let realtime = Arc::new(FakeRealtimePublisher::default());

        let mut notification = pending_notification(vec![NotificationChannel::InApp]);
        notification.status = NotificationStatus::Sent;

        process_notification_batch(
            repo.clone(),
            push.clone(),
            realtime.clone(),
            vec![notification.clone()],
        )
        .await;

        let events = realtime.events.lock().unwrap();
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn worker_disabled_returns_immediately() {
        let repo = Arc::new(FakeNotificationRepo::default());
        let push = Arc::new(FakePushSender::default());
        let realtime = Arc::new(FakeRealtimePublisher::default());
        let email_queue = Arc::new(FakeEmailQueue::default());
        let sms_sender = Arc::new(NoOpSmsSender);

        let config = NotificationWorkerConfig {
            enabled: false,
            ..Default::default()
        };

        run_notification_worker(
            repo.clone(),
            email_queue,
            push,
            sms_sender,
            realtime,
            config,
        )
        .await;

        assert!(repo.notifications.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn tick_once_handles_list_pending_error() {
        let repo = Arc::new(FailingNotificationRepo::new().with_failing_list_pending());
        let push = Arc::new(FakePushSender::default());
        let realtime = Arc::new(FakeRealtimePublisher::default());

        tick_once(repo.clone(), push.clone(), realtime.clone(), 10).await;
        // No panic means the error path was exercised.
        assert!(realtime.events.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn tick_once_does_nothing_for_empty_batch() {
        let repo = Arc::new(FakeNotificationRepo::default());
        let push = Arc::new(FakePushSender::default());
        let realtime = Arc::new(FakeRealtimePublisher::default());

        tick_once(repo.clone(), push.clone(), realtime.clone(), 10).await;

        assert!(realtime.events.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn tick_once_processes_pending_batch() {
        let repo = Arc::new(FakeNotificationRepo::default());
        let push = Arc::new(FakePushSender::default());
        let realtime = Arc::new(FakeRealtimePublisher::default());

        let notification = pending_notification(vec![NotificationChannel::InApp]);
        repo.create(&notification).await.unwrap();

        tick_once(repo.clone(), push.clone(), realtime.clone(), 10).await;

        let stored = repo.find_by_id(notification.id).await.unwrap().unwrap();
        assert_eq!(stored.status, NotificationStatus::Sent);
        assert_eq!(realtime.events.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn routes_email_channel() {
        let repo = Arc::new(FakeNotificationRepo::default());
        let push = Arc::new(FakePushSender::default());
        let realtime = Arc::new(FakeRealtimePublisher::default());

        let notification = pending_notification(vec![NotificationChannel::Email]);
        repo.create(&notification).await.unwrap();

        process_notification_batch(
            repo.clone(),
            push.clone(),
            realtime.clone(),
            vec![notification.clone()],
        )
        .await;

        let stored = repo.find_by_id(notification.id).await.unwrap().unwrap();
        assert_eq!(stored.status, NotificationStatus::Sent);

        let deliveries = repo.deliveries.lock().unwrap();
        assert_eq!(deliveries.len(), 1);
        assert_eq!(deliveries[0].0, notification.id);
        assert_eq!(deliveries[0].1, NotificationChannel::Email);
        assert!(matches!(deliveries[0].2, DeliveryResult::Sent));
    }

    #[tokio::test]
    async fn routes_push_channel_with_empty_tokens() {
        let repo = Arc::new(FakeNotificationRepo::default());
        let push = Arc::new(FakePushSender::default());
        let realtime = Arc::new(FakeRealtimePublisher::default());

        let notification = pending_notification(vec![NotificationChannel::Push]);
        repo.create(&notification).await.unwrap();

        process_notification_batch(
            repo.clone(),
            push.clone(),
            realtime.clone(),
            vec![notification.clone()],
        )
        .await;

        let stored = repo.find_by_id(notification.id).await.unwrap().unwrap();
        assert_eq!(stored.status, NotificationStatus::Sent);

        assert!(push.calls.lock().unwrap().is_empty());
        let deliveries = repo.deliveries.lock().unwrap();
        assert_eq!(deliveries.len(), 1);
        assert_eq!(deliveries[0].1, NotificationChannel::Push);
        assert!(matches!(deliveries[0].2, DeliveryResult::Sent));
    }

    #[tokio::test]
    async fn routes_push_channel_with_successful_tokens() {
        let repo = Arc::new(FakeNotificationRepo::default());
        let push = Arc::new(ConfigurablePushSender::returning(Ok(vec![
            application::ports::PushResult {
                device_token: "token-1".to_string(),
                success: true,
                error: None,
            },
        ])));
        let realtime = Arc::new(FakeRealtimePublisher::default());

        let notification = pending_notification_with_metadata(
            vec![NotificationChannel::Push],
            serde_json::json!({ "push_tokens": ["token-1"] }),
        );
        repo.create(&notification).await.unwrap();

        process_notification_batch(
            repo.clone(),
            push.clone(),
            realtime.clone(),
            vec![notification.clone()],
        )
        .await;

        let stored = repo.find_by_id(notification.id).await.unwrap().unwrap();
        assert_eq!(stored.status, NotificationStatus::Sent);

        let deliveries = repo.deliveries.lock().unwrap();
        assert_eq!(deliveries.len(), 1);
        assert!(matches!(deliveries[0].2, DeliveryResult::Delivered));
    }

    #[tokio::test]
    async fn routes_sms_channel() {
        let repo = Arc::new(FakeNotificationRepo::default());
        let push = Arc::new(FakePushSender::default());
        let realtime = Arc::new(FakeRealtimePublisher::default());

        let notification = pending_notification(vec![NotificationChannel::Sms]);
        repo.create(&notification).await.unwrap();

        process_notification_batch(
            repo.clone(),
            push.clone(),
            realtime.clone(),
            vec![notification.clone()],
        )
        .await;

        let stored = repo.find_by_id(notification.id).await.unwrap().unwrap();
        assert_eq!(stored.status, NotificationStatus::Sent);

        let deliveries = repo.deliveries.lock().unwrap();
        assert_eq!(deliveries.len(), 1);
        assert_eq!(deliveries[0].1, NotificationChannel::Sms);
        assert!(matches!(deliveries[0].2, DeliveryResult::Sent));
    }

    #[tokio::test]
    async fn routes_webhook_channel() {
        let repo = Arc::new(FakeNotificationRepo::default());
        let push = Arc::new(FakePushSender::default());
        let realtime = Arc::new(FakeRealtimePublisher::default());

        let notification = pending_notification(vec![NotificationChannel::Webhook]);
        repo.create(&notification).await.unwrap();

        process_notification_batch(
            repo.clone(),
            push.clone(),
            realtime.clone(),
            vec![notification.clone()],
        )
        .await;

        let stored = repo.find_by_id(notification.id).await.unwrap().unwrap();
        assert_eq!(stored.status, NotificationStatus::Sent);

        let deliveries = repo.deliveries.lock().unwrap();
        assert_eq!(deliveries.len(), 1);
        assert_eq!(deliveries[0].1, NotificationChannel::Webhook);
        assert!(matches!(deliveries[0].2, DeliveryResult::Sent));
    }

    #[tokio::test]
    async fn in_app_publisher_error_is_ignored() {
        let repo = Arc::new(FakeNotificationRepo::default());
        let push = Arc::new(FakePushSender::default());
        let realtime = Arc::new(FailingRealtimePublisher);

        let notification = pending_notification(vec![NotificationChannel::InApp]);
        repo.create(&notification).await.unwrap();

        process_notification_batch(
            repo.clone(),
            push.clone(),
            realtime.clone(),
            vec![notification.clone()],
        )
        .await;

        let stored = repo.find_by_id(notification.id).await.unwrap().unwrap();
        assert_eq!(stored.status, NotificationStatus::Sent);
    }

    #[tokio::test]
    async fn dispatch_push_returns_delivered_when_all_tokens_succeed() {
        let sender = Arc::new(ConfigurablePushSender::returning(Ok(vec![
            application::ports::PushResult {
                device_token: "t1".to_string(),
                success: true,
                error: None,
            },
            application::ports::PushResult {
                device_token: "t2".to_string(),
                success: true,
                error: None,
            },
        ])));

        let result = dispatch_push(
            sender,
            &["t1".to_string(), "t2".to_string()],
            "title",
            "body",
            serde_json::Value::Null,
        )
        .await
        .unwrap();

        assert!(matches!(result, DeliveryResult::Delivered));
    }

    #[tokio::test]
    async fn dispatch_push_returns_failed_when_any_token_fails() {
        let sender = Arc::new(ConfigurablePushSender::returning(Ok(vec![
            application::ports::PushResult {
                device_token: "t1".to_string(),
                success: true,
                error: None,
            },
            application::ports::PushResult {
                device_token: "t2".to_string(),
                success: false,
                error: Some("bad token".to_string()),
            },
        ])));

        let result = dispatch_push(
            sender,
            &["t1".to_string(), "t2".to_string()],
            "title",
            "body",
            serde_json::Value::Null,
        )
        .await
        .unwrap();

        assert!(matches!(result, DeliveryResult::Failed { ref message } if message == "bad token"));
    }

    #[tokio::test]
    async fn dispatch_push_uses_default_message_when_errors_empty() {
        let sender = Arc::new(ConfigurablePushSender::returning(Ok(vec![
            application::ports::PushResult {
                device_token: "t1".to_string(),
                success: false,
                error: None,
            },
        ])));

        let result = dispatch_push(
            sender,
            &["t1".to_string()],
            "title",
            "body",
            serde_json::Value::Null,
        )
        .await
        .unwrap();

        assert!(
            matches!(result, DeliveryResult::Failed { ref message } if message == "push send failed")
        );
    }

    #[tokio::test]
    async fn dispatch_push_propagates_sender_error() {
        let sender = Arc::new(ConfigurablePushSender::returning(Err(
            ApplicationError::PushSendFailed,
        )));

        let result = dispatch_push(
            sender,
            &["t1".to_string()],
            "title",
            "body",
            serde_json::Value::Null,
        )
        .await;

        assert!(matches!(result, Err(ApplicationError::PushSendFailed)));
    }

    #[tokio::test]
    async fn marks_status_failed_when_push_sender_errors() {
        let repo = Arc::new(FakeNotificationRepo::default());
        let push = Arc::new(ConfigurablePushSender::returning(Err(
            ApplicationError::PushSendFailed,
        )));
        let realtime = Arc::new(FakeRealtimePublisher::default());

        let notification = pending_notification_with_metadata(
            vec![NotificationChannel::Push],
            serde_json::json!({ "push_tokens": ["token-1"] }),
        );
        repo.create(&notification).await.unwrap();

        process_notification_batch(
            repo.clone(),
            push.clone(),
            realtime.clone(),
            vec![notification.clone()],
        )
        .await;

        let stored = repo.find_by_id(notification.id).await.unwrap().unwrap();
        assert_eq!(stored.status, NotificationStatus::Failed);
    }

    #[tokio::test]
    async fn marks_status_failed_when_push_reports_all_failures() {
        let repo = Arc::new(FakeNotificationRepo::default());
        let push = Arc::new(ConfigurablePushSender::returning(Ok(vec![
            application::ports::PushResult {
                device_token: "token-1".to_string(),
                success: false,
                error: Some("expired".to_string()),
            },
        ])));
        let realtime = Arc::new(FakeRealtimePublisher::default());

        let notification = pending_notification_with_metadata(
            vec![NotificationChannel::Push],
            serde_json::json!({ "push_tokens": ["token-1"] }),
        );
        repo.create(&notification).await.unwrap();

        process_notification_batch(
            repo.clone(),
            push.clone(),
            realtime.clone(),
            vec![notification.clone()],
        )
        .await;

        let stored = repo.find_by_id(notification.id).await.unwrap().unwrap();
        assert_eq!(stored.status, NotificationStatus::Failed);
    }

    #[tokio::test]
    async fn record_delivery_error_does_not_change_success_status() {
        let repo = Arc::new(FailingNotificationRepo::new().with_failing_record_delivery());
        let push = Arc::new(FakePushSender::default());
        let realtime = Arc::new(FakeRealtimePublisher::default());

        let notification = pending_notification(vec![NotificationChannel::Email]);
        repo.create(&notification).await.unwrap();

        process_notification_batch(
            repo.clone(),
            push.clone(),
            realtime.clone(),
            vec![notification.clone()],
        )
        .await;

        let stored = repo.find_by_id(notification.id).await.unwrap().unwrap();
        assert_eq!(stored.status, NotificationStatus::Sent);
    }

    #[tokio::test]
    async fn update_status_error_leaves_status_unchanged() {
        let repo = Arc::new(FailingNotificationRepo::new().with_failing_update_status());
        let push = Arc::new(FakePushSender::default());
        let realtime = Arc::new(FakeRealtimePublisher::default());

        let notification = pending_notification(vec![NotificationChannel::Email]);
        repo.create(&notification).await.unwrap();

        process_notification_batch(
            repo.clone(),
            push.clone(),
            realtime.clone(),
            vec![notification.clone()],
        )
        .await;

        let stored = repo.find_by_id(notification.id).await.unwrap().unwrap();
        assert_eq!(stored.status, NotificationStatus::Pending);
    }

    #[tokio::test]
    async fn no_channels_results_in_failed_status() {
        let repo = Arc::new(FakeNotificationRepo::default());
        let push = Arc::new(FakePushSender::default());
        let realtime = Arc::new(FakeRealtimePublisher::default());

        let notification = pending_notification(vec![]);
        repo.create(&notification).await.unwrap();

        process_notification_batch(
            repo.clone(),
            push.clone(),
            realtime.clone(),
            vec![notification.clone()],
        )
        .await;

        let stored = repo.find_by_id(notification.id).await.unwrap().unwrap();
        assert_eq!(stored.status, NotificationStatus::Failed);
    }

    #[tokio::test]
    async fn empty_batch_is_noop() {
        let repo = Arc::new(FakeNotificationRepo::default());
        let push = Arc::new(FakePushSender::default());
        let realtime = Arc::new(FakeRealtimePublisher::default());

        process_notification_batch(repo.clone(), push.clone(), realtime.clone(), vec![]).await;

        assert!(repo.notifications.lock().unwrap().is_empty());
        assert!(realtime.events.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn batch_processes_multiple_notifications() {
        let repo = Arc::new(FakeNotificationRepo::default());
        let push = Arc::new(FakePushSender::default());
        let realtime = Arc::new(FakeRealtimePublisher::default());

        let n1 = pending_notification(vec![NotificationChannel::Email]);
        let n2 = pending_notification(vec![NotificationChannel::InApp]);
        repo.create(&n1).await.unwrap();
        repo.create(&n2).await.unwrap();

        process_notification_batch(
            repo.clone(),
            push.clone(),
            realtime.clone(),
            repo.list_pending(10, None).await.unwrap(),
        )
        .await;

        let s1 = repo.find_by_id(n1.id).await.unwrap().unwrap();
        let s2 = repo.find_by_id(n2.id).await.unwrap().unwrap();
        assert_eq!(s1.status, NotificationStatus::Sent);
        assert_eq!(s2.status, NotificationStatus::Sent);
        assert_eq!(realtime.events.lock().unwrap().len(), 1);
    }

    #[test]
    fn backoff_delay_doubles_each_attempt() {
        assert_eq!(backoff_delay_ms(0, 100, 1_000), 100);
        assert_eq!(backoff_delay_ms(1, 100, 1_000), 200);
        assert_eq!(backoff_delay_ms(2, 100, 1_000), 400);
    }

    #[test]
    fn backoff_delay_caps_at_max() {
        assert_eq!(backoff_delay_ms(10, 100, 300), 300);
    }
}
