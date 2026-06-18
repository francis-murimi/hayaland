use crate::catalog::dto::{ContactCatalogOwnerCommand, ContactCatalogOwnerResult};
use crate::errors::ApplicationError;
use domain::entities::{Conversation, Message, MessageType, RecipientType};
use domain::repositories::{
    CatalogItemType, CatalogRepository, MessageRepository, PartyRepository,
};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Contact the owner of a catalogue item. Creates a direct-party conversation and sends an
/// initial message. Notification delivery is left as a future TODO.
#[derive(Clone)]
pub struct ContactCatalogOwner {
    catalog_repo: Arc<dyn CatalogRepository>,
    party_repo: Arc<dyn PartyRepository>,
    message_repo: Arc<dyn MessageRepository>,
}

impl ContactCatalogOwner {
    pub fn new(
        catalog_repo: Arc<dyn CatalogRepository>,
        party_repo: Arc<dyn PartyRepository>,
        message_repo: Arc<dyn MessageRepository>,
    ) -> Self {
        Self {
            catalog_repo,
            party_repo,
            message_repo,
        }
    }

    #[instrument(skip(self, cmd), fields(item_id = %cmd.item_id))]
    pub async fn execute(
        &self,
        cmd: ContactCatalogOwnerCommand,
    ) -> Result<ContactCatalogOwnerResult, ApplicationError> {
        if !cmd.is_admin {
            let is_member = self
                .party_repo
                .is_user_member_of_party(cmd.actor_user_id, cmd.actor_party_id)
                .await?;
            if !is_member {
                return Err(ApplicationError::Forbidden);
            }
        }

        let item_type =
            CatalogItemType::try_from(cmd.item_type.as_str()).map_err(ApplicationError::from)?;

        let (owner_party_id, can_contact) = match item_type {
            CatalogItemType::Resource => {
                let resource = self
                    .catalog_repo
                    .find_resource_by_id(cmd.item_id)
                    .await?
                    .ok_or(ApplicationError::ResourceNotFound)?;
                (resource.supplier_party_id, resource.can_contact_owner(true))
            }
            CatalogItemType::Need => {
                let need = self
                    .catalog_repo
                    .find_need_by_id(cmd.item_id)
                    .await?
                    .ok_or(ApplicationError::NeedNotFound)?;
                (need.consumer_party_id, need.can_contact_owner(true))
            }
            CatalogItemType::Enhancement => {
                let enhancement = self
                    .catalog_repo
                    .find_enhancement_by_id(cmd.item_id)
                    .await?
                    .ok_or(ApplicationError::EnhancementNotFound)?;
                (
                    enhancement.enhancer_party_id,
                    enhancement.can_contact_owner(true),
                )
            }
        };

        if cmd.actor_party_id == owner_party_id {
            return Err(ApplicationError::Validation(vec![
                "cannot contact yourself".to_string(),
            ]));
        }

        let owner_party = self
            .party_repo
            .find_by_id(owner_party_id)
            .await?
            .ok_or(ApplicationError::PartyNotFound)?;

        if !can_contact || !owner_party.accepts_catalog_inquiries {
            return Err(ApplicationError::Validation(vec![
                "owner is not accepting inquiries for this item".to_string(),
            ]));
        }

        let conversation = self
            .find_or_create_direct_party_conversation(cmd.actor_party_id, owner_party_id)
            .await?;

        let message = Message::new(
            Uuid::now_v7(),
            conversation.id,
            cmd.actor_user_id,
            Some(cmd.actor_party_id),
            RecipientType::Party,
            None,
            Some(owner_party_id),
            None,
            None,
            MessageType::Text,
            Some("Catalogue inquiry".to_string()),
            cmd.message,
            vec![],
            None,
        )
        .map_err(ApplicationError::from)?;

        self.message_repo.create_message(&message).await?;
        self.message_repo
            .touch_conversation(conversation.id, message.created_at)
            .await?;

        // TODO: send notification to owner party members.
        info!(conversation_id = %conversation.id, message_id = %message.id, "sent catalog inquiry");

        Ok(ContactCatalogOwnerResult {
            conversation_id: conversation.id,
            message_id: message.id,
        })
    }

    async fn find_or_create_direct_party_conversation(
        &self,
        party_a_id: Uuid,
        party_b_id: Uuid,
    ) -> Result<Conversation, ApplicationError> {
        let (a, b) = if party_a_id < party_b_id {
            (party_a_id, party_b_id)
        } else {
            (party_b_id, party_a_id)
        };

        match self
            .message_repo
            .find_direct_party_conversation(a, b)
            .await?
        {
            Some(c) => Ok(c),
            None => {
                let c = Conversation::new_direct_party(Uuid::now_v7(), a, b);
                self.message_repo.create_conversation(&c).await?;
                Ok(c)
            }
        }
    }
}
