use async_trait::async_trait;
use domain::entities::{
    Conversation, ConversationType, Message, MessageReaction, MessageRead, MessageType,
    ReactionType, RecipientType,
};
use domain::errors::DomainError;
use domain::repositories::{
    ConversationSummary, MessageListQuery, MessageRepository, MessageWithMeta,
};
use sqlx::{Error as SqlxError, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

pub struct PostgresMessageRepository {
    pool: PgPool,
}

impl PostgresMessageRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MessageRepository for PostgresMessageRepository {
    async fn create_conversation(&self, conversation: &Conversation) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO conversations (
                id, conversation_type, user_a_id, user_b_id, party_a_id, party_b_id,
                party_id, deal_id, room_id, title, last_message_at, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
            conversation.id,
            conversation.conversation_type.as_str(),
            conversation.user_a_id,
            conversation.user_b_id,
            conversation.party_a_id,
            conversation.party_b_id,
            conversation.party_id,
            conversation.deal_id,
            conversation.room_id,
            conversation.title,
            conversation.last_message_at,
            conversation.created_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn find_conversation_by_id(&self, id: Uuid) -> Result<Option<Conversation>, DomainError> {
        let row = sqlx::query_as!(
            ConversationRow,
            r#"
            SELECT id, conversation_type, user_a_id, user_b_id, party_a_id, party_b_id,
                party_id, deal_id, room_id, title, last_message_at, created_at
            FROM conversations
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_conversation))
    }

    async fn find_direct_user_conversation(
        &self,
        user_a_id: Uuid,
        user_b_id: Uuid,
    ) -> Result<Option<Conversation>, DomainError> {
        let row = sqlx::query_as!(
            ConversationRow,
            r#"
            SELECT id, conversation_type, user_a_id, user_b_id, party_a_id, party_b_id,
                party_id, deal_id, room_id, title, last_message_at, created_at
            FROM conversations
            WHERE conversation_type = 'DIRECT_USER'
              AND ((user_a_id = $1 AND user_b_id = $2) OR (user_a_id = $2 AND user_b_id = $1))
            "#,
            user_a_id,
            user_b_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_conversation))
    }

    async fn find_direct_party_conversation(
        &self,
        party_a_id: Uuid,
        party_b_id: Uuid,
    ) -> Result<Option<Conversation>, DomainError> {
        let row = sqlx::query_as!(
            ConversationRow,
            r#"
            SELECT id, conversation_type, user_a_id, user_b_id, party_a_id, party_b_id,
                party_id, deal_id, room_id, title, last_message_at, created_at
            FROM conversations
            WHERE conversation_type = 'DIRECT_PARTY'
              AND ((party_a_id = $1 AND party_b_id = $2) OR (party_a_id = $2 AND party_b_id = $1))
            "#,
            party_a_id,
            party_b_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_conversation))
    }

    async fn find_party_members_conversation(
        &self,
        party_id: Uuid,
    ) -> Result<Option<Conversation>, DomainError> {
        let row = sqlx::query_as!(
            ConversationRow,
            r#"
            SELECT id, conversation_type, user_a_id, user_b_id, party_a_id, party_b_id,
                party_id, deal_id, room_id, title, last_message_at, created_at
            FROM conversations
            WHERE conversation_type = 'PARTY_MEMBERS' AND party_id = $1
            "#,
            party_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_conversation))
    }

    async fn find_deal_conversation(
        &self,
        deal_id: Uuid,
    ) -> Result<Option<Conversation>, DomainError> {
        let row = sqlx::query_as!(
            ConversationRow,
            r#"
            SELECT id, conversation_type, user_a_id, user_b_id, party_a_id, party_b_id,
                party_id, deal_id, room_id, title, last_message_at, created_at
            FROM conversations
            WHERE conversation_type = 'DEAL' AND deal_id = $1
            "#,
            deal_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_conversation))
    }

    async fn find_room_conversation(
        &self,
        room_id: Uuid,
    ) -> Result<Option<Conversation>, DomainError> {
        let row = sqlx::query_as!(
            ConversationRow,
            r#"
            SELECT id, conversation_type, user_a_id, user_b_id, party_a_id, party_b_id,
                party_id, deal_id, room_id, title, last_message_at, created_at
            FROM conversations
            WHERE conversation_type = 'ROOM' AND room_id = $1
            "#,
            room_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_conversation))
    }

    async fn touch_conversation(
        &self,
        conversation_id: Uuid,
        last_message_at: OffsetDateTime,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE conversations
            SET last_message_at = $1
            WHERE id = $2
            "#,
            last_message_at,
            conversation_id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn create_message(&self, message: &Message) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO messages (
                id, conversation_id, sender_user_id, sender_party_id, recipient_type,
                recipient_user_id, recipient_party_id, recipient_deal_id, recipient_room_id,
                message_type, subject, content, content_encryption_key_id, attachment_urls,
                reply_to_message_id, is_pinned, pinned_at, is_deleted, edited_at, created_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20
            )
            "#,
            message.id,
            message.conversation_id,
            message.sender_user_id,
            message.sender_party_id,
            message.recipient_type.as_str(),
            message.recipient_user_id,
            message.recipient_party_id,
            message.recipient_deal_id,
            message.recipient_room_id,
            message.message_type.as_str(),
            message.subject,
            message.content,
            message.content_encryption_key_id,
            &message.attachment_urls,
            message.reply_to_message_id,
            message.is_pinned,
            message.pinned_at,
            message.is_deleted,
            message.edited_at,
            message.created_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn find_message_by_id(&self, id: Uuid) -> Result<Option<Message>, DomainError> {
        let row = sqlx::query_as!(
            MessageRow,
            r#"
            SELECT id, conversation_id, sender_user_id, sender_party_id, recipient_type,
                recipient_user_id, recipient_party_id, recipient_deal_id, recipient_room_id,
                message_type, subject, content, content_encryption_key_id, attachment_urls,
                reply_to_message_id, is_pinned, pinned_at, is_deleted, edited_at, created_at
            FROM messages
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_message))
    }

    async fn list_messages(
        &self,
        conversation_id: Uuid,
        query: &MessageListQuery,
    ) -> Result<Vec<MessageWithMeta>, DomainError> {
        let rows = sqlx::query_as!(
            MessageWithMetaRow,
            r#"
            SELECT
                m.id, m.conversation_id, m.sender_user_id, m.sender_party_id,
                m.recipient_type, m.recipient_user_id, m.recipient_party_id,
                m.recipient_deal_id, m.recipient_room_id, m.message_type,
                m.subject, m.content, m.content_encryption_key_id, m.attachment_urls,
                m.reply_to_message_id, m.is_pinned, m.pinned_at, m.is_deleted,
                m.edited_at, m.created_at,
                COUNT(DISTINCT mr.id) as "read_count!",
                COUNT(DISTINCT CASE WHEN r.reaction_type = 'LIKE' THEN r.id END) as "likes!",
                COUNT(DISTINCT CASE WHEN r.reaction_type = 'DISLIKE' THEN r.id END) as "dislikes!"
            FROM messages m
            LEFT JOIN message_reads mr ON mr.message_id = m.id
            LEFT JOIN message_reactions r ON r.message_id = m.id
            WHERE m.conversation_id = $1
              AND ($2::uuid IS NULL OR m.created_at < (SELECT created_at FROM messages WHERE id = $2))
            GROUP BY m.id
            ORDER BY m.created_at DESC
            LIMIT $3
            "#,
            conversation_id,
            query.before_id,
            query.limit
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows.into_iter().map(build_message_with_meta).collect())
    }

    async fn update_message(&self, message: &Message) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE messages
            SET content = $1,
                edited_at = $2,
                is_deleted = $3,
                is_pinned = $4,
                pinned_at = $5
            WHERE id = $6
            "#,
            message.content,
            message.edited_at,
            message.is_deleted,
            message.is_pinned,
            message.pinned_at,
            message.id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn soft_delete_message(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE messages
            SET is_deleted = true
            WHERE id = $1
            "#,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn set_message_pinned(
        &self,
        message_id: Uuid,
        is_pinned: bool,
        pinned_at: Option<OffsetDateTime>,
    ) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE messages
            SET is_pinned = $1, pinned_at = $2
            WHERE id = $3
            "#,
            is_pinned,
            pinned_at,
            message_id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn list_pinned_messages(
        &self,
        conversation_id: Uuid,
    ) -> Result<Vec<MessageWithMeta>, DomainError> {
        let rows = sqlx::query_as!(
            MessageWithMetaRow,
            r#"
            SELECT
                m.id, m.conversation_id, m.sender_user_id, m.sender_party_id,
                m.recipient_type, m.recipient_user_id, m.recipient_party_id,
                m.recipient_deal_id, m.recipient_room_id, m.message_type,
                m.subject, m.content, m.content_encryption_key_id, m.attachment_urls,
                m.reply_to_message_id, m.is_pinned, m.pinned_at, m.is_deleted,
                m.edited_at, m.created_at,
                COUNT(DISTINCT mr.id) as "read_count!",
                COUNT(DISTINCT CASE WHEN r.reaction_type = 'LIKE' THEN r.id END) as "likes!",
                COUNT(DISTINCT CASE WHEN r.reaction_type = 'DISLIKE' THEN r.id END) as "dislikes!"
            FROM messages m
            LEFT JOIN message_reads mr ON mr.message_id = m.id
            LEFT JOIN message_reactions r ON r.message_id = m.id
            WHERE m.conversation_id = $1 AND m.is_pinned = true AND m.is_deleted = false
            GROUP BY m.id
            ORDER BY m.pinned_at DESC NULLS LAST
            "#,
            conversation_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows.into_iter().map(build_message_with_meta).collect())
    }

    async fn mark_read(&self, read: &MessageRead) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO message_reads (id, message_id, user_id, party_id, read_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (message_id, user_id) DO NOTHING
            "#,
            read.id,
            read.message_id,
            read.user_id,
            read.party_id,
            read.read_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn find_read(
        &self,
        message_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<MessageRead>, DomainError> {
        let row = sqlx::query_as!(
            MessageReadRow,
            r#"
            SELECT id, message_id, user_id, party_id, read_at
            FROM message_reads
            WHERE message_id = $1 AND user_id = $2
            "#,
            message_id,
            user_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(build_read))
    }

    async fn unread_count_for_user(
        &self,
        user_id: Uuid,
        party_id: Option<Uuid>,
    ) -> Result<i64, DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM messages m
            JOIN conversations c ON c.id = m.conversation_id
            WHERE m.is_deleted = false
              AND m.sender_user_id != $1
              AND NOT EXISTS (
                  SELECT 1 FROM message_reads r
                  WHERE r.message_id = m.id AND r.user_id = $1
              )
              AND (
                  (c.conversation_type = 'DIRECT_USER' AND (c.user_a_id = $1 OR c.user_b_id = $1))
                  OR (c.conversation_type = 'DIRECT_PARTY'
                      AND $2::uuid IS NOT NULL
                      AND (c.party_a_id = $2 OR c.party_b_id = $2))
                  OR (c.conversation_type = 'PARTY_MEMBERS'
                      AND $2::uuid IS NOT NULL
                      AND c.party_id = $2)
                  OR (c.conversation_type = 'DEAL' AND EXISTS (
                      SELECT 1 FROM deal_participations dp
                      JOIN user_party_memberships upm ON upm.party_id = dp.party_id
                      WHERE dp.deal_id = c.deal_id AND upm.user_id = $1
                  ))
                  OR (c.conversation_type = 'ROOM' AND EXISTS (
                      SELECT 1 FROM chat_room_memberships crm
                      WHERE crm.room_id = c.room_id
                        AND (crm.user_id = $1 OR EXISTS (
                            SELECT 1 FROM user_party_memberships upm
                            WHERE upm.party_id = crm.party_id AND upm.user_id = $1
                        ))
                  ))
                  OR (c.conversation_type = 'ADMIN_BROADCAST' AND (
                      m.recipient_user_id = $1
                      OR m.recipient_party_id IN (
                          SELECT party_id FROM user_party_memberships WHERE user_id = $1
                      )
                  ))
              )
            "#,
            user_id,
            party_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.count.unwrap_or(0))
    }

    async fn list_conversations_for_user(
        &self,
        user_id: Uuid,
        party_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ConversationSummary>, DomainError> {
        let rows = sqlx::query!(
            r#"
            SELECT
                c.id, c.conversation_type, c.user_a_id, c.user_b_id,
                c.party_a_id, c.party_b_id, c.party_id, c.deal_id, c.room_id,
                c.title, c.last_message_at, c.created_at,
                (
                    SELECT COUNT(*)
                    FROM messages m
                    WHERE m.conversation_id = c.id
                      AND m.is_deleted = false
                      AND m.sender_user_id != $1
                      AND NOT EXISTS (
                          SELECT 1 FROM message_reads r
                          WHERE r.message_id = m.id AND r.user_id = $1
                      )
                ) as "unread_count!"
            FROM conversations c
            WHERE
                (c.conversation_type = 'DIRECT_USER' AND (c.user_a_id = $1 OR c.user_b_id = $1))
                OR (c.conversation_type = 'DIRECT_PARTY'
                    AND $2::uuid IS NOT NULL
                    AND (c.party_a_id = $2 OR c.party_b_id = $2))
                OR (c.conversation_type = 'PARTY_MEMBERS'
                    AND $2::uuid IS NOT NULL
                    AND c.party_id = $2)
                OR (c.conversation_type = 'DEAL' AND EXISTS (
                    SELECT 1 FROM deal_participations dp
                    JOIN user_party_memberships upm ON upm.party_id = dp.party_id
                    WHERE dp.deal_id = c.deal_id AND upm.user_id = $1
                ))
                OR (c.conversation_type = 'ROOM' AND EXISTS (
                    SELECT 1 FROM chat_room_memberships crm
                    WHERE crm.room_id = c.room_id
                      AND (crm.user_id = $1 OR EXISTS (
                          SELECT 1 FROM user_party_memberships upm
                          WHERE upm.party_id = crm.party_id AND upm.user_id = $1
                      ))
                ))
                OR (c.conversation_type = 'ADMIN_BROADCAST' AND EXISTS (
                    SELECT 1 FROM messages m
                    WHERE m.conversation_id = c.id
                      AND (m.recipient_user_id = $1
                           OR m.recipient_party_id IN (
                               SELECT party_id FROM user_party_memberships WHERE user_id = $1
                           ))
                ))
            ORDER BY c.last_message_at DESC
            LIMIT $3 OFFSET $4
            "#,
            user_id,
            party_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows
            .into_iter()
            .map(|r| ConversationSummary {
                conversation: Conversation {
                    id: r.id,
                    conversation_type: ConversationType::try_from(r.conversation_type.as_str())
                        .expect("stored conversation type is valid"),
                    user_a_id: r.user_a_id,
                    user_b_id: r.user_b_id,
                    party_a_id: r.party_a_id,
                    party_b_id: r.party_b_id,
                    party_id: r.party_id,
                    deal_id: r.deal_id,
                    room_id: r.room_id,
                    title: r.title,
                    last_message_at: r.last_message_at,
                    created_at: r.created_at,
                },
                unread_count: r.unread_count,
            })
            .collect())
    }

    async fn toggle_reaction(
        &self,
        reaction: &MessageReaction,
    ) -> Result<Option<MessageReaction>, DomainError> {
        let existing = sqlx::query!(
            r#"
            SELECT id FROM message_reactions
            WHERE message_id = $1 AND user_id = $2
              AND ($3::uuid IS NULL OR party_id = $3)
              AND reaction_type = $4
            "#,
            reaction.message_id,
            reaction.user_id,
            reaction.party_id,
            reaction.reaction_type.as_str()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        if let Some(row) = existing {
            sqlx::query!(
                r#"
                DELETE FROM message_reactions WHERE id = $1
                "#,
                row.id
            )
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
            return Ok(None);
        }

        sqlx::query!(
            r#"
            INSERT INTO message_reactions (id, message_id, user_id, party_id, reaction_type, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            reaction.id,
            reaction.message_id,
            reaction.user_id,
            reaction.party_id,
            reaction.reaction_type.as_str(),
            reaction.created_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(Some(reaction.clone()))
    }

    async fn list_reactions_for_message(
        &self,
        message_id: Uuid,
    ) -> Result<Vec<MessageReaction>, DomainError> {
        let rows = sqlx::query_as!(
            MessageReactionRow,
            r#"
            SELECT id, message_id, user_id, party_id, reaction_type, created_at
            FROM message_reactions
            WHERE message_id = $1
            ORDER BY created_at
            "#,
            message_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows.into_iter().map(build_reaction).collect())
    }

    async fn count_messages_by_recipient(
        &self,
        recipient_type: RecipientType,
        recipient_id: Uuid,
    ) -> Result<i64, DomainError> {
        match recipient_type {
            RecipientType::User => {
                let row = sqlx::query!(
                    r#"SELECT COUNT(*) as count FROM messages WHERE recipient_type = 'USER' AND recipient_user_id = $1"#,
                    recipient_id
                )
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
                Ok(row.count.unwrap_or(0))
            }
            RecipientType::Party => {
                let row = sqlx::query!(
                    r#"SELECT COUNT(*) as count FROM messages WHERE recipient_type = 'PARTY' AND recipient_party_id = $1"#,
                    recipient_id
                )
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
                Ok(row.count.unwrap_or(0))
            }
            RecipientType::Deal => {
                let row = sqlx::query!(
                    r#"SELECT COUNT(*) as count FROM messages WHERE recipient_type = 'DEAL' AND recipient_deal_id = $1"#,
                    recipient_id
                )
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
                Ok(row.count.unwrap_or(0))
            }
            RecipientType::Room => {
                let row = sqlx::query!(
                    r#"SELECT COUNT(*) as count FROM messages WHERE recipient_type = 'ROOM' AND recipient_room_id = $1"#,
                    recipient_id
                )
                .fetch_one(&self.pool)
                .await
                .map_err(map_err)?;
                Ok(row.count.unwrap_or(0))
            }
            _ => Ok(0),
        }
    }
}

#[derive(sqlx::FromRow)]
struct ConversationRow {
    id: Uuid,
    conversation_type: String,
    user_a_id: Option<Uuid>,
    user_b_id: Option<Uuid>,
    party_a_id: Option<Uuid>,
    party_b_id: Option<Uuid>,
    party_id: Option<Uuid>,
    deal_id: Option<Uuid>,
    room_id: Option<Uuid>,
    title: Option<String>,
    last_message_at: OffsetDateTime,
    created_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct MessageRow {
    id: Uuid,
    conversation_id: Uuid,
    sender_user_id: Uuid,
    sender_party_id: Option<Uuid>,
    recipient_type: String,
    recipient_user_id: Option<Uuid>,
    recipient_party_id: Option<Uuid>,
    recipient_deal_id: Option<Uuid>,
    recipient_room_id: Option<Uuid>,
    message_type: String,
    subject: Option<String>,
    content: String,
    content_encryption_key_id: Option<Uuid>,
    attachment_urls: Vec<String>,
    reply_to_message_id: Option<Uuid>,
    is_pinned: bool,
    pinned_at: Option<OffsetDateTime>,
    is_deleted: bool,
    edited_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct MessageWithMetaRow {
    id: Uuid,
    conversation_id: Uuid,
    sender_user_id: Uuid,
    sender_party_id: Option<Uuid>,
    recipient_type: String,
    recipient_user_id: Option<Uuid>,
    recipient_party_id: Option<Uuid>,
    recipient_deal_id: Option<Uuid>,
    recipient_room_id: Option<Uuid>,
    message_type: String,
    subject: Option<String>,
    content: String,
    content_encryption_key_id: Option<Uuid>,
    attachment_urls: Vec<String>,
    reply_to_message_id: Option<Uuid>,
    is_pinned: bool,
    pinned_at: Option<OffsetDateTime>,
    is_deleted: bool,
    edited_at: Option<OffsetDateTime>,
    created_at: OffsetDateTime,
    read_count: i64,
    likes: i64,
    dislikes: i64,
}

#[derive(sqlx::FromRow)]
struct MessageReadRow {
    id: Uuid,
    message_id: Uuid,
    user_id: Uuid,
    party_id: Option<Uuid>,
    read_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct MessageReactionRow {
    id: Uuid,
    message_id: Uuid,
    user_id: Uuid,
    party_id: Option<Uuid>,
    reaction_type: String,
    created_at: OffsetDateTime,
}

fn build_conversation(row: ConversationRow) -> Conversation {
    Conversation {
        id: row.id,
        conversation_type: ConversationType::try_from(row.conversation_type.as_str())
            .expect("stored conversation type is valid"),
        user_a_id: row.user_a_id,
        user_b_id: row.user_b_id,
        party_a_id: row.party_a_id,
        party_b_id: row.party_b_id,
        party_id: row.party_id,
        deal_id: row.deal_id,
        room_id: row.room_id,
        title: row.title,
        last_message_at: row.last_message_at,
        created_at: row.created_at,
    }
}

fn build_message(row: MessageRow) -> Message {
    let mut message = Message::new(
        row.id,
        row.conversation_id,
        row.sender_user_id,
        row.sender_party_id,
        RecipientType::try_from(row.recipient_type.as_str())
            .expect("stored recipient type is valid"),
        row.recipient_user_id,
        row.recipient_party_id,
        row.recipient_deal_id,
        row.recipient_room_id,
        MessageType::try_from(row.message_type.as_str()).expect("stored message type is valid"),
        row.subject,
        row.content,
        row.attachment_urls,
        row.reply_to_message_id,
    )
    .expect("stored message is valid");

    message.content_encryption_key_id = row.content_encryption_key_id;
    message.is_pinned = row.is_pinned;
    message.pinned_at = row.pinned_at;
    message.is_deleted = row.is_deleted;
    message.edited_at = row.edited_at;
    message.created_at = row.created_at;

    message
}

fn build_message_with_meta(row: MessageWithMetaRow) -> MessageWithMeta {
    let message = build_message(MessageRow {
        id: row.id,
        conversation_id: row.conversation_id,
        sender_user_id: row.sender_user_id,
        sender_party_id: row.sender_party_id,
        recipient_type: row.recipient_type,
        recipient_user_id: row.recipient_user_id,
        recipient_party_id: row.recipient_party_id,
        recipient_deal_id: row.recipient_deal_id,
        recipient_room_id: row.recipient_room_id,
        message_type: row.message_type,
        subject: row.subject,
        content: row.content,
        content_encryption_key_id: row.content_encryption_key_id,
        attachment_urls: row.attachment_urls,
        reply_to_message_id: row.reply_to_message_id,
        is_pinned: row.is_pinned,
        pinned_at: row.pinned_at,
        is_deleted: row.is_deleted,
        edited_at: row.edited_at,
        created_at: row.created_at,
    });

    MessageWithMeta {
        message,
        read_count: row.read_count,
        likes: row.likes,
        dislikes: row.dislikes,
        user_reaction: None,
    }
}

fn build_read(row: MessageReadRow) -> MessageRead {
    MessageRead {
        id: row.id,
        message_id: row.message_id,
        user_id: row.user_id,
        party_id: row.party_id,
        read_at: row.read_at,
    }
}

fn build_reaction(row: MessageReactionRow) -> MessageReaction {
    MessageReaction {
        id: row.id,
        message_id: row.message_id,
        user_id: row.user_id,
        party_id: row.party_id,
        reaction_type: ReactionType::try_from(row.reaction_type.as_str())
            .expect("stored reaction type is valid"),
        created_at: row.created_at,
    }
}

fn map_err(err: SqlxError) -> DomainError {
    if let SqlxError::Database(db_err) = &err {
        match db_err.constraint() {
            Some("messages_pkey") => {
                return DomainError::RepositoryError("message already exists".to_string())
            }
            Some("conversations_pkey") => {
                return DomainError::RepositoryError("conversation already exists".to_string())
            }
            Some("idx_conversations_direct_user")
            | Some("idx_conversations_direct_party")
            | Some("idx_conversations_party_members")
            | Some("idx_conversations_deal")
            | Some("idx_conversations_room") => {
                return DomainError::RepositoryError(
                    "conversation already exists for this context".to_string(),
                )
            }
            Some("messages_conversation_id_fkey") => return DomainError::ConversationNotFound,
            Some("messages_sender_party_id_fkey") | Some("messages_recipient_party_id_fkey") => {
                return DomainError::PartyNotFound
            }
            Some("messages_recipient_deal_id_fkey") => return DomainError::DealNotFound,
            Some("messages_recipient_room_id_fkey") => return DomainError::ChatRoomNotFound,
            Some("messages_reply_to_message_id_fkey") => return DomainError::MessageNotFound,
            Some("conversations_party_a_id_fkey")
            | Some("conversations_party_b_id_fkey")
            | Some("conversations_party_id_fkey") => return DomainError::PartyNotFound,
            Some("conversations_deal_id_fkey") => return DomainError::DealNotFound,
            Some("conversations_room_id_fkey") => return DomainError::ChatRoomNotFound,
            Some("message_reads_message_id_fkey") => return DomainError::MessageNotFound,
            Some("message_reactions_message_id_fkey") => return DomainError::MessageNotFound,
            _ => {}
        }
    }
    DomainError::RepositoryError(err.to_string())
}
