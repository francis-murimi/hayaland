use crate::errors::ApplicationError;
use crate::messages::access::is_admin_scope;
use crate::messages::dto::{
    to_message_result, AdminBroadcastCommand, BroadcastTarget, MessageResult,
};
use crate::ports::{EncryptionService, RealtimePublisher};
use domain::entities::{Conversation, ConversationType, Message, MessageType, RecipientType};
use domain::repositories::{MessageRepository, PartyRepository, UserRepository};
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

const BROADCAST_CONVERSATION_ID: Uuid =
    uuid::Uuid::from_u128(0x00000001_0001_0001_0001_000000000001);

#[derive(Clone)]
pub struct AdminBroadcast {
    message_repo: Arc<dyn MessageRepository>,
    user_repo: Arc<dyn UserRepository>,
    party_repo: Arc<dyn PartyRepository>,
    encryption: Arc<dyn EncryptionService>,
    publisher: Arc<dyn RealtimePublisher>,
}

impl AdminBroadcast {
    pub fn new(
        message_repo: Arc<dyn MessageRepository>,
        user_repo: Arc<dyn UserRepository>,
        party_repo: Arc<dyn PartyRepository>,
        encryption: Arc<dyn EncryptionService>,
        publisher: Arc<dyn RealtimePublisher>,
    ) -> Self {
        Self {
            message_repo,
            user_repo,
            party_repo,
            encryption,
            publisher,
        }
    }

    pub async fn execute(
        &self,
        cmd: AdminBroadcastCommand,
    ) -> Result<Vec<MessageResult>, ApplicationError> {
        if !is_admin_scope(&cmd.scopes) {
            return Err(ApplicationError::Forbidden);
        }
        if cmd.content.trim().is_empty() {
            return Err(ApplicationError::Validation(vec![
                "broadcast content cannot be empty".to_string(),
            ]));
        }

        let conversation = match self
            .message_repo
            .find_conversation_by_id(BROADCAST_CONVERSATION_ID)
            .await?
        {
            Some(c) => c,
            None => {
                let now = OffsetDateTime::now_utc();
                let c = Conversation {
                    id: BROADCAST_CONVERSATION_ID,
                    conversation_type: ConversationType::AdminBroadcast,
                    user_a_id: None,
                    user_b_id: None,
                    party_a_id: None,
                    party_b_id: None,
                    party_id: None,
                    deal_id: None,
                    room_id: None,
                    title: Some("Admin Broadcast".to_string()),
                    last_message_at: now,
                    created_at: now,
                };
                self.message_repo.create_conversation(&c).await?;
                c
            }
        };

        let encrypted = self.encryption.encrypt(&cmd.content).await?;
        let mut results = Vec::new();
        let mut recipients: Vec<(Option<Uuid>, Option<Uuid>)> = Vec::new();

        match cmd.target {
            BroadcastTarget::AllUsers => {
                let users = self.user_repo.list(i64::MAX, 0, Some(true)).await?;
                for u in users {
                    recipients.push((Some(u.id), None));
                }
            }
            BroadcastTarget::AllParties => {
                let parties = self
                    .party_repo
                    .list(&domain::repositories::PartySearchCriteria::default())
                    .await?;
                for p in parties {
                    recipients.push((None, Some(p.id)));
                }
            }
            BroadcastTarget::AllUsersAndParties => {
                let users = self.user_repo.list(i64::MAX, 0, Some(true)).await?;
                for u in users {
                    recipients.push((Some(u.id), None));
                }
                let parties = self
                    .party_repo
                    .list(&domain::repositories::PartySearchCriteria::default())
                    .await?;
                for p in parties {
                    recipients.push((None, Some(p.id)));
                }
            }
        }

        for (user_id, party_id) in recipients {
            let message = Message::new(
                Uuid::now_v7(),
                conversation.id,
                cmd.actor_user_id,
                None,
                RecipientType::AdminBroadcast,
                user_id,
                party_id,
                None,
                None,
                MessageType::AdminBroadcast,
                cmd.subject.clone(),
                encrypted.clone(),
                vec![],
                None,
            )
            .map_err(ApplicationError::from)?;

            self.message_repo.create_message(&message).await?;
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
            results.push(to_message_result(message, &self.encryption).await?);
        }

        self.message_repo
            .touch_conversation(conversation.id, OffsetDateTime::now_utc())
            .await?;

        Ok(results)
    }
}
