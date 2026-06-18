use crate::catalog::access::require_party_actor;
use crate::catalog::dto::{
    deal_role_for_item_type, BindCatalogItemToDealCommand, DealBoundCatalogItemResult,
};
use crate::errors::ApplicationError;
use domain::entities::{DealRole, DealStatus, Enhancement, Need, Resource};
use domain::repositories::{CatalogItemType, CatalogRepository, DealRepository, PartyRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Bind an existing catalogue item to a deal, creating a deal-bound copy.
#[derive(Clone)]
pub struct BindCatalogItemToDeal {
    catalog_repo: Arc<dyn CatalogRepository>,
    deal_repo: Arc<dyn DealRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl BindCatalogItemToDeal {
    pub fn new(
        catalog_repo: Arc<dyn CatalogRepository>,
        deal_repo: Arc<dyn DealRepository>,
        party_repo: Arc<dyn PartyRepository>,
    ) -> Self {
        Self {
            catalog_repo,
            deal_repo,
            party_repo,
        }
    }

    #[instrument(skip(self, cmd), fields(item_id = %cmd.item_id, deal_id = %cmd.deal_id))]
    pub async fn execute(
        &self,
        cmd: BindCatalogItemToDealCommand,
    ) -> Result<DealBoundCatalogItemResult, ApplicationError> {
        require_party_actor(
            &self.party_repo,
            cmd.actor_user_id,
            cmd.actor_party_id,
            cmd.is_admin,
        )
        .await?;

        let role = deal_role_for_item_type(&cmd.item_type)
            .ok_or_else(|| ApplicationError::Validation(vec!["invalid item type".to_string()]))?;

        let deal = self
            .deal_repo
            .find_by_id(cmd.deal_id)
            .await?
            .ok_or(ApplicationError::DealNotFound)?;

        if !is_deal_editable(deal.deal_status) {
            return Err(ApplicationError::Validation(vec![
                "deal is not editable".to_string()
            ]));
        }

        let participations = self
            .deal_repo
            .find_participations_by_deal(cmd.deal_id)
            .await?;

        let actor_participation = participations
            .iter()
            .find(|p| p.party_id == cmd.actor_party_id && p.role == role)
            .ok_or(ApplicationError::Validation(vec![format!(
                "actor is not a {role:?} participant in this deal"
            )]))?;

        if actor_participation.participation_status.as_str() == "WITHDRAWN" {
            return Err(ApplicationError::Validation(vec![
                "actor has withdrawn from this deal".to_string(),
            ]));
        }

        let catalog_item_id = match role {
            DealRole::Supplier => {
                let resource = self
                    .catalog_repo
                    .find_resource_by_id(cmd.item_id)
                    .await?
                    .ok_or(ApplicationError::ResourceNotFound)?;
                if resource.supplier_party_id != cmd.actor_party_id {
                    return Err(ApplicationError::CatalogAccessDenied);
                }
                if !resource.is_active || resource.platform_hidden {
                    return Err(ApplicationError::Validation(vec![
                        "resource is not available for binding".to_string(),
                    ]));
                }
                self.create_deal_bound_resource(&resource, cmd.deal_id)
                    .await?
            }
            DealRole::Consumer => {
                let need = self
                    .catalog_repo
                    .find_need_by_id(cmd.item_id)
                    .await?
                    .ok_or(ApplicationError::NeedNotFound)?;
                if need.consumer_party_id != cmd.actor_party_id {
                    return Err(ApplicationError::CatalogAccessDenied);
                }
                if !need.is_active || need.platform_hidden {
                    return Err(ApplicationError::Validation(vec![
                        "need is not available for binding".to_string(),
                    ]));
                }
                self.create_deal_bound_need(&need, cmd.deal_id).await?
            }
            DealRole::Enhancer => {
                let enhancement = self
                    .catalog_repo
                    .find_enhancement_by_id(cmd.item_id)
                    .await?
                    .ok_or(ApplicationError::EnhancementNotFound)?;
                if enhancement.enhancer_party_id != cmd.actor_party_id {
                    return Err(ApplicationError::CatalogAccessDenied);
                }
                if !enhancement.is_active || enhancement.platform_hidden {
                    return Err(ApplicationError::Validation(vec![
                        "enhancement is not available for binding".to_string(),
                    ]));
                }
                self.create_deal_bound_enhancement(&enhancement, cmd.deal_id)
                    .await?
            }
        };

        let item_type = match role {
            DealRole::Supplier => CatalogItemType::Resource,
            DealRole::Consumer => CatalogItemType::Need,
            DealRole::Enhancer => CatalogItemType::Enhancement,
        };
        self.catalog_repo
            .increment_deal_count(item_type, cmd.item_id)
            .await?;

        info!(catalog_item_id = %catalog_item_id, "bound catalog item to deal");
        Ok(DealBoundCatalogItemResult {
            item_type: cmd.item_type.to_ascii_uppercase(),
            item_id: catalog_item_id,
            deal_id: cmd.deal_id,
            catalog_item_id: cmd.item_id,
        })
    }

    async fn create_deal_bound_resource(
        &self,
        resource: &Resource,
        deal_id: Uuid,
    ) -> Result<Uuid, ApplicationError> {
        let mut copy = resource.clone();
        let id = Uuid::now_v7();
        copy.id = id;
        copy.deal_id = Some(deal_id);
        copy.catalog_item_id = Some(resource.id);
        self.catalog_repo.create_resource(&copy).await?;
        Ok(id)
    }

    async fn create_deal_bound_need(
        &self,
        need: &Need,
        deal_id: Uuid,
    ) -> Result<Uuid, ApplicationError> {
        let mut copy = need.clone();
        let id = Uuid::now_v7();
        copy.id = id;
        copy.deal_id = Some(deal_id);
        copy.catalog_item_id = Some(need.id);
        self.catalog_repo.create_need(&copy).await?;
        Ok(id)
    }

    async fn create_deal_bound_enhancement(
        &self,
        enhancement: &Enhancement,
        deal_id: Uuid,
    ) -> Result<Uuid, ApplicationError> {
        let mut copy = enhancement.clone();
        let id = Uuid::now_v7();
        copy.id = id;
        copy.deal_id = Some(deal_id);
        copy.catalog_item_id = Some(enhancement.id);
        self.catalog_repo.create_enhancement(&copy).await?;
        Ok(id)
    }
}

fn is_deal_editable(status: DealStatus) -> bool {
    matches!(
        status,
        DealStatus::Draft
            | DealStatus::Suggested
            | DealStatus::PendingReview
            | DealStatus::Negotiating
            | DealStatus::AwaitingParty
            | DealStatus::TermsLocked
    )
}
