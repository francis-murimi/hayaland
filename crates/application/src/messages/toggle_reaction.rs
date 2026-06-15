use crate::errors::ApplicationError;
use crate::messages::access::is_message_visible_to_actor;
use crate::messages::dto::{MessageReactionResult, ToggleReactionCommand};
use crate::ports::RealtimePublisher;
use domain::entities::MessageReaction;
use domain::repositories::{
    ChatRoomRepository, DealRepository, MessageRepository, PartyRepository,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct ToggleReaction {
    message_repo: Arc<dyn MessageRepository>,
    party_repo: Arc<dyn PartyRepository>,
    deal_repo: Arc<dyn DealRepository>,
    room_repo: Arc<dyn ChatRoomRepository>,
    publisher: Arc<dyn RealtimePublisher>,
}

impl ToggleReaction {
    pub fn new(
        message_repo: Arc<dyn MessageRepository>,
        party_repo: Arc<dyn PartyRepository>,
        deal_repo: Arc<dyn DealRepository>,
        room_repo: Arc<dyn ChatRoomRepository>,
        publisher: Arc<dyn RealtimePublisher>,
    ) -> Self {
        Self {
            message_repo,
            party_repo,
            deal_repo,
            room_repo,
            publisher,
        }
    }

    pub async fn execute(
        &self,
        cmd: ToggleReactionCommand,
    ) -> Result<Option<MessageReactionResult>, ApplicationError> {
        let message = self
            .message_repo
            .find_message_by_id(cmd.message_id)
            .await?
            .ok_or(ApplicationError::MessageNotFound)?;

        if !is_message_visible_to_actor(
            &message,
            cmd.actor_user_id,
            cmd.actor_party_id,
            cmd.is_admin,
            self.party_repo.as_ref(),
            self.deal_repo.as_ref(),
            self.room_repo.as_ref(),
        )
        .await?
        {
            return Err(ApplicationError::Forbidden);
        }

        let reaction = MessageReaction::new(
            uuid::Uuid::now_v7(),
            cmd.message_id,
            cmd.actor_user_id,
            cmd.actor_party_id,
            cmd.reaction_type,
        );
        let added = self.message_repo.toggle_reaction(&reaction).await?;
        let reactions = self
            .message_repo
            .list_reactions_for_message(cmd.message_id)
            .await?;
        let likes = reactions
            .iter()
            .filter(|r| matches!(r.reaction_type, domain::entities::ReactionType::Like))
            .count() as i64;
        let dislikes = reactions
            .iter()
            .filter(|r| matches!(r.reaction_type, domain::entities::ReactionType::Dislike))
            .count() as i64;

        self.publisher
            .publish(crate::ports::MessageEvent::MessageReaction {
                message_id: cmd.message_id,
                user_id: cmd.actor_user_id,
                party_id: cmd.actor_party_id,
                reaction_type: cmd.reaction_type,
                total_likes: likes,
                total_dislikes: dislikes,
            })
            .await?;

        Ok(added.map(Into::into))
    }
}
