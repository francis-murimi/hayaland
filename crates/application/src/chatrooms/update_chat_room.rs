use crate::chatrooms::access::can_manage_chat_room;
use crate::chatrooms::dto::{ChatRoomResult, UpdateChatRoomCommand};
use crate::errors::ApplicationError;
use domain::entities::ChatRoomName;
use domain::repositories::ChatRoomRepository;
use std::sync::Arc;

#[derive(Clone)]
pub struct UpdateChatRoom {
    room_repo: Arc<dyn ChatRoomRepository>,
}

impl UpdateChatRoom {
    pub fn new(room_repo: Arc<dyn ChatRoomRepository>) -> Self {
        Self { room_repo }
    }

    pub async fn execute(
        &self,
        cmd: UpdateChatRoomCommand,
    ) -> Result<ChatRoomResult, ApplicationError> {
        let mut room = self
            .room_repo
            .find_room_by_id(cmd.room_id)
            .await?
            .ok_or(ApplicationError::ChatRoomNotFound)?;

        if room.is_deleted {
            return Err(ApplicationError::ChatRoomNotFound);
        }

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

        let new_name = match cmd.name {
            Some(n) => Some(ChatRoomName::new(&n).map_err(ApplicationError::from)?),
            None => None,
        };

        if let Some(ref name) = new_name {
            if name.as_str() != room.name.as_str()
                && self
                    .room_repo
                    .find_room_by_name(name.as_str())
                    .await?
                    .is_some()
            {
                return Err(ApplicationError::ChatRoomAlreadyExists);
            }
        }

        room.update(new_name, cmd.description);
        if let Some(room_type) = cmd.room_type {
            room.room_type = room_type;
        }
        self.room_repo.update_room(&room).await?;

        Ok(room.into())
    }
}
