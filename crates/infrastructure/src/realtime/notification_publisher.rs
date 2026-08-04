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

#[cfg(test)]
mod tests {
    use super::*;
    use application::ports::NotificationEvent;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use uuid::Uuid;

    #[derive(Default)]
    struct CountingRegistry {
        count: AtomicUsize,
    }

    #[async_trait]
    impl NotificationRegistry for CountingRegistry {
        async fn notify(&self, _event: NotificationEvent) {
            self.count.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn websocket_publisher_delegates_to_registry() {
        let registry = CountingRegistry::default();
        let publisher = NotificationWebSocketPublisher::new(registry);
        let event = NotificationEvent::NotificationNew {
            notification_id: Uuid::now_v7(),
            user_id: Some(Uuid::now_v7()),
            party_id: None,
        };
        publisher.publish(event).await.unwrap();
        assert_eq!(publisher.registry.count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn recording_publisher_stores_events() {
        let publisher = RecordingNotificationPublisher::new();
        let event = NotificationEvent::UnreadCountChanged {
            user_id: Some(Uuid::now_v7()),
            party_id: None,
            count: 3,
        };
        publisher.publish(event.clone()).await.unwrap();
        assert_eq!(publisher.snapshot().len(), 1);
        publisher.clear();
        assert!(publisher.snapshot().is_empty());
    }
}
