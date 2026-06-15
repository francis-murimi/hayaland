use crate::chatrooms::access::has_scope;
use crate::chatrooms::dto::{ChatRoomMembershipResult, JoinChatRoomCommand};
use crate::errors::ApplicationError;
use domain::entities::{ChatRoomMemberRole, ChatRoomMembership, ChatRoomType};
use domain::repositories::{ChatRoomRepository, PartyRepository};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct JoinChatRoom {
    room_repo: Arc<dyn ChatRoomRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl JoinChatRoom {
    pub fn new(
        room_repo: Arc<dyn ChatRoomRepository>,
        party_repo: Arc<dyn PartyRepository>,
    ) -> Self {
        Self {
            room_repo,
            party_repo,
        }
    }

    pub async fn execute(
        &self,
        cmd: JoinChatRoomCommand,
    ) -> Result<ChatRoomMembershipResult, ApplicationError> {
        let room = self
            .room_repo
            .find_room_by_id(cmd.room_id)
            .await?
            .ok_or(ApplicationError::ChatRoomNotFound)?;

        if room.is_deleted {
            return Err(ApplicationError::ChatRoomNotFound);
        }

        if room.room_type == ChatRoomType::Private
            && !cmd.is_admin
            && !has_scope(&cmd.scopes, "chatrooms:moderate")
        {
            return Err(ApplicationError::Forbidden);
        }

        let membership = if let Some(party_id) = cmd.actor_party_id {
            if !self
                .party_repo
                .is_user_member_of_party(cmd.actor_user_id, party_id)
                .await?
            {
                return Err(ApplicationError::Forbidden);
            }
            if self
                .room_repo
                .find_membership_for_party(cmd.room_id, party_id)
                .await?
                .is_some()
            {
                return Err(ApplicationError::AlreadyChatRoomMember);
            }
            ChatRoomMembership::for_party(
                Uuid::now_v7(),
                cmd.room_id,
                party_id,
                ChatRoomMemberRole::Member,
            )
        } else {
            if self
                .room_repo
                .find_membership_for_user(cmd.room_id, cmd.actor_user_id)
                .await?
                .is_some()
            {
                return Err(ApplicationError::AlreadyChatRoomMember);
            }
            ChatRoomMembership::for_user(
                Uuid::now_v7(),
                cmd.room_id,
                cmd.actor_user_id,
                ChatRoomMemberRole::Member,
            )
        };

        self.room_repo.add_membership(&membership).await?;
        Ok(membership.into())
    }
}
