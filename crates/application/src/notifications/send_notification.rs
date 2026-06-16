use crate::email::queue::{EmailQueue, EmailQueueItem};
use crate::errors::ApplicationError;
use crate::notifications::dto::{
    AdminSendNotificationRequest, RecipientSelector, SendNotificationCommand,
};
use crate::notifications::render::render_notification;
use crate::notifications::route::route_channels;
use crate::ports::{
    NotificationEvent, NotificationRealtimePublisher, PushNotificationSender, SmsSender,
};
use domain::entities::{
    notification_preference::NotificationPreference, Notification, NotificationChannel,
    NotificationPriority, NotificationStatus, NotificationType,
};
use domain::repositories::{
    DealRepository, NotificationPreferenceRepository, NotificationRepository,
    NotificationTemplateRepository, PartyRepository, UserRepository,
};
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

/// Internal use case that creates and dispatches notifications.
#[derive(Clone)]
pub struct SendNotification {
    notification_repo: Arc<dyn NotificationRepository>,
    preference_repo: Arc<dyn NotificationPreferenceRepository>,
    template_repo: Arc<dyn NotificationTemplateRepository>,
    user_repo: Arc<dyn UserRepository>,
    party_repo: Arc<dyn PartyRepository>,
    deal_repo: Arc<dyn DealRepository>,
    email_queue: Arc<dyn EmailQueue>,
    realtime_publisher: Arc<dyn NotificationRealtimePublisher>,
    _push_sender: Arc<dyn PushNotificationSender>,
    _sms_sender: Arc<dyn SmsSender>,
    default_locale: String,
}

impl SendNotification {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        notification_repo: Arc<dyn NotificationRepository>,
        preference_repo: Arc<dyn NotificationPreferenceRepository>,
        template_repo: Arc<dyn NotificationTemplateRepository>,
        user_repo: Arc<dyn UserRepository>,
        party_repo: Arc<dyn PartyRepository>,
        deal_repo: Arc<dyn DealRepository>,
        email_queue: Arc<dyn EmailQueue>,
        realtime_publisher: Arc<dyn NotificationRealtimePublisher>,
        _push_sender: Arc<dyn PushNotificationSender>,
        _sms_sender: Arc<dyn SmsSender>,
        default_locale: String,
    ) -> Self {
        Self {
            notification_repo,
            preference_repo,
            template_repo,
            user_repo,
            party_repo,
            deal_repo,
            email_queue,
            realtime_publisher,
            _push_sender,
            _sms_sender,
            default_locale,
        }
    }

    /// Resolve recipients, render, and persist one notification per recipient.
    pub async fn execute(
        &self,
        cmd: SendNotificationCommand,
    ) -> Result<Vec<Uuid>, ApplicationError> {
        let recipients = self.resolve_recipients(&cmd.recipient).await?;
        let priority = cmd.priority;
        let notification_type = cmd.notification_type;
        let now = OffsetDateTime::now_utc();

        let mut ids = Vec::with_capacity(recipients.len());

        for recipient in recipients {
            let (user_id, party_id) = recipient;
            let locale = self.resolve_locale(user_id).await;
            let variables = cmd.metadata.clone();

            let prefs = if let Some(uid) = user_id {
                self.preference_repo.get(uid).await?
            } else {
                // Party-level notifications default to in-app only if no preferences exist.
                NotificationPreference::new(Uuid::nil())
            };

            let channels = route_channels(notification_type, priority, &prefs, now);

            if channels.is_empty() {
                // Still persist as suppressed so the user can see it was attempted.
                let (title, body) = self
                    .render_for_channel(
                        notification_type,
                        NotificationChannel::InApp,
                        &locale,
                        &variables,
                    )
                    .await?;
                let id = self
                    .persist_notification(
                        user_id,
                        party_id,
                        notification_type,
                        priority,
                        title,
                        body,
                        vec![NotificationChannel::InApp],
                        NotificationStatus::Suppressed,
                        &cmd,
                    )
                    .await?;
                ids.push(id);
                continue;
            }

            let mut title: Option<String> = None;
            let mut body: Option<String> = None;
            for channel in &channels {
                if title.is_none() || *channel == NotificationChannel::Email {
                    let (t, b) = self
                        .render_for_channel(notification_type, *channel, &locale, &variables)
                        .await?;
                    title = Some(t);
                    if *channel == NotificationChannel::Email || body.is_none() {
                        body = Some(b);
                    }
                }
            }

            let title = title.unwrap_or_else(|| format!("{:?}", notification_type));
            let body = body.unwrap_or_else(|| "You have a new notification.".to_string());

            let email_subject = title.clone();
            let email_body = body.clone();

            let id = self
                .persist_notification(
                    user_id,
                    party_id,
                    notification_type,
                    priority,
                    title,
                    body,
                    channels.clone(),
                    NotificationStatus::Pending,
                    &cmd,
                )
                .await?;

            // Synchronous dispatch for in-app; async worker handles email/push/sms.
            if channels.contains(&NotificationChannel::InApp) {
                self.realtime_publisher
                    .publish(NotificationEvent::NotificationNew {
                        notification_id: id,
                        user_id,
                        party_id,
                    })
                    .await?;
            }

            if channels.contains(&NotificationChannel::Email) {
                if let Err(e) = self
                    .email_queue
                    .enqueue(EmailQueueItem {
                        to: self
                            .resolve_email(user_id, party_id)
                            .await
                            .unwrap_or_default(),
                        subject: email_subject,
                        body: email_body,
                    })
                    .await
                {
                    tracing::error!(error = %e, notification_id = %id, "failed to enqueue notification email");
                }
            }

            // Push and SMS are best-effort; worker will retry.
            // The notification worker polls PENDING rows and handles push/sms.

            ids.push(id);
        }

        Ok(ids)
    }

    /// Convenience overload for admin requests.
    pub async fn send_admin_notification(
        &self,
        actor_user_id: Uuid,
        req: AdminSendNotificationRequest,
    ) -> Result<Vec<Uuid>, ApplicationError> {
        self.execute(SendNotificationCommand {
            actor_user_id,
            actor_party_id: None,
            recipient: req.target,
            notification_type: req.notification_type,
            priority: req.priority,
            title: req.title,
            body: req.body,
            action_url: req.action_url,
            actions: req.actions,
            related_entity_type: req.related_entity_type,
            related_entity_id: req.related_entity_id,
            metadata: req.metadata,
            locale: req.locale,
        })
        .await
    }

    async fn resolve_recipients(
        &self,
        selector: &RecipientSelector,
    ) -> Result<Vec<(Option<Uuid>, Option<Uuid>)>, ApplicationError> {
        match selector {
            RecipientSelector::User { user_id } => Ok(vec![(Some(*user_id), None)]),
            RecipientSelector::Party { party_id } => Ok(vec![(None, Some(*party_id))]),
            RecipientSelector::PartyMembers { party_id } => {
                let members = self.party_repo.list_members_for_party(*party_id).await?;
                Ok(members
                    .into_iter()
                    .map(|m| (Some(m.user_id), None))
                    .collect())
            }
            RecipientSelector::DealParticipants { deal_id } => {
                let aggregate = self
                    .deal_repo
                    .find_aggregate_by_id(*deal_id)
                    .await?
                    .ok_or(ApplicationError::DealNotFound)?;
                Ok(aggregate
                    .participations
                    .into_iter()
                    .map(|p| (None, Some(p.party_id)))
                    .collect())
            }
            RecipientSelector::AllUsers => {
                let users = self.user_repo.list(i64::MAX, 0, Some(true)).await?;
                Ok(users.into_iter().map(|u| (Some(u.id), None)).collect())
            }
            RecipientSelector::AllParties => {
                let criteria = domain::repositories::PartySearchCriteria {
                    active_only: Some(true),
                    limit: i64::MAX,
                    offset: 0,
                    ..Default::default()
                };
                let parties = self.party_repo.list(&criteria).await?;
                Ok(parties.into_iter().map(|p| (None, Some(p.id))).collect())
            }
        }
    }

    async fn resolve_locale(&self, user_id: Option<Uuid>) -> String {
        match user_id {
            Some(uid) => self
                .preference_repo
                .get(uid)
                .await
                .map(|p| p.quiet_hours.timezone.clone())
                .unwrap_or_else(|_| self.default_locale.clone()),
            None => self.default_locale.clone(),
        }
    }

    async fn resolve_email(&self, user_id: Option<Uuid>, party_id: Option<Uuid>) -> Option<String> {
        if let Some(uid) = user_id {
            self.user_repo
                .find_by_id(uid)
                .await
                .ok()
                .flatten()
                .map(|u| u.email.as_str().to_string())
        } else if let Some(pid) = party_id {
            self.party_repo
                .find_by_id(pid)
                .await
                .ok()
                .flatten()
                .map(|p| p.email.as_str().to_string())
        } else {
            None
        }
    }

    async fn render_for_channel(
        &self,
        notification_type: NotificationType,
        channel: NotificationChannel,
        locale: &str,
        variables: &serde_json::Value,
    ) -> Result<(String, String), ApplicationError> {
        render_notification(
            self.template_repo.clone(),
            notification_type,
            channel,
            locale,
            variables,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn persist_notification(
        &self,
        user_id: Option<Uuid>,
        party_id: Option<Uuid>,
        notification_type: NotificationType,
        priority: NotificationPriority,
        title: String,
        body: String,
        channels: Vec<NotificationChannel>,
        status: NotificationStatus,
        cmd: &SendNotificationCommand,
    ) -> Result<Uuid, ApplicationError> {
        let mut notification = Notification::new(
            Uuid::now_v7(),
            user_id,
            party_id,
            notification_type,
            title,
            body,
            priority,
            cmd.action_url.clone(),
            cmd.actions.iter().map(|a| a.clone().into()).collect(),
            cmd.related_entity_type.clone(),
            cmd.related_entity_id,
            cmd.metadata.clone(),
            None,
        )
        .map_err(ApplicationError::from)?;

        notification.channels = channels;
        notification.status = status;

        self.notification_repo.create(&notification).await?;
        Ok(notification.id)
    }
}
