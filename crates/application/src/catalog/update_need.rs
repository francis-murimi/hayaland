use crate::catalog::access::require_catalog_owner_or_admin;
use crate::catalog::dto::{NeedResult, UpdateNeedCommand};
use crate::catalog::mappers::map_need_to_result;
use crate::errors::ApplicationError;
use domain::entities::GeoPoint;
use domain::repositories::{CatalogRepository, PartyRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Update a catalogue need. Owner or admin only.
#[derive(Clone)]
pub struct UpdateNeed {
    catalog_repo: Arc<dyn CatalogRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl UpdateNeed {
    pub fn new(
        catalog_repo: Arc<dyn CatalogRepository>,
        party_repo: Arc<dyn PartyRepository>,
    ) -> Self {
        Self {
            catalog_repo,
            party_repo,
        }
    }

    #[instrument(skip(self, cmd), fields(need_id = %id))]
    pub async fn execute(
        &self,
        id: Uuid,
        cmd: UpdateNeedCommand,
    ) -> Result<NeedResult, ApplicationError> {
        let mut need = self
            .catalog_repo
            .find_need_by_id(id)
            .await?
            .ok_or(ApplicationError::NeedNotFound)?;

        require_catalog_owner_or_admin(
            &self.party_repo,
            cmd.actor_user_id,
            cmd.actor_party_id,
            need.consumer_party_id,
            cmd.is_admin,
        )
        .await?;

        if let Some(category_id) = cmd.need_category_id {
            need.need_category_id = category_id;
        }
        if let Some(description) = cmd.need_description {
            need.set_description(description)
                .map_err(ApplicationError::from)?;
        }
        if let Some(quantity) = cmd.required_quantity {
            need.set_quantity(quantity)
                .map_err(ApplicationError::from)?;
        }
        if let Some(unit) = cmd.quantity_unit {
            need.quantity_unit = unit;
        }
        if cmd.quality_requirements.is_some() {
            need.quality_requirements = cmd.quality_requirements;
        }
        if cmd.required_by_date.is_some() {
            need.required_by_date = cmd.required_by_date;
        }
        if cmd.max_budget.is_some() {
            need.max_budget = cmd.max_budget;
        }
        if let Some(currency) = cmd.budget_currency {
            need.budget_currency = currency;
        }
        if cmd.estimated_fulfillment_value.is_some() {
            need.estimated_fulfillment_value = cmd.estimated_fulfillment_value;
        }
        if cmd.acceptable_variants.is_some() {
            need.acceptable_variants = cmd.acceptable_variants;
        }
        if cmd.priority.is_some() {
            need.priority = cmd.priority;
        }
        if let (Some(lat), Some(lng)) = (cmd.latitude, cmd.longitude) {
            need.location = Some(GeoPoint::new(lat, lng).map_err(ApplicationError::from)?);
        } else if cmd.latitude.is_none()
            && cmd.longitude.is_none()
            && cmd.location_address.is_some()
        {
            need.set_location(None);
        }
        if cmd.location_address.is_some() {
            need.location_address = cmd.location_address;
        }
        if cmd.delivery_preferences.is_some() {
            need.delivery_preferences = cmd.delivery_preferences;
        }
        if cmd.metadata.is_some() {
            need.metadata = cmd.metadata;
        }
        if let Some(active) = cmd.is_active {
            need.set_active(active);
        }

        self.catalog_repo.update_need(&need).await?;

        info!(%id, "updated need");
        Ok(map_need_to_result(need))
    }
}
