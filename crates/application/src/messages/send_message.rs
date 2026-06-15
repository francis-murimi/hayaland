use crate::errors::ApplicationError;
use crate::messages::dto::{to_message_result, MessageResult, SendMessageCommand};
use crate::ports::{EncryptionService, RealtimePublisher};
use domain::entities::message::validate_recipient;
use domain::entities::{Conversation, Message, RecipientType};
use domain::repositories::{
    ChatRoomRepository, DealRepository, MessageRepository, PartyRepository,
};
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone)]
pub struct SendMessage {
    message_repo: Arc<dyn MessageRepository>,
    party_repo: Arc<dyn PartyRepository>,
    deal_repo: Arc<dyn DealRepository>,
    room_repo: Arc<dyn ChatRoomRepository>,
    encryption: Arc<dyn EncryptionService>,
    publisher: Arc<dyn RealtimePublisher>,
}

impl SendMessage {
    pub fn new(
        message_repo: Arc<dyn MessageRepository>,
        party_repo: Arc<dyn PartyRepository>,
        deal_repo: Arc<dyn DealRepository>,
        room_repo: Arc<dyn ChatRoomRepository>,
        encryption: Arc<dyn EncryptionService>,
        publisher: Arc<dyn RealtimePublisher>,
    ) -> Self {
        Self {
            message_repo,
            party_repo,
            deal_repo,
            room_repo,
            encryption,
            publisher,
        }
    }

    pub async fn execute(
        &self,
        cmd: SendMessageCommand,
    ) -> Result<MessageResult, ApplicationError> {
        if cmd.content.trim().is_empty() {
            return Err(ApplicationError::Validation(vec![
                "message content cannot be empty".to_string(),
            ]));
        }

        validate_recipient(
            &cmd.recipient_type,
            cmd.actor_party_id,
            cmd.recipient_user_id,
            cmd.recipient_party_id,
            cmd.recipient_deal_id,
            cmd.recipient_room_id,
        )
        .map_err(ApplicationError::from)?;

        match cmd.recipient_type {
            RecipientType::AdminBroadcast => {
                return Err(ApplicationError::Validation(vec![
                    "use AdminBroadcast for broadcast messages".to_string(),
                ]));
            }
            RecipientType::User => {}
            RecipientType::Party | RecipientType::PartyMembers => {
                let party_id = cmd.actor_party_id.ok_or(ApplicationError::Forbidden)?;
                if !self
                    .party_repo
                    .is_user_member_of_party(cmd.actor_user_id, party_id)
                    .await?
                {
                    return Err(ApplicationError::Forbidden);
                }
            }
            RecipientType::Deal => {
                if cmd.is_admin {
                    // admins bypass party membership check
                } else {
                    let party_id = cmd.actor_party_id.ok_or(ApplicationError::Forbidden)?;
                    if !self
                        .deal_repo
                        .is_party_participant(
                            cmd.recipient_deal_id.unwrap_or_else(Uuid::nil),
                            party_id,
                        )
                        .await?
                        || !self
                            .party_repo
                            .is_user_member_of_party(cmd.actor_user_id, party_id)
                            .await?
                    {
                        return Err(ApplicationError::Forbidden);
                    }
                }
            }
            RecipientType::Room => {
                let room_id = cmd.recipient_room_id.ok_or(ApplicationError::Forbidden)?;
                let party_ids: Vec<_> = cmd.actor_party_id.into_iter().collect();
                if cmd.is_admin
                    || self
                        .room_repo
                        .is_user_in_room(room_id, cmd.actor_user_id, &party_ids)
                        .await?
                {
                    // allowed
                } else {
                    return Err(ApplicationError::Forbidden);
                }
            }
        }

        let conversation = self.resolve_or_create_conversation(&cmd).await?;

        if let Some(reply_id) = cmd.reply_to_message_id {
            let reply = self
                .message_repo
                .find_message_by_id(reply_id)
                .await?
                .ok_or(ApplicationError::ReplyNotInSameContext)?;
            if reply.conversation_id != conversation.id {
                return Err(ApplicationError::ReplyNotInSameContext);
            }
        }

        let encrypted = self.encryption.encrypt(&cmd.content).await?;
        let message = Message::new(
            Uuid::now_v7(),
            conversation.id,
            cmd.actor_user_id,
            cmd.actor_party_id,
            cmd.recipient_type,
            cmd.recipient_user_id,
            cmd.recipient_party_id,
            cmd.recipient_deal_id,
            cmd.recipient_room_id,
            cmd.message_type,
            cmd.subject,
            encrypted,
            cmd.attachment_urls,
            cmd.reply_to_message_id,
        )
        .map_err(ApplicationError::from)?;

        self.message_repo.create_message(&message).await?;
        self.message_repo
            .touch_conversation(conversation.id, OffsetDateTime::now_utc())
            .await?;

        self.publisher
            .publish(crate::ports::MessageEvent::MessageNew {
                message_id: message.id,
                conversation_id: message.conversation_id,
                sender_user_id: message.sender_user_id,
                sender_party_id: message.sender_party_id,
                recipient_type: message.recipient_type,
                recipient_user_id: message.recipient_user_id,
                recipient_party_id: message.recipient_party_id,
                recipient_deal_id: message.recipient_deal_id,
                recipient_room_id: message.recipient_room_id,
                message_type: message.message_type,
                subject: message.subject.clone(),
                content: cmd.content.clone(),
                reply_to_message_id: message.reply_to_message_id,
                created_at: message.created_at,
            })
            .await?;

        to_message_result(message, &self.encryption).await
    }

    async fn resolve_or_create_conversation(
        &self,
        cmd: &SendMessageCommand,
    ) -> Result<Conversation, ApplicationError> {
        let conversation = match cmd.recipient_type {
            RecipientType::User => {
                let other = cmd.recipient_user_id.ok_or(ApplicationError::Forbidden)?;
                let (a, b) = if cmd.actor_user_id < other {
                    (cmd.actor_user_id, other)
                } else {
                    (other, cmd.actor_user_id)
                };
                match self
                    .message_repo
                    .find_direct_user_conversation(a, b)
                    .await?
                {
                    Some(c) => c,
                    None => {
                        let c = Conversation::new_direct_user(Uuid::now_v7(), a, b);
                        self.message_repo.create_conversation(&c).await?;
                        c
                    }
                }
            }
            RecipientType::Party => {
                let sender = cmd.actor_party_id.ok_or(ApplicationError::Forbidden)?;
                let other = cmd.recipient_party_id.ok_or(ApplicationError::Forbidden)?;
                let (a, b) = if sender < other {
                    (sender, other)
                } else {
                    (other, sender)
                };
                match self
                    .message_repo
                    .find_direct_party_conversation(a, b)
                    .await?
                {
                    Some(c) => c,
                    None => {
                        let c = Conversation::new_direct_party(Uuid::now_v7(), a, b);
                        self.message_repo.create_conversation(&c).await?;
                        c
                    }
                }
            }
            RecipientType::PartyMembers => {
                let party_id = cmd.actor_party_id.ok_or(ApplicationError::Forbidden)?;
                match self
                    .message_repo
                    .find_party_members_conversation(party_id)
                    .await?
                {
                    Some(c) => c,
                    None => {
                        let c = Conversation::new_party_members(
                            Uuid::now_v7(),
                            party_id,
                            cmd.subject.clone(),
                        );
                        self.message_repo.create_conversation(&c).await?;
                        c
                    }
                }
            }
            RecipientType::Deal => {
                let deal_id = cmd.recipient_deal_id.ok_or(ApplicationError::Forbidden)?;
                match self.message_repo.find_deal_conversation(deal_id).await? {
                    Some(c) => c,
                    None => {
                        let c =
                            Conversation::new_deal(Uuid::now_v7(), deal_id, cmd.subject.clone());
                        self.message_repo.create_conversation(&c).await?;
                        c
                    }
                }
            }
            RecipientType::Room => {
                let room_id = cmd.recipient_room_id.ok_or(ApplicationError::Forbidden)?;
                match self.message_repo.find_room_conversation(room_id).await? {
                    Some(c) => c,
                    None => {
                        let c =
                            Conversation::new_room(Uuid::now_v7(), room_id, cmd.subject.clone());
                        self.message_repo.create_conversation(&c).await?;
                        c
                    }
                }
            }
            RecipientType::AdminBroadcast => unreachable!(),
        };
        Ok(conversation)
    }
}
