use crate::errors::DomainError;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// A reaction that can be attached to a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReactionType {
    Like,
    Dislike,
}

impl ReactionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReactionType::Like => "LIKE",
            ReactionType::Dislike => "DISLIKE",
        }
    }
}

impl TryFrom<&str> for ReactionType {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "LIKE" => Ok(ReactionType::Like),
            "DISLIKE" => Ok(ReactionType::Dislike),
            _ => Err(DomainError::InvalidReactionType {
                message: format!("unknown reaction type: {value}"),
            }),
        }
    }
}

/// A like/dislike reaction attached to a message by a user.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageReaction {
    pub id: Uuid,
    pub message_id: Uuid,
    pub user_id: Uuid,
    pub party_id: Option<Uuid>,
    pub reaction_type: ReactionType,
    pub created_at: OffsetDateTime,
}

impl MessageReaction {
    pub fn new(
        id: Uuid,
        message_id: Uuid,
        user_id: Uuid,
        party_id: Option<Uuid>,
        reaction_type: ReactionType,
    ) -> Self {
        Self {
            id,
            message_id,
            user_id,
            party_id,
            reaction_type,
            created_at: OffsetDateTime::now_utc(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reaction_type_from_str() {
        assert_eq!(ReactionType::try_from("LIKE").unwrap(), ReactionType::Like);
        assert_eq!(
            ReactionType::try_from("DISLIKE").unwrap(),
            ReactionType::Dislike
        );
        assert!(ReactionType::try_from("UNKNOWN").is_err());
    }

    #[test]
    fn new_reaction_has_correct_fields() {
        let reaction = MessageReaction::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            None,
            ReactionType::Like,
        );
        assert_eq!(reaction.reaction_type, ReactionType::Like);
        assert!(reaction.party_id.is_none());
    }
}
