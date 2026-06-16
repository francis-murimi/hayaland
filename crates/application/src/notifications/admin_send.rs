use crate::errors::ApplicationError;
use crate::notifications::dto::{AdminSendNotificationRequest, AdminSendNotificationResult};
use crate::notifications::send_notification::SendNotification;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct AdminSendNotification {
    send_notification: Arc<SendNotification>,
}

impl AdminSendNotification {
    pub fn new(send_notification: Arc<SendNotification>) -> Self {
        Self { send_notification }
    }

    pub async fn execute(
        &self,
        actor_user_id: Uuid,
        req: AdminSendNotificationRequest,
    ) -> Result<AdminSendNotificationResult, ApplicationError> {
        let ids = self
            .send_notification
            .send_admin_notification(actor_user_id, req)
            .await?;

        Ok(AdminSendNotificationResult {
            sent_count: ids.len(),
            notification_ids: ids,
        })
    }
}
