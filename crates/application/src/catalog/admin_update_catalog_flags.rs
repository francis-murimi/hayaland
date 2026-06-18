use crate::catalog::dto::{AdminUpdateFlagsCommand, EnhancementResult, NeedResult, ResourceResult};
use crate::catalog::mappers::{
    map_enhancement_to_result, map_need_to_result, map_resource_to_result,
};
use crate::errors::ApplicationError;
use domain::repositories::{AdminFlags, CatalogItemType, CatalogRepository};
use std::sync::Arc;
use tracing::{info, instrument};
use uuid::Uuid;

/// Admin-only use case to update catalogue item flags.
#[derive(Clone)]
pub struct AdminUpdateCatalogFlags {
    catalog_repo: Arc<dyn CatalogRepository>,
}

impl AdminUpdateCatalogFlags {
    pub fn new(catalog_repo: Arc<dyn CatalogRepository>) -> Self {
        Self { catalog_repo }
    }

    #[instrument(skip(self, cmd), fields(item_id = %id))]
    pub async fn update_resource(
        &self,
        id: Uuid,
        cmd: AdminUpdateFlagsCommand,
    ) -> Result<ResourceResult, ApplicationError> {
        require_admin(&cmd)?;

        let flags = build_flags(&cmd);
        self.catalog_repo
            .update_resource_admin_flags(id, flags)
            .await?;

        let resource = self
            .catalog_repo
            .find_resource_by_id(id)
            .await?
            .ok_or(ApplicationError::ResourceNotFound)?;

        info!(%id, "updated resource admin flags");
        Ok(map_resource_to_result(resource))
    }

    #[instrument(skip(self, cmd), fields(item_id = %id))]
    pub async fn update_need(
        &self,
        id: Uuid,
        cmd: AdminUpdateFlagsCommand,
    ) -> Result<NeedResult, ApplicationError> {
        require_admin(&cmd)?;

        let flags = build_flags(&cmd);
        self.catalog_repo.update_need_admin_flags(id, flags).await?;

        let need = self
            .catalog_repo
            .find_need_by_id(id)
            .await?
            .ok_or(ApplicationError::NeedNotFound)?;

        info!(%id, "updated need admin flags");
        Ok(map_need_to_result(need))
    }

    #[instrument(skip(self, cmd), fields(item_id = %id))]
    pub async fn update_enhancement(
        &self,
        id: Uuid,
        cmd: AdminUpdateFlagsCommand,
    ) -> Result<EnhancementResult, ApplicationError> {
        require_admin(&cmd)?;

        let flags = build_flags(&cmd);
        self.catalog_repo
            .update_enhancement_admin_flags(id, flags)
            .await?;

        let enhancement = self
            .catalog_repo
            .find_enhancement_by_id(id)
            .await?
            .ok_or(ApplicationError::EnhancementNotFound)?;

        info!(%id, "updated enhancement admin flags");
        Ok(map_enhancement_to_result(enhancement))
    }

    /// Unified dispatch by item type string.
    #[instrument(skip(self, cmd), fields(item_id = %id))]
    pub async fn execute(
        &self,
        item_type: &str,
        id: Uuid,
        cmd: AdminUpdateFlagsCommand,
    ) -> Result<AdminCatalogItemResult, ApplicationError> {
        match CatalogItemType::try_from(item_type).map_err(ApplicationError::from)? {
            CatalogItemType::Resource => self
                .update_resource(id, cmd)
                .await
                .map(AdminCatalogItemResult::Resource),
            CatalogItemType::Need => self
                .update_need(id, cmd)
                .await
                .map(AdminCatalogItemResult::Need),
            CatalogItemType::Enhancement => self
                .update_enhancement(id, cmd)
                .await
                .map(AdminCatalogItemResult::Enhancement),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AdminCatalogItemResult {
    Resource(ResourceResult),
    Need(NeedResult),
    Enhancement(EnhancementResult),
}

fn require_admin(cmd: &AdminUpdateFlagsCommand) -> Result<(), ApplicationError> {
    if !cmd.is_admin {
        return Err(ApplicationError::CatalogAccessDenied);
    }
    Ok(())
}

fn build_flags(cmd: &AdminUpdateFlagsCommand) -> AdminFlags {
    AdminFlags {
        platform_hidden: cmd.platform_hidden,
        platform_featured: cmd.platform_featured,
        admin_notes: cmd.admin_notes.clone(),
        admin_reviewed_by: Some(cmd.actor_user_id),
    }
}
