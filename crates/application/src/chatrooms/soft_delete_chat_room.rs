use crate::chatrooms::access::can_manage_chat_room;
use crate::chatrooms::dto::SoftDeleteChatRoomCommand;
use crate::errors::ApplicationError;
use crate::ports::RealtimePublisher;
use domain::repositories::ChatRoomRepository;
use std::sync::Arc;

#[derive(Clone)]
pub struct SoftDeleteChatRoom {
    room_repo: Arc<dyn ChatRoomRepository>,
    publisher: Arc<dyn RealtimePublisher>,
}

impl SoftDeleteChatRoom {
    pub fn new(
        room_repo: Arc<dyn ChatRoomRepository>,
        publisher: Arc<dyn RealtimePublisher>,
    ) -> Self {
        Self {
            room_repo,
            publisher,
        }
    }

    pub async fn execute(&self, cmd: SoftDeleteChatRoomCommand) -> Result<(), ApplicationError> {
        let mut room = self
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

        room.soft_delete();
        self.room_repo.update_room(&room).await?;

        self.publisher
            .publish(crate::ports::MessageEvent::RoomDeleted { room_id: room.id })
            .await?;

        Ok(())
    }
}
