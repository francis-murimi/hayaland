use crate::chatrooms::access::is_chat_room_visible;
use crate::chatrooms::dto::ChatRoomResult;
use crate::errors::ApplicationError;
use domain::repositories::ChatRoomRepository;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct GetChatRoom {
    room_repo: Arc<dyn ChatRoomRepository>,
}

impl GetChatRoom {
    pub fn new(room_repo: Arc<dyn ChatRoomRepository>) -> Self {
        Self { room_repo }
    }

    pub async fn execute(
        &self,
        room_id: Uuid,
        actor_user_id: Uuid,
        actor_party_ids: Vec<Uuid>,
        is_admin: bool,
    ) -> Result<ChatRoomResult, ApplicationError> {
        let room = self
            .room_repo
            .find_room_by_id(room_id)
            .await?
            .ok_or(ApplicationError::ChatRoomNotFound)?;

        if !is_chat_room_visible(
            &room,
            actor_user_id,
            &actor_party_ids,
            is_admin,
            self.room_repo.as_ref(),
        )
        .await?
        {
            return Err(ApplicationError::Forbidden);
        }

        Ok(room.into())
    }
}
