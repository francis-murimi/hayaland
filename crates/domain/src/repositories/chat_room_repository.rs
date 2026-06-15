use crate::entities::{ChatRoom, ChatRoomMemberRole, ChatRoomMembership, ChatRoomType};
use crate::errors::DomainError;
use async_trait::async_trait;
use uuid::Uuid;

/// Pagination / filtering for listing chatrooms.
#[derive(Debug, Clone, Default)]
pub struct ChatRoomListQuery {
    pub room_type: Option<ChatRoomType>,
    pub include_deleted: bool,
    pub limit: i64,
    pub offset: i64,
}

/// Outbound port for persisting and retrieving chatrooms and memberships.
#[async_trait]
pub trait ChatRoomRepository: Send + Sync {
    /// Create a chatroom.
    async fn create_room(&self, room: &ChatRoom) -> Result<(), DomainError>;

    /// Find a chatroom by id.
    async fn find_room_by_id(&self, id: Uuid) -> Result<Option<ChatRoom>, DomainError>;

    /// Find a chatroom by name.
    async fn find_room_by_name(&self, name: &str) -> Result<Option<ChatRoom>, DomainError>;

    /// Update chatroom fields.
    async fn update_room(&self, room: &ChatRoom) -> Result<(), DomainError>;

    /// Soft-delete a chatroom.
    async fn soft_delete_room(&self, id: Uuid) -> Result<(), DomainError>;

    /// List chatrooms the caller can see.
    async fn list_rooms(
        &self,
        query: &ChatRoomListQuery,
        visible_room_ids: &[Uuid],
    ) -> Result<Vec<ChatRoom>, DomainError>;

    /// Count chatrooms the caller can see.
    async fn count_rooms(
        &self,
        query: &ChatRoomListQuery,
        visible_room_ids: &[Uuid],
    ) -> Result<i64, DomainError>;

    /// Add a membership.
    async fn add_membership(&self, membership: &ChatRoomMembership) -> Result<(), DomainError>;

    /// Remove a membership by id.
    async fn remove_membership(&self, membership_id: Uuid) -> Result<(), DomainError>;

    /// Find a membership by id.
    async fn find_membership_by_id(
        &self,
        membership_id: Uuid,
    ) -> Result<Option<ChatRoomMembership>, DomainError>;

    /// Find a user's membership in a room.
    async fn find_membership_for_user(
        &self,
        room_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<ChatRoomMembership>, DomainError>;

    /// Find a party's membership in a room.
    async fn find_membership_for_party(
        &self,
        room_id: Uuid,
        party_id: Uuid,
    ) -> Result<Option<ChatRoomMembership>, DomainError>;

    /// Update a membership's role.
    async fn update_membership_role(
        &self,
        membership_id: Uuid,
        role: ChatRoomMemberRole,
    ) -> Result<(), DomainError>;

    /// List memberships for a room.
    async fn list_memberships_for_room(
        &self,
        room_id: Uuid,
    ) -> Result<Vec<ChatRoomMembership>, DomainError>;

    /// List room ids a user can access (directly or through parties).
    async fn list_room_ids_for_user(
        &self,
        user_id: Uuid,
        party_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, DomainError>;

    /// Check whether a user is a member of a room (directly or through a party).
    async fn is_user_in_room(
        &self,
        room_id: Uuid,
        user_id: Uuid,
        party_ids: &[Uuid],
    ) -> Result<bool, DomainError>;

    /// Check whether any of the given parties is a member of a room.
    async fn is_party_in_room(
        &self,
        room_id: Uuid,
        party_ids: &[Uuid],
    ) -> Result<bool, DomainError>;

    /// List rooms where a user is a member.
    async fn list_rooms_for_user(
        &self,
        user_id: Uuid,
        party_ids: &[Uuid],
    ) -> Result<Vec<ChatRoom>, DomainError>;
}
