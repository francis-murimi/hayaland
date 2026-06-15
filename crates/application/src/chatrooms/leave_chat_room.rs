use crate::chatrooms::dto::LeaveChatRoomCommand;
use crate::errors::ApplicationError;
use domain::repositories::ChatRoomRepository;
use std::sync::Arc;

#[derive(Clone)]
pub struct LeaveChatRoom {
    room_repo: Arc<dyn ChatRoomRepository>,
}

impl LeaveChatRoom {
    pub fn new(room_repo: Arc<dyn ChatRoomRepository>) -> Self {
        Self { room_repo }
    }

    pub async fn execute(&self, cmd: LeaveChatRoomCommand) -> Result<(), ApplicationError> {
        let membership = if let Some(party_id) = cmd.actor_party_id {
            self.room_repo
                .find_membership_for_party(cmd.room_id, party_id)
                .await?
                .ok_or(ApplicationError::ChatRoomMembershipNotFound)?
        } else {
            self.room_repo
                .find_membership_for_user(cmd.room_id, cmd.actor_user_id)
                .await?
                .ok_or(ApplicationError::ChatRoomMembershipNotFound)?
        };

        let belongs = membership.user_id == Some(cmd.actor_user_id)
            || membership.party_id == cmd.actor_party_id;
        if !belongs {
            return Err(ApplicationError::Forbidden);
        }

        self.room_repo.remove_membership(membership.id).await?;
        Ok(())
    }
}
