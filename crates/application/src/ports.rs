use crate::errors::ApplicationError;
use async_trait::async_trait;
use domain::entities::{MessageType, ReactionType, RecipientType};
use uuid::Uuid;

/// Outbound port used to request trust-score recalculation when a trust input changes.
#[async_trait]
pub trait TrustScoreRecalculationPort: Send + Sync {
    async fn request_recalculation(&self, party_id: Uuid) -> Result<(), ApplicationError>;
}

/// No-op implementation used until the real trust-score use case is wired in.
pub struct NoOpTrustScoreRecalculation;

#[async_trait]
impl TrustScoreRecalculationPort for NoOpTrustScoreRecalculation {
    async fn request_recalculation(&self, _party_id: Uuid) -> Result<(), ApplicationError> {
        Ok(())
    }
}

/// Concrete implementation that delegates to the `RecalculateTrustScore` use case.
pub struct TrustScoreRecalculationService {
    recalc: std::sync::Arc<crate::trust_scores::RecalculateTrustScore>,
}

impl TrustScoreRecalculationService {
    pub fn new(recalc: std::sync::Arc<crate::trust_scores::RecalculateTrustScore>) -> Self {
        Self { recalc }
    }
}

#[async_trait]
impl TrustScoreRecalculationPort for TrustScoreRecalculationService {
    async fn request_recalculation(&self, party_id: Uuid) -> Result<(), ApplicationError> {
        self.recalc.execute(party_id).await.map(|_| ())
    }
}

/// Events published to connected clients over the real-time delivery channel.
#[derive(Debug, Clone)]
pub enum MessageEvent {
    MessageNew {
        message_id: Uuid,
        conversation_id: Uuid,
        sender_user_id: Uuid,
        sender_party_id: Option<Uuid>,
        recipient_type: RecipientType,
        recipient_user_id: Option<Uuid>,
        recipient_party_id: Option<Uuid>,
        recipient_deal_id: Option<Uuid>,
        recipient_room_id: Option<Uuid>,
        message_type: MessageType,
        subject: Option<String>,
        content: String,
        reply_to_message_id: Option<Uuid>,
        created_at: time::OffsetDateTime,
    },
    MessageUpdated {
        message_id: Uuid,
        conversation_id: Uuid,
        content: String,
        edited_at: time::OffsetDateTime,
    },
    MessageDeleted {
        message_id: Uuid,
        conversation_id: Uuid,
    },
    MessageRead {
        message_id: Uuid,
        user_id: Uuid,
        party_id: Option<Uuid>,
        read_at: time::OffsetDateTime,
    },
    MessageReaction {
        message_id: Uuid,
        user_id: Uuid,
        party_id: Option<Uuid>,
        reaction_type: ReactionType,
        total_likes: i64,
        total_dislikes: i64,
    },
    RoomDeleted {
        room_id: Uuid,
    },
}

/// Outbound port used to publish real-time message events.
#[async_trait]
pub trait RealtimePublisher: Send + Sync {
    async fn publish(&self, event: MessageEvent) -> Result<(), ApplicationError>;
}

/// No-op publisher that silently discards all events.
pub struct NoOpRealtimePublisher;

#[async_trait]
impl RealtimePublisher for NoOpRealtimePublisher {
    async fn publish(&self, _event: MessageEvent) -> Result<(), ApplicationError> {
        Ok(())
    }
}

/// Outbound port used to encrypt and decrypt message bodies at the application layer.
#[async_trait]
pub trait EncryptionService: Send + Sync {
    async fn encrypt(&self, plaintext: &str) -> Result<String, ApplicationError>;
    async fn decrypt(&self, ciphertext: &str) -> Result<String, ApplicationError>;
}
