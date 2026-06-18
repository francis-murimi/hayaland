use crate::catalog::access::require_party_actor;
use crate::catalog::dto::{CreateNeedCommand, NeedResult};
use crate::catalog::mappers::map_need_to_result;
use crate::errors::ApplicationError;
use domain::entities::DealRole;
use domain::entities::{GeoPoint, Need};
use domain::repositories::{CatalogRepository, PartyRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Create a new consumer catalogue need.
#[derive(Clone)]
pub struct CreateNeed {
    catalog_repo: Arc<dyn CatalogRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl CreateNeed {
    pub fn new(
        catalog_repo: Arc<dyn CatalogRepository>,
        party_repo: Arc<dyn PartyRepository>,
    ) -> Self {
        Self {
            catalog_repo,
            party_repo,
        }
    }

    #[instrument(skip(self, cmd), fields(actor = %cmd.actor_user_id))]
    pub async fn execute(&self, cmd: CreateNeedCommand) -> Result<NeedResult, ApplicationError> {
        require_party_actor(
            &self.party_repo,
            cmd.actor_user_id,
            cmd.actor_party_id,
            cmd.is_admin,
        )
        .await?;

        let party = self
            .party_repo
            .find_by_id(cmd.actor_party_id)
            .await?
            .ok_or(ApplicationError::PartyNotFound)?;

        if !party.is_active {
            return Err(ApplicationError::PartyNotFound);
        }

        if !self
            .party_repo
            .has_role(cmd.actor_party_id, DealRole::Consumer)
            .await?
        {
            return Err(ApplicationError::Validation(vec![
                "party does not have the CONSUMER role".to_string(),
            ]));
        }

        let id = Uuid::now_v7();
        let mut need = Need::new(
            id,
            cmd.actor_party_id,
            cmd.need_category_id,
            cmd.need_description,
            cmd.required_quantity,
            cmd.quantity_unit,
        )
        .map_err(ApplicationError::from)?;

        need.quality_requirements = cmd.quality_requirements;
        need.required_by_date = cmd.required_by_date;
        need.max_budget = cmd.max_budget;
        if let Some(currency) = cmd.budget_currency {
            need.budget_currency = currency;
        }
        need.estimated_fulfillment_value = cmd.estimated_fulfillment_value;
        need.acceptable_variants = cmd.acceptable_variants;
        need.priority = cmd.priority;
        if let (Some(lat), Some(lng)) = (cmd.latitude, cmd.longitude) {
            need.location = Some(GeoPoint::new(lat, lng).map_err(ApplicationError::from)?);
        }
        need.location_address = cmd.location_address;
        need.delivery_preferences = cmd.delivery_preferences;
        need.metadata = cmd.metadata;

        self.catalog_repo.create_need(&need).await?;

        info!(%id, actor = %cmd.actor_user_id, "created need");
        Ok(map_need_to_result(need))
    }
}
