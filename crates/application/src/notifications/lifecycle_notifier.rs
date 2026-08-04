use crate::errors::ApplicationError;
use crate::notifications::dto::{RecipientSelector, SendNotificationCommand};
use crate::notifications::send_notification::SendNotification;
use domain::entities::NotificationType;
use std::sync::Arc;
use uuid::Uuid;

/// Convenience wrapper around [`SendNotification`] for emitting lifecycle notifications.
///
/// Callers typically invoke one of the typed helper methods and then ignore a failure:
/// notifications are best-effort and should not break the core business operation.
#[derive(Clone)]
pub struct LifecycleNotifier {
    inner: Arc<SendNotification>,
}

impl LifecycleNotifier {
    pub fn new(inner: Arc<SendNotification>) -> Self {
        Self { inner }
    }

    /// Notify all parties participating in a deal.
    pub async fn notify_deal_participants(
        &self,
        actor_user_id: Uuid,
        deal_id: Uuid,
        notification_type: NotificationType,
        metadata: serde_json::Value,
    ) -> Result<Vec<Uuid>, ApplicationError> {
        self.inner
            .execute(SendNotificationCommand {
                actor_user_id,
                actor_party_id: None,
                recipient: RecipientSelector::DealParticipants { deal_id },
                notification_type,
                priority: notification_type.default_priority(),
                title: None,
                body: None,
                action_url: Some(format!("/deals/{}", deal_id)),
                actions: vec![],
                related_entity_type: Some("deal".to_string()),
                related_entity_id: Some(deal_id),
                metadata,
                locale: "en".to_string(),
            })
            .await
    }

    /// Notify all members of a specific party.
    pub async fn notify_party_members(
        &self,
        actor_user_id: Uuid,
        party_id: Uuid,
        notification_type: NotificationType,
        related_entity_type: Option<&str>,
        related_entity_id: Option<Uuid>,
        metadata: serde_json::Value,
    ) -> Result<Vec<Uuid>, ApplicationError> {
        let action_url = related_entity_id.and_then(|id| {
            related_entity_type.map(|ty| match ty {
                "deal" => format!("/deals/{}", id),
                "verification" => format!("/verifications/{}", id),
                _ => format!("/{}/{}", ty, id),
            })
        });

        self.inner
            .execute(SendNotificationCommand {
                actor_user_id,
                actor_party_id: None,
                recipient: RecipientSelector::PartyMembers { party_id },
                notification_type,
                priority: notification_type.default_priority(),
                title: None,
                body: None,
                action_url,
                actions: vec![],
                related_entity_type: related_entity_type.map(|s| s.to_string()),
                related_entity_id,
                metadata,
                locale: "en".to_string(),
            })
            .await
    }

    /// Notify a single user.
    pub async fn notify_user(
        &self,
        actor_user_id: Uuid,
        user_id: Uuid,
        notification_type: NotificationType,
        related_entity_type: Option<&str>,
        related_entity_id: Option<Uuid>,
        metadata: serde_json::Value,
    ) -> Result<Vec<Uuid>, ApplicationError> {
        let action_url = related_entity_id.and_then(|id| {
            related_entity_type.map(|ty| match ty {
                "deal" => format!("/deals/{}", id),
                "verification" => format!("/verifications/{}", id),
                _ => format!("/{}/{}", ty, id),
            })
        });

        self.inner
            .execute(SendNotificationCommand {
                actor_user_id,
                actor_party_id: None,
                recipient: RecipientSelector::User { user_id },
                notification_type,
                priority: notification_type.default_priority(),
                title: None,
                body: None,
                action_url,
                actions: vec![],
                related_entity_type: related_entity_type.map(|s| s.to_string()),
                related_entity_id,
                metadata,
                locale: "en".to_string(),
            })
            .await
    }

    /// Best-effort wrapper that logs errors but never fails the caller's operation.
    pub fn fire_and_forget(
        &self,
        result: Result<Vec<Uuid>, ApplicationError>,
        context: &'static str,
    ) {
        if let Err(e) = result {
            tracing::warn!(error = %e, context, "lifecycle notification failed");
        }
    }
}
