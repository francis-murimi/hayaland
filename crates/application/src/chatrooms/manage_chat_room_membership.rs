use crate::chatrooms::access::can_manage_chat_room;
use crate::chatrooms::dto::{ChatRoomMembershipResult, ManageMembershipCommand, MembershipAction};
use crate::errors::ApplicationError;
use domain::entities::{ChatRoomMemberRole, ChatRoomMembership};
use domain::repositories::ChatRoomRepository;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct ManageChatRoomMembership {
    room_repo: Arc<dyn ChatRoomRepository>,
}

impl ManageChatRoomMembership {
    pub fn new(room_repo: Arc<dyn ChatRoomRepository>) -> Self {
        Self { room_repo }
    }

    pub async fn execute(
        &self,
        cmd: ManageMembershipCommand,
    ) -> Result<Option<ChatRoomMembershipResult>, ApplicationError> {
        let room = self
            .room_repo
            .find_room_by_id(cmd.room_id)
            .await?
            .ok_or(ApplicationError::ChatRoomNotFound)?;

        if !can_manage_chat_room(
            &room,
            cmd.actor_user_id,
            cmd.is_admin,
            self.room_repo.as_ref(),
        )
        .await?
        {
            return Err(ApplicationError::CannotManageChatRoom);
        }

        match cmd.action {
            MembershipAction::Add => {
                let membership = if let Some(user_id) = cmd.target_user_id {
                    if self
                        .room_repo
                        .find_membership_for_user(cmd.room_id, user_id)
                        .await?
                        .is_some()
                    {
                        return Err(ApplicationError::AlreadyChatRoomMember);
                    }
                    ChatRoomMembership::for_user(
                        Uuid::now_v7(),
                        cmd.room_id,
                        user_id,
                        cmd.role.unwrap_or(ChatRoomMemberRole::Member),
                    )
                } else if let Some(party_id) = cmd.target_party_id {
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
                        cmd.role.unwrap_or(ChatRoomMemberRole::Member),
                    )
                } else {
                    return Err(ApplicationError::Validation(vec![
                        "target user or party is required".to_string(),
                    ]));
                };
                self.room_repo.add_membership(&membership).await?;
                Ok(Some(membership.into()))
            }
            MembershipAction::Remove => {
                let membership_id = cmd.membership_id.ok_or(ApplicationError::Validation(vec![
                    "membership_id is required".to_string(),
                ]))?;
                let membership = self
                    .room_repo
                    .find_membership_by_id(membership_id)
                    .await?
                    .ok_or(ApplicationError::ChatRoomMembershipNotFound)?;

                if membership.user_id == Some(room.created_by_user_id) && !cmd.is_admin {
                    return Err(ApplicationError::CannotManageChatRoom);
                }

                self.room_repo.remove_membership(membership_id).await?;
                Ok(None)
            }
            MembershipAction::SetRole => {
                let membership_id = cmd.membership_id.ok_or(ApplicationError::Validation(vec![
                    "membership_id is required".to_string(),
                ]))?;
                let role = cmd.role.ok_or(ApplicationError::Validation(vec![
                    "role is required".to_string(),
                ]))?;
                self.room_repo
                    .update_membership_role(membership_id, role)
                    .await?;
                Ok(None)
            }
        }
    }
}
