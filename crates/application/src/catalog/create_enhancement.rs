use crate::catalog::access::require_party_actor;
use crate::catalog::dto::{CreateEnhancementCommand, EnhancementResult};
use crate::catalog::mappers::map_enhancement_to_result;
use crate::errors::ApplicationError;
use domain::entities::{DealRole, Enhancement};
use domain::repositories::{CatalogRepository, PartyRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Create a new enhancer catalogue enhancement.
#[derive(Clone)]
pub struct CreateEnhancement {
    catalog_repo: Arc<dyn CatalogRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl CreateEnhancement {
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
        cmd: CreateEnhancementCommand,
    ) -> Result<EnhancementResult, ApplicationError> {
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
            .has_role(cmd.actor_party_id, DealRole::Enhancer)
            .await?
        {
            return Err(ApplicationError::Validation(vec![
                "party does not have the ENHANCER role".to_string(),
            ]));
        }

        let id = Uuid::now_v7();
        let mut enhancement = Enhancement::new(
            id,
            cmd.actor_party_id,
            cmd.enhancement_type_id,
            cmd.enhancement_name,
        )
        .map_err(ApplicationError::from)?;

        enhancement.description = cmd.description;
        if let Some(q) = cmd.input_quantity {
            enhancement
                .set_input_quantity(Some(q))
                .map_err(ApplicationError::from)?;
        }
        if let Some(unit) = cmd.quantity_unit {
            enhancement.quantity_unit = Some(unit);
        }
        enhancement.estimated_input_cost = cmd.estimated_input_cost;
        if let Some(hours) = cmd.service_duration_hours {
            enhancement
                .set_service_duration_hours(Some(hours))
                .map_err(ApplicationError::from)?;
        }
        if let Some(days) = cmd.estimated_completion_days {
            enhancement
                .set_completion_days(Some(days))
                .map_err(ApplicationError::from)?;
        }
        enhancement.deliverables = cmd.deliverables;
        enhancement.prerequisites = cmd.prerequisites;
        if !cmd.skills.is_empty() {
            enhancement.skills = cmd.skills;
        }
        enhancement.certifications = cmd.certifications;
        if !cmd.equipment.is_empty() {
            enhancement.equipment = cmd.equipment;
        }
        enhancement.pricing = cmd.pricing;
        enhancement.availability = cmd.availability;
        enhancement.service_area = cmd.service_area;
        enhancement.metadata = cmd.metadata;

        self.catalog_repo.create_enhancement(&enhancement).await?;

        info!(%id, actor = %cmd.actor_user_id, "created enhancement");
        Ok(map_enhancement_to_result(enhancement))
    }
}
