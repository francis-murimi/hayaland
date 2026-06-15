use crate::chatrooms::dto::{ChatRoomListQuery, ChatRoomResult};
use crate::errors::ApplicationError;
use domain::repositories::{ChatRoomRepository, PartyRepository};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct ListChatRooms {
    room_repo: Arc<dyn ChatRoomRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl ListChatRooms {
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
        actor_user_id: Uuid,
        query: ChatRoomListQuery,
    ) -> Result<Vec<ChatRoomResult>, ApplicationError> {
        let memberships = self
            .party_repo
            .list_memberships_for_user(actor_user_id)
            .await?;
        let party_ids: Vec<Uuid> = memberships.into_iter().map(|(_, party)| party.id).collect();

        let visible_room_ids = self
            .room_repo
            .list_room_ids_for_user(actor_user_id, &party_ids)
            .await?;

        let domain_query = domain::repositories::ChatRoomListQuery {
            room_type: query.room_type,
            include_deleted: query.include_deleted,
            limit: query.limit,
            offset: query.offset,
        };
        let rooms = self
            .room_repo
            .list_rooms(&domain_query, &visible_room_ids)
            .await?;

        Ok(rooms.into_iter().map(Into::into).collect())
    }
}
