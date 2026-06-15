use async_trait::async_trait;
use domain::entities::{ChatRoom, ChatRoomMemberRole, ChatRoomMembership, ChatRoomType};
use domain::errors::DomainError;
use domain::repositories::{ChatRoomListQuery, ChatRoomRepository};
use sqlx::{Error as SqlxError, PgPool};
use uuid::Uuid;

pub struct PostgresChatRoomRepository {
    pool: PgPool,
}

impl PostgresChatRoomRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ChatRoomRepository for PostgresChatRoomRepository {
    async fn create_room(&self, room: &ChatRoom) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO chat_rooms (id, name, description, room_type, created_by_user_id, is_deleted, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            room.id,
            room.name.as_str(),
            room.description,
            room.room_type.as_str(),
            room.created_by_user_id,
            room.is_deleted,
            room.created_at,
            room.updated_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn find_room_by_id(&self, id: Uuid) -> Result<Option<ChatRoom>, DomainError> {
        let row = sqlx::query_as!(
            ChatRoomRow,
            r#"
            SELECT id, name, description, room_type, created_by_user_id, is_deleted, created_at, updated_at
            FROM chat_rooms
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_room))
    }

    async fn find_room_by_name(&self, name: &str) -> Result<Option<ChatRoom>, DomainError> {
        let row = sqlx::query_as!(
            ChatRoomRow,
            r#"
            SELECT id, name, description, room_type, created_by_user_id, is_deleted, created_at, updated_at
            FROM chat_rooms
            WHERE name = $1
            "#,
            name
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_room))
    }

    async fn update_room(&self, room: &ChatRoom) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE chat_rooms
            SET name = $1,
                description = $2,
                room_type = $3,
                created_by_user_id = $4,
                is_deleted = $5,
                created_at = $6,
                updated_at = $7
            WHERE id = $8
            "#,
            room.name.as_str(),
            room.description,
            room.room_type.as_str(),
            room.created_by_user_id,
            room.is_deleted,
            room.created_at,
            room.updated_at,
            room.id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn soft_delete_room(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE chat_rooms
            SET is_deleted = true, updated_at = now()
            WHERE id = $1
            "#,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn list_rooms(
        &self,
        query: &ChatRoomListQuery,
        visible_room_ids: &[Uuid],
    ) -> Result<Vec<ChatRoom>, DomainError> {
        let rows = sqlx::query_as!(
            ChatRoomRow,
            r#"
            SELECT id, name, description, room_type, created_by_user_id, is_deleted, created_at, updated_at
            FROM chat_rooms
            WHERE ($1::bool = true OR is_deleted = false)
              AND ($2::text IS NULL OR room_type = $2)
              AND (room_type = 'PUBLIC' OR id = ANY($3))
            ORDER BY created_at DESC
            LIMIT $4 OFFSET $5
            "#,
            query.include_deleted,
            query.room_type.map(|t| t.as_str()),
            visible_room_ids,
            query.limit,
            query.offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows.into_iter().map(build_room).collect())
    }

    async fn count_rooms(
        &self,
        query: &ChatRoomListQuery,
        visible_room_ids: &[Uuid],
    ) -> Result<i64, DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM chat_rooms
            WHERE ($1::bool = true OR is_deleted = false)
              AND ($2::text IS NULL OR room_type = $2)
              AND (room_type = 'PUBLIC' OR id = ANY($3))
            "#,
            query.include_deleted,
            query.room_type.map(|t| t.as_str()),
            visible_room_ids
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.count.unwrap_or(0))
    }

    async fn add_membership(&self, membership: &ChatRoomMembership) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO chat_room_memberships (id, room_id, user_id, party_id, member_role, joined_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            membership.id,
            membership.room_id,
            membership.user_id,
            membership.party_id,
            membership.member_role.as_str(),
            membership.joined_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn remove_membership(&self, membership_id: Uuid) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            DELETE FROM chat_room_memberships WHERE id = $1
            "#,
            membership_id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn find_membership_by_id(
        &self,
        membership_id: Uuid,
    ) -> Result<Option<ChatRoomMembership>, DomainError> {
        let row = sqlx::query_as!(
            ChatRoomMembershipRow,
            r#"
            SELECT id, room_id, user_id, party_id, member_role, joined_at
            FROM chat_room_memberships
            WHERE id = $1
            "#,
            membership_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_membership))
    }

    async fn find_membership_for_user(
        &self,
        room_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<ChatRoomMembership>, DomainError> {
        let row = sqlx::query_as!(
            ChatRoomMembershipRow,
            r#"
            SELECT id, room_id, user_id, party_id, member_role, joined_at
            FROM chat_room_memberships
            WHERE room_id = $1 AND user_id = $2
            "#,
            room_id,
            user_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_membership))
    }

    async fn find_membership_for_party(
        &self,
        room_id: Uuid,
        party_id: Uuid,
    ) -> Result<Option<ChatRoomMembership>, DomainError> {
        let row = sqlx::query_as!(
            ChatRoomMembershipRow,
            r#"
            SELECT id, room_id, user_id, party_id, member_role, joined_at
            FROM chat_room_memberships
            WHERE room_id = $1 AND party_id = $2
            "#,
            room_id,
            party_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_membership))
    }

    async fn update_membership_role(
        &self,
        membership_id: Uuid,
        role: ChatRoomMemberRole,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE chat_room_memberships
            SET member_role = $1
            WHERE id = $2
            "#,
            role.as_str(),
            membership_id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn list_memberships_for_room(
        &self,
        room_id: Uuid,
    ) -> Result<Vec<ChatRoomMembership>, DomainError> {
        let rows = sqlx::query_as!(
            ChatRoomMembershipRow,
            r#"
            SELECT id, room_id, user_id, party_id, member_role, joined_at
            FROM chat_room_memberships
            WHERE room_id = $1
            ORDER BY joined_at
            "#,
            room_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows.into_iter().map(build_membership).collect())
    }

    async fn list_room_ids_for_user(
        &self,
        user_id: Uuid,
        party_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, DomainError> {
        let rows = sqlx::query!(
            r#"
            SELECT DISTINCT room_id
            FROM chat_room_memberships
            WHERE user_id = $1 OR party_id = ANY($2)
            "#,
            user_id,
            party_ids
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows.into_iter().map(|r| r.room_id).collect())
    }

    async fn is_user_in_room(
        &self,
        room_id: Uuid,
        user_id: Uuid,
        party_ids: &[Uuid],
    ) -> Result<bool, DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM chat_room_memberships
                WHERE room_id = $1
                  AND (user_id = $2 OR party_id = ANY($3))
            ) as exists
            "#,
            room_id,
            user_id,
            party_ids
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.exists.unwrap_or(false))
    }

    async fn is_party_in_room(
        &self,
        room_id: Uuid,
        party_ids: &[Uuid],
    ) -> Result<bool, DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM chat_room_memberships
                WHERE room_id = $1 AND party_id = ANY($2)
            ) as exists
            "#,
            room_id,
            party_ids
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.exists.unwrap_or(false))
    }

    async fn list_rooms_for_user(
        &self,
        user_id: Uuid,
        party_ids: &[Uuid],
    ) -> Result<Vec<ChatRoom>, DomainError> {
        let rows = sqlx::query_as!(
            ChatRoomRow,
            r#"
            SELECT r.id, r.name, r.description, r.room_type, r.created_by_user_id, r.is_deleted, r.created_at, r.updated_at
            FROM chat_rooms r
            WHERE r.is_deleted = false
              AND EXISTS (
                  SELECT 1 FROM chat_room_memberships m
                  WHERE m.room_id = r.id
                    AND (m.user_id = $1 OR m.party_id = ANY($2))
              )
            ORDER BY r.created_at DESC
            "#,
            user_id,
            party_ids
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows.into_iter().map(build_room).collect())
    }
}

#[derive(sqlx::FromRow)]
struct ChatRoomRow {
    id: Uuid,
    name: String,
    description: Option<String>,
    room_type: String,
    created_by_user_id: Uuid,
    is_deleted: bool,
    created_at: time::OffsetDateTime,
    updated_at: time::OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct ChatRoomMembershipRow {
    id: Uuid,
    room_id: Uuid,
    user_id: Option<Uuid>,
    party_id: Option<Uuid>,
    member_role: String,
    joined_at: time::OffsetDateTime,
}

fn build_room(row: ChatRoomRow) -> ChatRoom {
    let mut room = ChatRoom::new(
        row.id,
        domain::entities::ChatRoomName::new(&row.name).expect("stored room name is valid"),
        row.description,
        ChatRoomType::try_from(row.room_type.as_str()).expect("stored room type is valid"),
        row.created_by_user_id,
    );
    room.is_deleted = row.is_deleted;
    room.created_at = row.created_at;
    room.updated_at = row.updated_at;
    room
}

fn build_membership(row: ChatRoomMembershipRow) -> ChatRoomMembership {
    ChatRoomMembership {
        id: row.id,
        room_id: row.room_id,
        user_id: row.user_id,
        party_id: row.party_id,
        member_role: ChatRoomMemberRole::try_from(row.member_role.as_str())
            .expect("stored member role is valid"),
        joined_at: row.joined_at,
    }
}

fn map_err(err: SqlxError) -> DomainError {
    if let SqlxError::Database(db_err) = &err {
        match db_err.constraint() {
            Some("chat_rooms_pkey") => {
                return DomainError::RepositoryError("chat room already exists".to_string())
            }
            Some("chat_rooms_name_key") => return DomainError::ChatRoomAlreadyExists,
            Some("chat_room_memberships_room_id_user_id_key")
            | Some("chat_room_memberships_room_id_party_id_key") => {
                return DomainError::AlreadyChatRoomMember
            }
            Some("chat_room_memberships_room_id_fkey") => return DomainError::ChatRoomNotFound,
            Some("chat_room_memberships_party_id_fkey") => return DomainError::PartyNotFound,
            _ => {}
        }
    }
    DomainError::RepositoryError(err.to_string())
}
