use application::errors::ApplicationError;
use application::ports::{NotificationEvent, NotificationRealtimePublisher};
use async_trait::async_trait;
use std::sync::Arc;

/// Real-time publisher that forwards notification events to connected WebSocket sessions.
#[derive(Clone)]
pub struct NotificationWebSocketPublisher<R> {
    registry: Arc<R>,
}

impl<R> NotificationWebSocketPublisher<R> {
    pub fn new(registry: R) -> Self {
        Self {
            registry: Arc::new(registry),
        }
    }
}

#[async_trait]
impl<R> NotificationRealtimePublisher for NotificationWebSocketPublisher<R>
where
    R: NotificationRegistry + Send + Sync,
{
    async fn publish(&self, event: NotificationEvent) -> Result<(), ApplicationError> {
        self.registry.notify(event).await;
        Ok(())
    }
}

/// Trait abstracting how notification events reach connected clients.
#[async_trait]
pub trait NotificationRegistry: Send + Sync {
    async fn notify(&self, event: NotificationEvent);
}

/// In-memory recording publisher for tests.
#[derive(Default)]
pub struct RecordingNotificationPublisher {
    events: std::sync::Mutex<Vec<NotificationEvent>>,
}

impl RecordingNotificationPublisher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> Vec<NotificationEvent> {
        self.events.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }
}

#[async_trait]
impl NotificationRealtimePublisher for RecordingNotificationPublisher {
    async fn publish(&self, event: NotificationEvent) -> Result<(), ApplicationError> {
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}
