use crate::errors::ApplicationError;
use crate::matching::dto::{MatchResponseAction, RespondToMatchCommand};
use domain::entities::{MatchStatus, MatchSuggestion};
use domain::repositories::{MatchRepository, PartyRepository};
use std::sync::Arc;
use tracing::{info, instrument};

/// Respond to a match suggestion as one of the participating parties.
#[derive(Clone)]
pub struct RespondToMatch {
    match_repo: Arc<dyn MatchRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl RespondToMatch {
    pub fn new(match_repo: Arc<dyn MatchRepository>, party_repo: Arc<dyn PartyRepository>) -> Self {
        Self {
            match_repo,
            party_repo,
        }
    }

    #[instrument(skip(self, cmd))]
    pub async fn execute(&self, cmd: RespondToMatchCommand) -> Result<(), ApplicationError> {
        let suggestion = self
            .match_repo
            .find_by_id(cmd.match_suggestion_id)
            .await?
            .ok_or(ApplicationError::NotFound)?;

        self.verify_participant(&cmd, &suggestion).await?;
        suggestion.can_respond().map_err(ApplicationError::from)?;

        match cmd.response {
            MatchResponseAction::Accept => {
                self.match_repo
                    .update_status(cmd.match_suggestion_id, MatchStatus::Accepted, cmd.notes)
                    .await?;
                info!(
                    match_id = %cmd.match_suggestion_id,
                    party_id = %cmd.actor_party_id,
                    "match suggestion accepted"
                );
            }
            MatchResponseAction::Decline => {
                self.match_repo
                    .update_status(cmd.match_suggestion_id, MatchStatus::Declined, cmd.notes)
                    .await?;
                info!(
                    match_id = %cmd.match_suggestion_id,
                    party_id = %cmd.actor_party_id,
                    "match suggestion declined"
                );
            }
            MatchResponseAction::CounterPropose => {
                self.match_repo
                    .update_counter_proposal(cmd.match_suggestion_id, cmd.counter_value, cmd.notes)
                    .await?;
                info!(
                    match_id = %cmd.match_suggestion_id,
                    party_id = %cmd.actor_party_id,
                    "match suggestion counter-proposed"
                );
            }
        }

        Ok(())
    }

    async fn verify_participant(
        &self,
        cmd: &RespondToMatchCommand,
        suggestion: &MatchSuggestion,
    ) -> Result<(), ApplicationError> {
        let is_member = self
            .party_repo
            .is_user_member_of_party(cmd.actor_user_id, cmd.actor_party_id)
            .await?;
        if !is_member {
            return Err(ApplicationError::Forbidden);
        }

        if !suggestion.is_participant(cmd.actor_party_id) {
            return Err(ApplicationError::from(
                domain::errors::DomainError::PartyNotMatchParticipant,
            ));
        }

        Ok(())
    }
}
