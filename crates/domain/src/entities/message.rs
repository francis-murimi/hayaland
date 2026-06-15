use crate::errors::DomainError;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// The type of recipient context for a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecipientType {
    User,
    Party,
    PartyMembers,
    Deal,
    Room,
    AdminBroadcast,
}

impl RecipientType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RecipientType::User => "USER",
            RecipientType::Party => "PARTY",
            RecipientType::PartyMembers => "PARTY_MEMBERS",
            RecipientType::Deal => "DEAL",
            RecipientType::Room => "ROOM",
            RecipientType::AdminBroadcast => "ADMIN_BROADCAST",
        }
    }
}

impl TryFrom<&str> for RecipientType {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "USER" => Ok(RecipientType::User),
            "PARTY" => Ok(RecipientType::Party),
            "PARTY_MEMBERS" => Ok(RecipientType::PartyMembers),
            "DEAL" => Ok(RecipientType::Deal),
            "ROOM" => Ok(RecipientType::Room),
            "ADMIN_BROADCAST" => Ok(RecipientType::AdminBroadcast),
            _ => Err(DomainError::InvalidRecipient {
                message: format!("unknown recipient type: {value}"),
            }),
        }
    }
}

/// The type of message payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MessageType {
    Text,
    File,
    System,
    AdminBroadcast,
}

impl MessageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageType::Text => "TEXT",
            MessageType::File => "FILE",
            MessageType::System => "SYSTEM",
            MessageType::AdminBroadcast => "ADMIN_BROADCAST",
        }
    }
}

impl TryFrom<&str> for MessageType {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "TEXT" => Ok(MessageType::Text),
            "FILE" => Ok(MessageType::File),
            "SYSTEM" => Ok(MessageType::System),
            "ADMIN_BROADCAST" => Ok(MessageType::AdminBroadcast),
            _ => Err(DomainError::InvalidMessageType {
                message: format!("unknown message type: {value}"),
            }),
        }
    }
}

/// An encrypted message record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender_user_id: Uuid,
    pub sender_party_id: Option<Uuid>,
    pub recipient_type: RecipientType,
    pub recipient_user_id: Option<Uuid>,
    pub recipient_party_id: Option<Uuid>,
    pub recipient_deal_id: Option<Uuid>,
    pub recipient_room_id: Option<Uuid>,
    pub message_type: MessageType,
    pub subject: Option<String>,
    pub content: String,
    pub content_encryption_key_id: Option<Uuid>,
    pub attachment_urls: Vec<String>,
    pub reply_to_message_id: Option<Uuid>,
    pub is_pinned: bool,
    pub pinned_at: Option<OffsetDateTime>,
    pub is_deleted: bool,
    pub edited_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

impl Message {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Uuid,
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
        attachment_urls: Vec<String>,
        reply_to_message_id: Option<Uuid>,
    ) -> Result<Self, DomainError> {
        validate_recipient(
            &recipient_type,
            sender_party_id,
            recipient_user_id,
            recipient_party_id,
            recipient_deal_id,
            recipient_room_id,
        )?;
        Ok(Self {
            id,
            conversation_id,
            sender_user_id,
            sender_party_id,
            recipient_type,
            recipient_user_id,
            recipient_party_id,
            recipient_deal_id,
            recipient_room_id,
            message_type,
            subject,
            content,
            content_encryption_key_id: None,
            attachment_urls,
            reply_to_message_id,
            is_pinned: false,
            pinned_at: None,
            is_deleted: false,
            edited_at: None,
            created_at: OffsetDateTime::now_utc(),
        })
    }

    pub fn can_be_edited_by(&self, user_id: Uuid, is_admin: bool) -> bool {
        !self.is_deleted && (self.sender_user_id == user_id || is_admin)
    }

    pub fn can_be_deleted_by(&self, user_id: Uuid, is_admin: bool) -> bool {
        self.sender_user_id == user_id || is_admin
    }

    pub fn edit(&mut self, new_content: String) {
        self.content = new_content;
        self.edited_at = Some(OffsetDateTime::now_utc());
    }

    pub fn soft_delete(&mut self, placeholder_content: String) {
        self.is_deleted = true;
        self.content = placeholder_content;
        self.edited_at = None;
    }

    pub fn pin(&mut self) {
        self.is_pinned = true;
        self.pinned_at = Some(OffsetDateTime::now_utc());
    }

    pub fn unpin(&mut self) {
        self.is_pinned = false;
        self.pinned_at = None;
    }
}

pub fn validate_recipient(
    recipient_type: &RecipientType,
    sender_party_id: Option<Uuid>,
    recipient_user_id: Option<Uuid>,
    recipient_party_id: Option<Uuid>,
    recipient_deal_id: Option<Uuid>,
    recipient_room_id: Option<Uuid>,
) -> Result<(), DomainError> {
    let valid = match recipient_type {
        RecipientType::User => recipient_user_id.is_some(),
        RecipientType::Party => recipient_party_id.is_some(),
        RecipientType::PartyMembers => sender_party_id.is_some(),
        RecipientType::Deal => recipient_deal_id.is_some(),
        RecipientType::Room => recipient_room_id.is_some(),
        RecipientType::AdminBroadcast => true,
    };
    if valid {
        Ok(())
    } else {
        Err(DomainError::InvalidRecipient {
            message: format!("missing recipient id for {:?}", recipient_type),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipient_type_from_str() {
        assert_eq!(
            RecipientType::try_from("USER").unwrap(),
            RecipientType::User
        );
        assert_eq!(
            RecipientType::try_from("PARTY_MEMBERS").unwrap(),
            RecipientType::PartyMembers
        );
        assert!(RecipientType::try_from("UNKNOWN").is_err());
    }

    #[test]
    fn message_type_from_str() {
        assert_eq!(MessageType::try_from("TEXT").unwrap(), MessageType::Text);
        assert!(MessageType::try_from("UNKNOWN").is_err());
    }

    #[test]
    fn message_requires_recipient_id() {
        let conv = Uuid::now_v7();
        let sender = Uuid::now_v7();
        let result = Message::new(
            Uuid::now_v7(),
            conv,
            sender,
            None,
            RecipientType::User,
            None,
            None,
            None,
            None,
            MessageType::Text,
            None,
            "hello".to_string(),
            vec![],
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn party_members_requires_sender_party() {
        let conv = Uuid::now_v7();
        let sender = Uuid::now_v7();
        let result = Message::new(
            Uuid::now_v7(),
            conv,
            sender,
            None,
            RecipientType::PartyMembers,
            None,
            None,
            None,
            None,
            MessageType::Text,
            None,
            "hello".to_string(),
            vec![],
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn valid_direct_user_message() {
        let conv = Uuid::now_v7();
        let sender = Uuid::now_v7();
        let recipient = Uuid::now_v7();
        let msg = Message::new(
            Uuid::now_v7(),
            conv,
            sender,
            None,
            RecipientType::User,
            Some(recipient),
            None,
            None,
            None,
            MessageType::Text,
            None,
            "hello".to_string(),
            vec![],
            None,
        )
        .unwrap();
        assert_eq!(msg.recipient_user_id, Some(recipient));
        assert!(!msg.is_deleted);
    }

    #[test]
    fn only_sender_or_admin_can_edit() {
        let mut msg = Message::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            None,
            RecipientType::User,
            Some(Uuid::now_v7()),
            None,
            None,
            None,
            MessageType::Text,
            None,
            "hello".to_string(),
            vec![],
            None,
        )
        .unwrap();
        assert!(msg.can_be_edited_by(msg.sender_user_id, false));
        assert!(!msg.can_be_edited_by(Uuid::now_v7(), false));
        assert!(msg.can_be_edited_by(Uuid::now_v7(), true));
        msg.soft_delete("deleted".to_string());
        assert!(!msg.can_be_edited_by(msg.sender_user_id, true));
    }

    #[test]
    fn soft_delete_clears_content() {
        let mut msg = Message::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            None,
            RecipientType::User,
            Some(Uuid::now_v7()),
            None,
            None,
            None,
            MessageType::Text,
            None,
            "secret".to_string(),
            vec![],
            None,
        )
        .unwrap();
        msg.soft_delete("deleted".to_string());
        assert!(msg.is_deleted);
        assert_eq!(msg.content, "deleted");
    }

    #[test]
    fn pin_sets_pinned_at() {
        let mut msg = Message::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            None,
            RecipientType::User,
            Some(Uuid::now_v7()),
            None,
            None,
            None,
            MessageType::Text,
            None,
            "hello".to_string(),
            vec![],
            None,
        )
        .unwrap();
        assert!(!msg.is_pinned);
        msg.pin();
        assert!(msg.is_pinned);
        assert!(msg.pinned_at.is_some());
        msg.unpin();
        assert!(!msg.is_pinned);
        assert!(msg.pinned_at.is_none());
    }
}
