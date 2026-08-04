use application::errors::ApplicationError;
use application::ports::{MessageEvent, RealtimePublisher};
use async_trait::async_trait;
use std::sync::Mutex;

/// In-memory real-time publisher that records all published events.
///
/// This is intended for tests and local development where a real delivery
/// channel (e.g. WebSockets) is not wired up.
pub struct InMemoryRealtimePublisher {
    events: Mutex<Vec<MessageEvent>>,
}

impl Default for InMemoryRealtimePublisher {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryRealtimePublisher {
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
    }

    /// Return a snapshot of the events published so far.
    pub fn snapshot(&self) -> Vec<MessageEvent> {
        self.events.lock().unwrap().clone()
    }

    /// Clear all recorded events.
    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }
}

#[async_trait]
impl RealtimePublisher for InMemoryRealtimePublisher {
    async fn publish(&self, event: MessageEvent) -> Result<(), ApplicationError> {
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use application::ports::MessageEvent;
    use domain::entities::message::RecipientType;
    use domain::entities::MessageType;
    use time::OffsetDateTime;
    use uuid::Uuid;

    fn sample_event() -> MessageEvent {
        MessageEvent::MessageNew {
            message_id: Uuid::now_v7(),
            conversation_id: Uuid::now_v7(),
            sender_user_id: Uuid::now_v7(),
            sender_party_id: None,
            recipient_type: RecipientType::User,
            recipient_user_id: Some(Uuid::now_v7()),
            recipient_party_id: None,
            recipient_deal_id: None,
            recipient_room_id: None,
            message_type: MessageType::Text,
            subject: None,
            content: "hello".into(),
            reply_to_message_id: None,
            created_at: OffsetDateTime::now_utc(),
        }
    }

    #[tokio::test]
    async fn records_published_events() {
        let publisher = InMemoryRealtimePublisher::new();
        let event = sample_event();
        publisher.publish(event.clone()).await.unwrap();
        assert_eq!(publisher.snapshot().len(), 1);
    }

    #[tokio::test]
    async fn clear_removes_recorded_events() {
        let publisher = InMemoryRealtimePublisher::new();
        publisher.publish(sample_event()).await.unwrap();
        publisher.clear();
        assert!(publisher.snapshot().is_empty());
    }
}
