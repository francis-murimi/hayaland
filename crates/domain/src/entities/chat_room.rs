use crate::errors::DomainError;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Visibility of a chatroom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChatRoomType {
    Public,
    Private,
}

impl ChatRoomType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChatRoomType::Public => "PUBLIC",
            ChatRoomType::Private => "PRIVATE",
        }
    }
}

impl TryFrom<&str> for ChatRoomType {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "PUBLIC" => Ok(ChatRoomType::Public),
            "PRIVATE" => Ok(ChatRoomType::Private),
            _ => Err(DomainError::InvalidChatRoomType {
                message: format!("unknown chat room type: {value}"),
            }),
        }
    }
}

/// A validated chatroom name (3–120 characters).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChatRoomName(String);

impl ChatRoomName {
    pub fn new(value: &str) -> Result<Self, DomainError> {
        let trimmed = value.trim();
        let len = trimmed.chars().count();
        if !(3..=120).contains(&len) {
            return Err(DomainError::InvalidChatRoomName {
                message: "chat room name must be between 3 and 120 characters".to_string(),
            });
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A platform-wide chatroom.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatRoom {
    pub id: Uuid,
    pub name: ChatRoomName,
    pub description: Option<String>,
    pub room_type: ChatRoomType,
    pub created_by_user_id: Uuid,
    pub is_deleted: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl ChatRoom {
    pub fn new(
        id: Uuid,
        name: ChatRoomName,
        description: Option<String>,
        room_type: ChatRoomType,
        created_by_user_id: Uuid,
    ) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id,
            name,
            description,
            room_type,
            created_by_user_id,
            is_deleted: false,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn soft_delete(&mut self) {
        self.is_deleted = true;
        self.updated_at = OffsetDateTime::now_utc();
    }

    pub fn update(&mut self, name: Option<ChatRoomName>, description: Option<String>) {
        if let Some(name) = name {
            self.name = name;
        }
        if description.is_some() {
            self.description = description;
        }
        self.updated_at = OffsetDateTime::now_utc();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_room_type_from_str() {
        assert_eq!(
            ChatRoomType::try_from("PUBLIC").unwrap(),
            ChatRoomType::Public
        );
        assert_eq!(
            ChatRoomType::try_from("PRIVATE").unwrap(),
            ChatRoomType::Private
        );
        assert!(ChatRoomType::try_from("UNKNOWN").is_err());
    }

    #[test]
    fn chat_room_name_rejects_too_short() {
        assert!(ChatRoomName::new("ab").is_err());
    }

    #[test]
    fn chat_room_name_rejects_too_long() {
        assert!(ChatRoomName::new(&"a".repeat(121)).is_err());
    }

    #[test]
    fn chat_room_name_accepts_valid() {
        let name = ChatRoomName::new("Agriculture Deals").unwrap();
        assert_eq!(name.as_str(), "Agriculture Deals");
    }

    #[test]
    fn new_room_is_active() {
        let room = ChatRoom::new(
            Uuid::now_v7(),
            ChatRoomName::new("General").unwrap(),
            None,
            ChatRoomType::Public,
            Uuid::now_v7(),
        );
        assert!(!room.is_deleted);
        assert_eq!(room.room_type, ChatRoomType::Public);
    }

    #[test]
    fn soft_delete_marks_deleted() {
        let mut room = ChatRoom::new(
            Uuid::now_v7(),
            ChatRoomName::new("General").unwrap(),
            None,
            ChatRoomType::Public,
            Uuid::now_v7(),
        );
        room.soft_delete();
        assert!(room.is_deleted);
    }
}
