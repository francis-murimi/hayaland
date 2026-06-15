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
