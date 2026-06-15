use crate::chatrooms::access::can_create_chat_room;
use crate::chatrooms::dto::{ChatRoomResult, CreateChatRoomCommand};
use crate::errors::ApplicationError;
use domain::entities::{
    ChatRoom, ChatRoomMemberRole, ChatRoomMembership, ChatRoomName, Conversation,
};
use domain::repositories::{ChatRoomRepository, MessageRepository};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct CreateChatRoom {
    room_repo: Arc<dyn ChatRoomRepository>,
    message_repo: Arc<dyn MessageRepository>,
}

impl CreateChatRoom {
    pub fn new(
        room_repo: Arc<dyn ChatRoomRepository>,
        message_repo: Arc<dyn MessageRepository>,
    ) -> Self {
        Self {
            room_repo,
            message_repo,
        }
    }

    pub async fn execute(
        &self,
        cmd: CreateChatRoomCommand,
    ) -> Result<ChatRoomResult, ApplicationError> {
        if !can_create_chat_room(&cmd.scopes) {
            return Err(ApplicationError::Forbidden);
        }

        let name = ChatRoomName::new(&cmd.name).map_err(ApplicationError::from)?;
        if self
            .room_repo
            .find_room_by_name(name.as_str())
            .await?
            .is_some()
        {
            return Err(ApplicationError::ChatRoomAlreadyExists);
        }

        let room = ChatRoom::new(
            Uuid::now_v7(),
            name,
            cmd.description,
            cmd.room_type,
            cmd.actor_user_id,
        );
        self.room_repo.create_room(&room).await?;

        let conversation = Conversation::new_room(room.id, room.id, Some(cmd.name));
        self.message_repo.create_conversation(&conversation).await?;

        let membership = ChatRoomMembership::for_user(
            Uuid::now_v7(),
            room.id,
            cmd.actor_user_id,
            ChatRoomMemberRole::Moderator,
        );
        self.room_repo.add_membership(&membership).await?;

        Ok(room.into())
    }
}
