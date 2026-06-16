use crate::errors::ApplicationError;
use domain::entities::{NotificationChannel, NotificationTemplate, NotificationType};
use domain::repositories::NotificationTemplateRepository;
use std::sync::Arc;

/// Render a notification for a specific channel, falling back to default locale
/// and then to a generated fallback message.
pub async fn render_notification(
    template_repo: Arc<dyn NotificationTemplateRepository>,
    notification_type: NotificationType,
    channel: NotificationChannel,
    locale: &str,
    variables: &serde_json::Value,
) -> Result<(String, String), ApplicationError> {
    // Try requested locale.
    if let Some(template) = template_repo
        .find_active(notification_type, channel, locale)
        .await?
    {
        return template.render(variables).map_err(ApplicationError::from);
    }

    // Fall back to English.
    if locale != "en" {
        if let Some(template) = template_repo
            .find_active(notification_type, channel, "en")
            .await?
        {
            return template.render(variables).map_err(ApplicationError::from);
        }
    }

    // Final fallback.
    Ok(fallback_render(notification_type, channel, variables))
}

/// Build a fallback title/body when no template exists.
fn fallback_render(
    notification_type: NotificationType,
    channel: NotificationChannel,
    variables: &serde_json::Value,
) -> (String, String) {
    let title = match notification_type {
        NotificationType::DealInvite => "New deal invitation".to_string(),
        NotificationType::DealTermsLocked => "Deal terms locked".to_string(),
        NotificationType::DealCommitted => "Deal committed".to_string(),
        NotificationType::DealCompleted => "Deal completed".to_string(),
        NotificationType::DealCancelled => "Deal cancelled".to_string(),
        NotificationType::DealDisputed => "Deal disputed".to_string(),
        NotificationType::MilestoneVerified => "Milestone verified".to_string(),
        NotificationType::EscrowReleased => "Payment released".to_string(),
        NotificationType::DisputeOpened => "Dispute opened".to_string(),
        NotificationType::DisputeResolved => "Dispute resolved".to_string(),
        NotificationType::AdminBroadcast => variables
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Announcement")
            .to_string(),
        _ => format!("{:?}", notification_type),
    };

    let body = match channel {
        NotificationChannel::Email => variables
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("You have a new notification.")
            .to_string(),
        _ => variables
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("You have a new notification.")
            .to_string(),
    };

    (title, body)
}

/// Render a template synchronously (used by tests and internal helpers).
pub fn render_template(
    template: &NotificationTemplate,
    variables: &serde_json::Value,
) -> Result<(String, String), ApplicationError> {
    template.render(variables).map_err(ApplicationError::from)
}
