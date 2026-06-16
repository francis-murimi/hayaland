use crate::errors::DomainError;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// A channel over which a notification may be delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NotificationChannel {
    InApp,
    Email,
    Push,
    Sms,
    Webhook,
}

impl NotificationChannel {
    pub fn as_str(&self) -> &'static str {
        match self {
            NotificationChannel::InApp => "IN_APP",
            NotificationChannel::Email => "EMAIL",
            NotificationChannel::Push => "PUSH",
            NotificationChannel::Sms => "SMS",
            NotificationChannel::Webhook => "WEBHOOK",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            NotificationChannel::InApp,
            NotificationChannel::Email,
            NotificationChannel::Push,
            NotificationChannel::Sms,
            NotificationChannel::Webhook,
        ]
    }
}

impl TryFrom<&str> for NotificationChannel {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "IN_APP" => Ok(NotificationChannel::InApp),
            "EMAIL" => Ok(NotificationChannel::Email),
            "PUSH" => Ok(NotificationChannel::Push),
            "SMS" => Ok(NotificationChannel::Sms),
            "WEBHOOK" => Ok(NotificationChannel::Webhook),
            _ => Err(DomainError::InvalidNotificationChannel {
                message: format!("unknown notification channel: {value}"),
            }),
        }
    }
}

/// The category of a notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NotificationType {
    // Deal lifecycle
    DealInvite,
    DealSubmitted,
    DealTermsLocked,
    DealCommitted,
    DealExecuting,
    DealCompleted,
    DealCancelled,
    DealExpired,
    DealDisputed,
    // Negotiation
    TermProposed,
    TermAccepted,
    TermRejected,
    TermCountered,
    // Milestones
    MilestoneAssigned,
    MilestoneStarted,
    MilestoneCompleted,
    MilestoneVerified,
    MilestoneDue,
    // Payments
    EscrowFunded,
    EscrowReleased,
    PaymentDue,
    PaymentReceived,
    TransactionPendingApproval,
    TransactionApproved,
    TransactionRejected,
    // Reviews / trust / disputes / verifications
    ReviewRequested,
    ReviewReceived,
    TrustScoreUpdated,
    DisputeOpened,
    DisputeResolved,
    VerificationApproved,
    VerificationRejected,
    // Messaging
    MessageReceived,
    Mentioned,
    // Admin / system
    AdminBroadcast,
    SystemMaintenance,
    SecurityAlert,
    // Scoped custom admin message
    Custom,
}

impl NotificationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            NotificationType::DealInvite => "DEAL_INVITE",
            NotificationType::DealSubmitted => "DEAL_SUBMITTED",
            NotificationType::DealTermsLocked => "DEAL_TERMS_LOCKED",
            NotificationType::DealCommitted => "DEAL_COMMITTED",
            NotificationType::DealExecuting => "DEAL_EXECUTING",
            NotificationType::DealCompleted => "DEAL_COMPLETED",
            NotificationType::DealCancelled => "DEAL_CANCELLED",
            NotificationType::DealExpired => "DEAL_EXPIRED",
            NotificationType::DealDisputed => "DEAL_DISPUTED",
            NotificationType::TermProposed => "TERM_PROPOSED",
            NotificationType::TermAccepted => "TERM_ACCEPTED",
            NotificationType::TermRejected => "TERM_REJECTED",
            NotificationType::TermCountered => "TERM_COUNTERED",
            NotificationType::MilestoneAssigned => "MILESTONE_ASSIGNED",
            NotificationType::MilestoneStarted => "MILESTONE_STARTED",
            NotificationType::MilestoneCompleted => "MILESTONE_COMPLETED",
            NotificationType::MilestoneVerified => "MILESTONE_VERIFIED",
            NotificationType::MilestoneDue => "MILESTONE_DUE",
            NotificationType::EscrowFunded => "ESCROW_FUNDED",
            NotificationType::EscrowReleased => "ESCROW_RELEASED",
            NotificationType::PaymentDue => "PAYMENT_DUE",
            NotificationType::PaymentReceived => "PAYMENT_RECEIVED",
            NotificationType::TransactionPendingApproval => "TRANSACTION_PENDING_APPROVAL",
            NotificationType::TransactionApproved => "TRANSACTION_APPROVED",
            NotificationType::TransactionRejected => "TRANSACTION_REJECTED",
            NotificationType::ReviewRequested => "REVIEW_REQUESTED",
            NotificationType::ReviewReceived => "REVIEW_RECEIVED",
            NotificationType::TrustScoreUpdated => "TRUST_SCORE_UPDATED",
            NotificationType::DisputeOpened => "DISPUTE_OPENED",
            NotificationType::DisputeResolved => "DISPUTE_RESOLVED",
            NotificationType::VerificationApproved => "VERIFICATION_APPROVED",
            NotificationType::VerificationRejected => "VERIFICATION_REJECTED",
            NotificationType::MessageReceived => "MESSAGE_RECEIVED",
            NotificationType::Mentioned => "MENTIONED",
            NotificationType::AdminBroadcast => "ADMIN_BROADCAST",
            NotificationType::SystemMaintenance => "SYSTEM_MAINTENANCE",
            NotificationType::SecurityAlert => "SECURITY_ALERT",
            NotificationType::Custom => "CUSTOM",
        }
    }

    pub fn default_priority(&self) -> NotificationPriority {
        match self {
            NotificationType::DealDisputed
            | NotificationType::DisputeOpened
            | NotificationType::SecurityAlert => NotificationPriority::Critical,
            NotificationType::DealInvite
            | NotificationType::DealTermsLocked
            | NotificationType::DealCommitted
            | NotificationType::DealCompleted
            | NotificationType::EscrowReleased
            | NotificationType::PaymentDue
            | NotificationType::MilestoneDue
            | NotificationType::VerificationApproved
            | NotificationType::VerificationRejected => NotificationPriority::High,
            NotificationType::ReviewRequested
            | NotificationType::MessageReceived
            | NotificationType::Mentioned
            | NotificationType::AdminBroadcast
            | NotificationType::SystemMaintenance => NotificationPriority::Normal,
            _ => NotificationPriority::Normal,
        }
    }
}

impl TryFrom<&str> for NotificationType {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "DEAL_INVITE" => Ok(NotificationType::DealInvite),
            "DEAL_SUBMITTED" => Ok(NotificationType::DealSubmitted),
            "DEAL_TERMS_LOCKED" => Ok(NotificationType::DealTermsLocked),
            "DEAL_COMMITTED" => Ok(NotificationType::DealCommitted),
            "DEAL_EXECUTING" => Ok(NotificationType::DealExecuting),
            "DEAL_COMPLETED" => Ok(NotificationType::DealCompleted),
            "DEAL_CANCELLED" => Ok(NotificationType::DealCancelled),
            "DEAL_EXPIRED" => Ok(NotificationType::DealExpired),
            "DEAL_DISPUTED" => Ok(NotificationType::DealDisputed),
            "TERM_PROPOSED" => Ok(NotificationType::TermProposed),
            "TERM_ACCEPTED" => Ok(NotificationType::TermAccepted),
            "TERM_REJECTED" => Ok(NotificationType::TermRejected),
            "TERM_COUNTERED" => Ok(NotificationType::TermCountered),
            "MILESTONE_ASSIGNED" => Ok(NotificationType::MilestoneAssigned),
            "MILESTONE_STARTED" => Ok(NotificationType::MilestoneStarted),
            "MILESTONE_COMPLETED" => Ok(NotificationType::MilestoneCompleted),
            "MILESTONE_VERIFIED" => Ok(NotificationType::MilestoneVerified),
            "MILESTONE_DUE" => Ok(NotificationType::MilestoneDue),
            "ESCROW_FUNDED" => Ok(NotificationType::EscrowFunded),
            "ESCROW_RELEASED" => Ok(NotificationType::EscrowReleased),
            "PAYMENT_DUE" => Ok(NotificationType::PaymentDue),
            "PAYMENT_RECEIVED" => Ok(NotificationType::PaymentReceived),
            "TRANSACTION_PENDING_APPROVAL" => Ok(NotificationType::TransactionPendingApproval),
            "TRANSACTION_APPROVED" => Ok(NotificationType::TransactionApproved),
            "TRANSACTION_REJECTED" => Ok(NotificationType::TransactionRejected),
            "REVIEW_REQUESTED" => Ok(NotificationType::ReviewRequested),
            "REVIEW_RECEIVED" => Ok(NotificationType::ReviewReceived),
            "TRUST_SCORE_UPDATED" => Ok(NotificationType::TrustScoreUpdated),
            "DISPUTE_OPENED" => Ok(NotificationType::DisputeOpened),
            "DISPUTE_RESOLVED" => Ok(NotificationType::DisputeResolved),
            "VERIFICATION_APPROVED" => Ok(NotificationType::VerificationApproved),
            "VERIFICATION_REJECTED" => Ok(NotificationType::VerificationRejected),
            "MESSAGE_RECEIVED" => Ok(NotificationType::MessageReceived),
            "MENTIONED" => Ok(NotificationType::Mentioned),
            "ADMIN_BROADCAST" => Ok(NotificationType::AdminBroadcast),
            "SYSTEM_MAINTENANCE" => Ok(NotificationType::SystemMaintenance),
            "SECURITY_ALERT" => Ok(NotificationType::SecurityAlert),
            "CUSTOM" => Ok(NotificationType::Custom),
            _ => Err(DomainError::InvalidNotificationType {
                message: format!("unknown notification type: {value}"),
            }),
        }
    }
}

/// The urgency level of a notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Critical,
}

impl NotificationPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            NotificationPriority::Low => "LOW",
            NotificationPriority::Normal => "NORMAL",
            NotificationPriority::High => "HIGH",
            NotificationPriority::Critical => "CRITICAL",
        }
    }
}

impl TryFrom<&str> for NotificationPriority {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "LOW" => Ok(NotificationPriority::Low),
            "NORMAL" => Ok(NotificationPriority::Normal),
            "HIGH" => Ok(NotificationPriority::High),
            "CRITICAL" => Ok(NotificationPriority::Critical),
            _ => Err(DomainError::InvalidNotificationPriority {
                message: format!("unknown notification priority: {value}"),
            }),
        }
    }
}

/// The persisted status of a notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NotificationStatus {
    Pending,
    Sent,
    Delivered,
    Failed,
    Suppressed,
}

impl NotificationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            NotificationStatus::Pending => "PENDING",
            NotificationStatus::Sent => "SENT",
            NotificationStatus::Delivered => "DELIVERED",
            NotificationStatus::Failed => "FAILED",
            NotificationStatus::Suppressed => "SUPPRESSED",
        }
    }
}

impl TryFrom<&str> for NotificationStatus {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "PENDING" => Ok(NotificationStatus::Pending),
            "SENT" => Ok(NotificationStatus::Sent),
            "DELIVERED" => Ok(NotificationStatus::Delivered),
            "FAILED" => Ok(NotificationStatus::Failed),
            "SUPPRESSED" => Ok(NotificationStatus::Suppressed),
            _ => Err(DomainError::InvalidNotificationStatus {
                message: format!("unknown notification status: {value}"),
            }),
        }
    }
}

/// An actionable button/link attached to a notification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationAction {
    pub label: String,
    pub action_type: ActionType,
    pub url: Option<String>,
    pub method: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActionType {
    Navigate,
    ApiCall,
    Dismiss,
}

impl ActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionType::Navigate => "NAVIGATE",
            ActionType::ApiCall => "API_CALL",
            ActionType::Dismiss => "DISMISS",
        }
    }
}

impl TryFrom<&str> for ActionType {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "NAVIGATE" => Ok(ActionType::Navigate),
            "API_CALL" => Ok(ActionType::ApiCall),
            "DISMISS" => Ok(ActionType::Dismiss),
            _ => Err(DomainError::Validation(vec![format!(
                "unknown action type: {value}"
            )])),
        }
    }
}

/// A platform notification delivered to a user or party.
#[derive(Debug, Clone)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub party_id: Option<Uuid>,
    pub notification_type: NotificationType,
    pub title: String,
    pub body: String,
    pub channels: Vec<NotificationChannel>,
    pub priority: NotificationPriority,
    pub status: NotificationStatus,
    pub read_at: Option<OffsetDateTime>,
    pub actioned_at: Option<OffsetDateTime>,
    pub expires_at: Option<OffsetDateTime>,
    pub action_url: Option<String>,
    pub actions: Vec<NotificationAction>,
    pub related_entity_type: Option<String>,
    pub related_entity_id: Option<Uuid>,
    pub metadata: serde_json::Value,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl Notification {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Uuid,
        user_id: Option<Uuid>,
        party_id: Option<Uuid>,
        notification_type: NotificationType,
        title: String,
        body: String,
        priority: NotificationPriority,
        action_url: Option<String>,
        actions: Vec<NotificationAction>,
        related_entity_type: Option<String>,
        related_entity_id: Option<Uuid>,
        metadata: serde_json::Value,
        expires_at: Option<OffsetDateTime>,
    ) -> Result<Self, DomainError> {
        if user_id.is_none() && party_id.is_none() {
            return Err(DomainError::Validation(vec![
                "notification must have a user_id or party_id".to_string(),
            ]));
        }
        if title.trim().is_empty() {
            return Err(DomainError::Validation(vec![
                "notification title cannot be empty".to_string(),
            ]));
        }
        if body.trim().is_empty() {
            return Err(DomainError::Validation(vec![
                "notification body cannot be empty".to_string(),
            ]));
        }
        let now = OffsetDateTime::now_utc();
        Ok(Self {
            id,
            user_id,
            party_id,
            notification_type,
            title,
            body,
            channels: vec![NotificationChannel::InApp],
            priority,
            status: NotificationStatus::Pending,
            read_at: None,
            actioned_at: None,
            expires_at,
            action_url,
            actions,
            related_entity_type,
            related_entity_id,
            metadata,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn mark_read(&mut self, at: OffsetDateTime) {
        self.read_at = Some(at);
        self.updated_at = at;
        if self.status == NotificationStatus::Sent {
            self.status = NotificationStatus::Delivered;
        }
    }

    pub fn mark_actioned(&mut self, at: OffsetDateTime) {
        self.actioned_at = Some(at);
        self.updated_at = at;
    }

    pub fn is_expired(&self, now: OffsetDateTime) -> bool {
        self.expires_at.is_some_and(|exp| now >= exp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_requires_recipient() {
        let err = Notification::new(
            Uuid::now_v7(),
            None,
            None,
            NotificationType::SystemMaintenance,
            "Title".to_string(),
            "Body".to_string(),
            NotificationPriority::Normal,
            None,
            vec![],
            None,
            None,
            serde_json::Value::Null,
            None,
        )
        .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[test]
    fn notification_requires_non_empty_title() {
        let err = Notification::new(
            Uuid::now_v7(),
            Some(Uuid::now_v7()),
            None,
            NotificationType::SystemMaintenance,
            "   ".to_string(),
            "Body".to_string(),
            NotificationPriority::Normal,
            None,
            vec![],
            None,
            None,
            serde_json::Value::Null,
            None,
        )
        .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[test]
    fn mark_read_updates_status() {
        let mut n = Notification::new(
            Uuid::now_v7(),
            Some(Uuid::now_v7()),
            None,
            NotificationType::DealInvite,
            "Title".to_string(),
            "Body".to_string(),
            NotificationPriority::High,
            None,
            vec![],
            None,
            None,
            serde_json::Value::Null,
            None,
        )
        .unwrap();
        n.status = NotificationStatus::Sent;
        let now = OffsetDateTime::now_utc();
        n.mark_read(now);
        assert_eq!(n.status, NotificationStatus::Delivered);
        assert_eq!(n.read_at, Some(now));
    }

    #[test]
    fn channel_try_from_round_trip() {
        for ch in NotificationChannel::all() {
            let s = ch.as_str();
            assert_eq!(NotificationChannel::try_from(s).unwrap(), ch);
        }
    }

    #[test]
    fn type_try_from_unknown_fails() {
        assert!(NotificationType::try_from("UNKNOWN").is_err());
    }

    #[test]
    fn notification_creation_succeeds_with_user() {
        let n = Notification::new(
            Uuid::now_v7(),
            Some(Uuid::now_v7()),
            None,
            NotificationType::DealInvite,
            "Title".to_string(),
            "Body".to_string(),
            NotificationPriority::Normal,
            None,
            vec![],
            None,
            None,
            serde_json::Value::Null,
            None,
        )
        .unwrap();
        assert_eq!(n.status, NotificationStatus::Pending);
    }

    #[test]
    fn notification_creation_succeeds_with_party() {
        let n = Notification::new(
            Uuid::now_v7(),
            None,
            Some(Uuid::now_v7()),
            NotificationType::AdminBroadcast,
            "Title".to_string(),
            "Body".to_string(),
            NotificationPriority::Normal,
            None,
            vec![],
            None,
            None,
            serde_json::Value::Null,
            None,
        )
        .unwrap();
        assert!(n.user_id.is_none());
        assert!(n.party_id.is_some());
    }

    #[test]
    fn mark_actioned_updates_status() {
        let mut n = Notification::new(
            Uuid::now_v7(),
            Some(Uuid::now_v7()),
            None,
            NotificationType::DealInvite,
            "Title".to_string(),
            "Body".to_string(),
            NotificationPriority::Normal,
            None,
            vec![],
            None,
            None,
            serde_json::Value::Null,
            None,
        )
        .unwrap();
        let now = OffsetDateTime::now_utc();
        n.mark_actioned(now);
        assert_eq!(n.actioned_at, Some(now));
        assert_eq!(n.updated_at, now);
    }

    #[test]
    fn expiration_check_works() {
        let mut n = Notification::new(
            Uuid::now_v7(),
            Some(Uuid::now_v7()),
            None,
            NotificationType::DealInvite,
            "Title".to_string(),
            "Body".to_string(),
            NotificationPriority::Normal,
            None,
            vec![],
            None,
            None,
            serde_json::Value::Null,
            Some(OffsetDateTime::now_utc()),
        )
        .unwrap();
        assert!(n.is_expired(OffsetDateTime::now_utc()));
        n.expires_at = None;
        assert!(!n.is_expired(OffsetDateTime::now_utc()));
    }

    #[test]
    fn channel_unknown_fails() {
        assert!(NotificationChannel::try_from("UNKNOWN").is_err());
    }

    #[test]
    fn type_round_trip() {
        assert_eq!(
            NotificationType::try_from(NotificationType::SecurityAlert.as_str()).unwrap(),
            NotificationType::SecurityAlert
        );
    }

    #[test]
    fn all_notification_types_round_trip() {
        let types = [
            NotificationType::DealInvite,
            NotificationType::DealSubmitted,
            NotificationType::DealTermsLocked,
            NotificationType::DealCommitted,
            NotificationType::DealExecuting,
            NotificationType::DealCompleted,
            NotificationType::DealCancelled,
            NotificationType::DealExpired,
            NotificationType::DealDisputed,
            NotificationType::TermProposed,
            NotificationType::TermAccepted,
            NotificationType::TermRejected,
            NotificationType::TermCountered,
            NotificationType::MilestoneAssigned,
            NotificationType::MilestoneStarted,
            NotificationType::MilestoneCompleted,
            NotificationType::MilestoneVerified,
            NotificationType::MilestoneDue,
            NotificationType::EscrowFunded,
            NotificationType::EscrowReleased,
            NotificationType::PaymentDue,
            NotificationType::PaymentReceived,
            NotificationType::TransactionPendingApproval,
            NotificationType::TransactionApproved,
            NotificationType::TransactionRejected,
            NotificationType::ReviewRequested,
            NotificationType::ReviewReceived,
            NotificationType::TrustScoreUpdated,
            NotificationType::DisputeOpened,
            NotificationType::DisputeResolved,
            NotificationType::VerificationApproved,
            NotificationType::VerificationRejected,
            NotificationType::MessageReceived,
            NotificationType::Mentioned,
            NotificationType::AdminBroadcast,
            NotificationType::SystemMaintenance,
            NotificationType::SecurityAlert,
            NotificationType::Custom,
        ];
        for t in types {
            let s = t.as_str();
            assert_eq!(NotificationType::try_from(s).unwrap(), t);
        }
    }

    #[test]
    fn default_priority_for_critical_and_high() {
        assert_eq!(
            NotificationType::SecurityAlert.default_priority(),
            NotificationPriority::Critical
        );
        assert_eq!(
            NotificationType::DealInvite.default_priority(),
            NotificationPriority::High
        );
    }

    #[test]
    fn priority_round_trip() {
        for p in [
            NotificationPriority::Low,
            NotificationPriority::Normal,
            NotificationPriority::High,
            NotificationPriority::Critical,
        ] {
            assert_eq!(NotificationPriority::try_from(p.as_str()).unwrap(), p);
        }
    }

    #[test]
    fn status_round_trip() {
        for s in [
            NotificationStatus::Pending,
            NotificationStatus::Sent,
            NotificationStatus::Delivered,
            NotificationStatus::Failed,
            NotificationStatus::Suppressed,
        ] {
            assert_eq!(NotificationStatus::try_from(s.as_str()).unwrap(), s);
        }
    }

    #[test]
    fn action_type_round_trip() {
        for a in [
            ActionType::Navigate,
            ActionType::ApiCall,
            ActionType::Dismiss,
        ] {
            assert_eq!(ActionType::try_from(a.as_str()).unwrap(), a);
        }
        assert!(ActionType::try_from("UNKNOWN").is_err());
    }
}
