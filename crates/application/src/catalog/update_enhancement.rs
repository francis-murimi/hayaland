use crate::catalog::access::require_catalog_owner_or_admin;
use crate::catalog::dto::{EnhancementResult, UpdateEnhancementCommand};
use crate::catalog::mappers::map_enhancement_to_result;
use crate::errors::ApplicationError;
use domain::repositories::{CatalogRepository, PartyRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Update a catalogue enhancement. Owner or admin only.
#[derive(Clone)]
pub struct UpdateEnhancement {
    catalog_repo: Arc<dyn CatalogRepository>,
    party_repo: Arc<dyn PartyRepository>,
}

impl UpdateEnhancement {
    pub fn new(
        catalog_repo: Arc<dyn CatalogRepository>,
        party_repo: Arc<dyn PartyRepository>,
    ) -> Self {
        Self {
            catalog_repo,
            party_repo,
        }
    }

    #[instrument(skip(self, cmd), fields(enhancement_id = %id))]
    pub async fn execute(
        &self,
        id: Uuid,
        cmd: UpdateEnhancementCommand,
    ) -> Result<EnhancementResult, ApplicationError> {
        let mut enhancement = self
            .catalog_repo
            .find_enhancement_by_id(id)
            .await?
            .ok_or(ApplicationError::EnhancementNotFound)?;

        require_catalog_owner_or_admin(
            &self.party_repo,
            cmd.actor_user_id,
            cmd.actor_party_id,
            enhancement.enhancer_party_id,
            cmd.is_admin,
        )
        .await?;

        if let Some(type_id) = cmd.enhancement_type_id {
            enhancement.enhancement_type_id = type_id;
        }
        if let Some(name) = cmd.enhancement_name {
            enhancement.set_name(name).map_err(ApplicationError::from)?;
        }
        if cmd.description.is_some() {
            enhancement.description = cmd.description;
        }
        if let Some(q) = cmd.input_quantity {
            enhancement
                .set_input_quantity(Some(q))
                .map_err(ApplicationError::from)?;
        } else if cmd.input_quantity.is_none() {
            // Explicitly clearing.
            enhancement.input_quantity = None;
        }
        if cmd.quantity_unit.is_some() {
            enhancement.quantity_unit = cmd.quantity_unit;
        }
        if cmd.estimated_input_cost.is_some() {
            enhancement.estimated_input_cost = cmd.estimated_input_cost;
        }
        if let Some(hours) = cmd.service_duration_hours {
            enhancement
                .set_service_duration_hours(Some(hours))
                .map_err(ApplicationError::from)?;
        } else if cmd.service_duration_hours.is_none() {
            enhancement.service_duration_hours = None;
        }
        if let Some(days) = cmd.estimated_completion_days {
            enhancement
                .set_completion_days(Some(days))
                .map_err(ApplicationError::from)?;
        } else if cmd.estimated_completion_days.is_none() {
            enhancement.estimated_completion_days = None;
        }
        if cmd.deliverables.is_some() {
            enhancement.deliverables = cmd.deliverables;
        }
        if cmd.prerequisites.is_some() {
            enhancement.prerequisites = cmd.prerequisites;
        }
        if let Some(skills) = cmd.skills {
            enhancement.skills = skills;
        }
        if cmd.certifications.is_some() {
            enhancement.certifications = cmd.certifications;
        }
        if let Some(equipment) = cmd.equipment {
            enhancement.equipment = equipment;
        }
        if cmd.pricing.is_some() {
            enhancement.pricing = cmd.pricing;
        }
        if cmd.availability.is_some() {
            enhancement.availability = cmd.availability;
        }
        if cmd.service_area.is_some() {
            enhancement.service_area = cmd.service_area;
        }
        if cmd.metadata.is_some() {
            enhancement.metadata = cmd.metadata;
        }
        if let Some(active) = cmd.is_active {
            enhancement.set_active(active);
        }

        self.catalog_repo.update_enhancement(&enhancement).await?;

        info!(%id, "updated enhancement");
        Ok(map_enhancement_to_result(enhancement))
    }
}
