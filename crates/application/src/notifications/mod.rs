pub mod admin_send;
pub mod admin_templates;
pub mod delete_notification;
pub mod dto;
pub mod get_notification;
pub mod get_preferences;
pub mod get_unread_count;
pub mod list_notifications;
pub mod mark_all_read;
pub mod mark_read;
pub mod render;
pub mod route;
pub mod send_notification;
pub mod update_preferences;

pub use admin_send::AdminSendNotification;
pub use admin_templates::{
    AdminCreateTemplate, AdminDeleteTemplate, AdminGetTemplate, AdminListTemplates,
    AdminUpdateTemplate,
};
pub use delete_notification::DeleteNotification;
pub use get_notification::GetNotification;
pub use get_preferences::GetNotificationPreferences;
pub use get_unread_count::GetUnreadCount;
pub use list_notifications::ListNotifications;
pub use mark_all_read::MarkAllNotificationsRead;
pub use mark_read::MarkNotificationRead;
pub use send_notification::SendNotification;
pub use update_preferences::UpdateNotificationPreferences;

#[cfg(test)]
mod tests;
