use crate::catalog::access::require_party_actor;
use crate::catalog::dto::{CreateResourceCommand, ResourceResult};
use crate::catalog::mappers::map_resource_to_result;
use crate::errors::ApplicationError;
use domain::entities::DealRole;
use domain::entities::{GeoPoint, Resource};
use domain::repositories::{CatalogRepository, PartyRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Create a new supplier catalogue resource.
#[derive(Clone)]
pub struct CreateResource {
    catalog_repo: Arc<dyn CatalogRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl CreateResource {
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
    pub async fn execute(
        &self,
        cmd: CreateResourceCommand,
    ) -> Result<ResourceResult, ApplicationError> {
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
            .has_role(cmd.actor_party_id, DealRole::Supplier)
            .await?
        {
            return Err(ApplicationError::Validation(vec![
                "party does not have the SUPPLIER role".to_string(),
            ]));
        }

        let id = Uuid::now_v7();
        let mut resource = Resource::new(
            id,
            cmd.actor_party_id,
            cmd.resource_type_id,
            cmd.resource_name,
            cmd.quantity,
            cmd.quantity_unit,
        )
        .map_err(ApplicationError::from)?;

        resource.description = cmd.description;
        resource.condition = cmd.condition;
        if let (Some(lat), Some(lng)) = (cmd.latitude, cmd.longitude) {
            resource.location = Some(GeoPoint::new(lat, lng).map_err(ApplicationError::from)?);
        }
        resource.location_address = cmd.location_address;
        resource.availability_start = cmd.availability_start;
        resource.availability_end = cmd.availability_end;
        resource.document_urls = cmd.document_urls;
        resource.opportunity_cost = cmd.opportunity_cost;
        resource.metadata = cmd.metadata;

        self.catalog_repo.create_resource(&resource).await?;

        info!(%id, actor = %cmd.actor_user_id, "created resource");
        Ok(map_resource_to_result(resource))
    }
}
