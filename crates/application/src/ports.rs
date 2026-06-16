use crate::errors::ApplicationError;
use async_trait::async_trait;
use domain::entities::deal::DealStatus;
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

/// Events published to connected clients for notification center updates.
#[derive(Debug, Clone)]
pub enum NotificationEvent {
    NotificationNew {
        notification_id: Uuid,
        user_id: Option<Uuid>,
        party_id: Option<Uuid>,
    },
    NotificationRead {
        notification_id: Uuid,
        user_id: Uuid,
    },
    UnreadCountChanged {
        user_id: Option<Uuid>,
        party_id: Option<Uuid>,
        count: i64,
    },
}

/// Outbound port used to publish real-time notification events.
#[async_trait]
pub trait NotificationRealtimePublisher: Send + Sync {
    async fn publish(&self, event: NotificationEvent) -> Result<(), ApplicationError>;
}

/// No-op publisher that silently discards notification events.
pub struct NoOpNotificationRealtimePublisher;

#[async_trait]
impl NotificationRealtimePublisher for NoOpNotificationRealtimePublisher {
    async fn publish(&self, _event: NotificationEvent) -> Result<(), ApplicationError> {
        Ok(())
    }
}

/// Outbound port for sending mobile/web push notifications.
#[async_trait]
pub trait PushNotificationSender: Send + Sync {
    async fn send(
        &self,
        device_tokens: &[String],
        title: &str,
        body: &str,
        data: serde_json::Value,
    ) -> Result<Vec<PushResult>, ApplicationError>;
}

#[derive(Debug, Clone)]
pub struct PushResult {
    pub device_token: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Outbound port for sending SMS messages.
#[async_trait]
pub trait SmsSender: Send + Sync {
    async fn send(&self, phone: &str, body: &str) -> Result<(), ApplicationError>;
}

/// A lightweight domain event emitted after a business transaction succeeds.
#[derive(Debug, Clone)]
pub enum DomainEvent {
    DealCreated {
        deal_id: Uuid,
        actor_party_id: Uuid,
    },
    DealStateChanged {
        deal_id: Uuid,
        from: DealStatus,
        to: DealStatus,
    },
    TermProposed {
        deal_id: Uuid,
        term_id: Uuid,
        proposer_party_id: Uuid,
    },
    TermAccepted {
        deal_id: Uuid,
        term_id: Uuid,
    },
    TermRejected {
        deal_id: Uuid,
        term_id: Uuid,
    },
    TermCountered {
        deal_id: Uuid,
        term_id: Uuid,
    },
    MilestoneCompleted {
        deal_id: Uuid,
        milestone_id: Uuid,
    },
    MilestoneVerified {
        deal_id: Uuid,
        milestone_id: Uuid,
    },
    EscrowFunded {
        deal_id: Uuid,
        transaction_id: Uuid,
    },
    EscrowReleased {
        deal_id: Uuid,
        transaction_id: Uuid,
    },
    TransactionApproved {
        deal_id: Uuid,
        transaction_id: Uuid,
    },
    TransactionRejected {
        deal_id: Uuid,
        transaction_id: Uuid,
    },
    DisputeRaised {
        deal_id: Uuid,
        dispute_id: Uuid,
    },
    DisputeResolved {
        deal_id: Uuid,
        dispute_id: Uuid,
    },
    ReviewSubmitted {
        deal_id: Uuid,
        review_id: Uuid,
    },
    VerificationApproved {
        party_id: Uuid,
        verification_id: Uuid,
    },
    VerificationRejected {
        party_id: Uuid,
        verification_id: Uuid,
    },
    TrustScoreUpdated {
        party_id: Uuid,
    },
    MessageReceived {
        conversation_id: Uuid,
        message_id: Uuid,
        recipient_user_id: Option<Uuid>,
        recipient_party_id: Option<Uuid>,
    },
}

/// Outbound port for publishing domain events to interested consumers.
#[async_trait]
pub trait DomainEventPublisher: Send + Sync {
    async fn publish(&self, event: DomainEvent) -> Result<(), ApplicationError>;
}

/// No-op publisher used in tests or before an event bus is wired.
pub struct NoOpDomainEventPublisher;

#[async_trait]
impl DomainEventPublisher for NoOpDomainEventPublisher {
    async fn publish(&self, _event: DomainEvent) -> Result<(), ApplicationError> {
        Ok(())
    }
}
