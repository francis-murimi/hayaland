use crate::catalog::access::require_catalog_owner_or_admin;
use crate::catalog::dto::{ResourceResult, UpdateResourceCommand};
use crate::catalog::mappers::map_resource_to_result;
use crate::errors::ApplicationError;
use domain::entities::GeoPoint;
use domain::repositories::{CatalogRepository, PartyRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Update a catalogue resource. Owner or admin only.
#[derive(Clone)]
pub struct UpdateResource {
    catalog_repo: Arc<dyn CatalogRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl UpdateResource {
    pub fn new(
        catalog_repo: Arc<dyn CatalogRepository>,
        party_repo: Arc<dyn PartyRepository>,
    ) -> Self {
        Self {
            catalog_repo,
            party_repo,
        }
    }

    #[instrument(skip(self, cmd), fields(resource_id = %id))]
    pub async fn execute(
        &self,
        id: Uuid,
        cmd: UpdateResourceCommand,
    ) -> Result<ResourceResult, ApplicationError> {
        let mut resource = self
            .catalog_repo
            .find_resource_by_id(id)
            .await?
            .ok_or(ApplicationError::ResourceNotFound)?;

        require_catalog_owner_or_admin(
            &self.party_repo,
            cmd.actor_user_id,
            cmd.actor_party_id,
            resource.supplier_party_id,
            cmd.is_admin,
        )
        .await?;

        if let Some(resource_type_id) = cmd.resource_type_id {
            resource.resource_type_id = resource_type_id;
        }
        if let Some(name) = cmd.resource_name {
            resource.set_name(name).map_err(ApplicationError::from)?;
        }
        if cmd.description.is_some() {
            resource.description = cmd.description;
        }
        if let Some(quantity) = cmd.quantity {
            resource
                .set_quantity(quantity)
                .map_err(ApplicationError::from)?;
        }
        if let Some(unit) = cmd.quantity_unit {
            resource.quantity_unit = unit;
        }
        if cmd.condition.is_some() {
            resource.set_condition(cmd.condition);
        }
        if let (Some(lat), Some(lng)) = (cmd.latitude, cmd.longitude) {
            resource.location = Some(GeoPoint::new(lat, lng).map_err(ApplicationError::from)?);
        } else if cmd.latitude.is_none()
            && cmd.longitude.is_none()
            && cmd.location_address.is_some()
        {
            // Allow clearing location only when both coords are omitted.
            resource.set_location(None);
        }
        if cmd.location_address.is_some() {
            resource.location_address = cmd.location_address;
        }
        if cmd.availability_start.is_some() {
            resource.availability_start = cmd.availability_start;
        }
        if cmd.availability_end.is_some() {
            resource.availability_end = cmd.availability_end;
        }
        if let Some(urls) = cmd.document_urls {
            resource.document_urls = urls;
        }
        if cmd.opportunity_cost.is_some() {
            resource.opportunity_cost = cmd.opportunity_cost;
        }
        if cmd.metadata.is_some() {
            resource.metadata = cmd.metadata;
        }
        if let Some(active) = cmd.is_active {
            resource.set_active(active);
        }

        self.catalog_repo.update_resource(&resource).await?;

        info!(%id, "updated resource");
        Ok(map_resource_to_result(resource))
    }
}
