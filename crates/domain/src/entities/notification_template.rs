use crate::entities::{NotificationChannel, NotificationType};
use crate::errors::DomainError;
use time::OffsetDateTime;
use uuid::Uuid;

/// A reusable template for rendering a notification on a specific channel.
#[derive(Debug, Clone)]
pub struct NotificationTemplate {
    pub id: Uuid,
    pub name: String,
    pub notification_type: NotificationType,
    pub channel: NotificationChannel,
    pub locale: String,
    pub subject_template: String,
    pub body_template: String,
    pub variables_schema: serde_json::Value,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl NotificationTemplate {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Uuid,
        name: String,
        notification_type: NotificationType,
        channel: NotificationChannel,
        locale: String,
        subject_template: String,
        body_template: String,
        variables_schema: serde_json::Value,
    ) -> Result<Self, DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::Validation(vec![
                "template name cannot be empty".to_string(),
            ]));
        }
        if body_template.trim().is_empty() {
            return Err(DomainError::Validation(vec![
                "template body cannot be empty".to_string(),
            ]));
        }
        if channel == NotificationChannel::Email && subject_template.trim().is_empty() {
            return Err(DomainError::Validation(vec![
                "email template subject cannot be empty".to_string(),
            ]));
        }
        let now = OffsetDateTime::now_utc();
        Ok(Self {
            id,
            name,
            notification_type,
            channel,
            locale,
            subject_template,
            body_template,
            variables_schema,
            is_active: true,
            created_at: now,
            updated_at: now,
        })
    }

    /// Perform a simple variable substitution for MVP templates.
    /// Variables are written as `{{variable_name}}`.
    pub fn render(&self, variables: &serde_json::Value) -> Result<(String, String), DomainError> {
        let mut subject = self.subject_template.clone();
        let mut body = self.body_template.clone();
        if let Some(map) = variables.as_object() {
            for (key, value) in map {
                let placeholder = format!("{{{{{}}}}}", key);
                let rendered = match value {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                subject = subject.replace(&placeholder, &rendered);
                body = body.replace(&placeholder, &rendered);
            }
        }
        Ok((subject, body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_renders_variables() {
        let template = NotificationTemplate::new(
            Uuid::now_v7(),
            "deal_invite_email".to_string(),
            NotificationType::DealInvite,
            NotificationChannel::Email,
            "en".to_string(),
            "Invitation to {{deal_name}}".to_string(),
            "Hi {{recipient_name}}, you were invited to {{deal_name}}.".to_string(),
            serde_json::Value::Null,
        )
        .unwrap();

        let vars = serde_json::json!({
            "deal_name": "Cotton Partnership",
            "recipient_name": "Alice",
        });
        let (subject, body) = template.render(&vars).unwrap();
        assert_eq!(subject, "Invitation to Cotton Partnership");
        assert_eq!(body, "Hi Alice, you were invited to Cotton Partnership.");
    }

    #[test]
    fn email_template_requires_subject() {
        let err = NotificationTemplate::new(
            Uuid::now_v7(),
            "name".to_string(),
            NotificationType::DealInvite,
            NotificationChannel::Email,
            "en".to_string(),
            "   ".to_string(),
            "Body".to_string(),
            serde_json::Value::Null,
        )
        .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[test]
    fn non_email_template_allows_empty_subject() {
        let template = NotificationTemplate::new(
            Uuid::now_v7(),
            "in_app_template".to_string(),
            NotificationType::DealInvite,
            NotificationChannel::InApp,
            "en".to_string(),
            "".to_string(),
            "Body".to_string(),
            serde_json::Value::Null,
        )
        .unwrap();
        assert!(template.subject_template.is_empty());
    }

    #[test]
    fn template_requires_name() {
        let err = NotificationTemplate::new(
            Uuid::now_v7(),
            "   ".to_string(),
            NotificationType::DealInvite,
            NotificationChannel::InApp,
            "en".to_string(),
            "".to_string(),
            "Body".to_string(),
            serde_json::Value::Null,
        )
        .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[test]
    fn template_requires_body() {
        let err = NotificationTemplate::new(
            Uuid::now_v7(),
            "name".to_string(),
            NotificationType::DealInvite,
            NotificationChannel::InApp,
            "en".to_string(),
            "".to_string(),
            "   ".to_string(),
            serde_json::Value::Null,
        )
        .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[test]
    fn render_without_variables_returns_template_unchanged() {
        let template = NotificationTemplate::new(
            Uuid::now_v7(),
            "name".to_string(),
            NotificationType::DealInvite,
            NotificationChannel::InApp,
            "en".to_string(),
            "".to_string(),
            "Static body".to_string(),
            serde_json::Value::Null,
        )
        .unwrap();
        let (subject, body) = template.render(&serde_json::Value::Null).unwrap();
        assert!(subject.is_empty());
        assert_eq!(body, "Static body");
    }
}
