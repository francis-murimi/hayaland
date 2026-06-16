use crate::errors::ApplicationError;
use crate::notifications::dto::{NotificationTemplateRequest, NotificationTemplateResult};
use domain::entities::NotificationTemplate;
use domain::repositories::{NotificationTemplateRepository, Pagination};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct AdminListTemplates {
    repo: Arc<dyn NotificationTemplateRepository>,
}

impl AdminListTemplates {
    pub fn new(repo: Arc<dyn NotificationTemplateRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<NotificationTemplateResult>, ApplicationError> {
        let pagination = Pagination {
            limit: limit.unwrap_or(50).clamp(1, 100),
            offset: offset.unwrap_or(0).max(0),
        };
        let templates = self.repo.list(pagination).await?;
        Ok(templates
            .iter()
            .map(NotificationTemplateResult::from)
            .collect())
    }
}

#[derive(Clone)]
pub struct AdminCreateTemplate {
    repo: Arc<dyn NotificationTemplateRepository>,
}

impl AdminCreateTemplate {
    pub fn new(repo: Arc<dyn NotificationTemplateRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        req: NotificationTemplateRequest,
    ) -> Result<NotificationTemplateResult, ApplicationError> {
        let template = NotificationTemplate::new(
            Uuid::now_v7(),
            req.name,
            req.notification_type,
            req.channel,
            req.locale,
            req.subject_template,
            req.body_template,
            req.variables_schema,
        )
        .map_err(ApplicationError::from)?;

        self.repo.create(&template).await?;
        Ok(NotificationTemplateResult::from(&template))
    }
}

#[derive(Clone)]
pub struct AdminGetTemplate {
    repo: Arc<dyn NotificationTemplateRepository>,
}

impl AdminGetTemplate {
    pub fn new(repo: Arc<dyn NotificationTemplateRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, id: Uuid) -> Result<NotificationTemplateResult, ApplicationError> {
        let template = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or(ApplicationError::NotificationTemplateNotFound)?;
        Ok(NotificationTemplateResult::from(&template))
    }
}

#[derive(Clone)]
pub struct AdminUpdateTemplate {
    repo: Arc<dyn NotificationTemplateRepository>,
}

impl AdminUpdateTemplate {
    pub fn new(repo: Arc<dyn NotificationTemplateRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        id: Uuid,
        req: NotificationTemplateRequest,
    ) -> Result<NotificationTemplateResult, ApplicationError> {
        let mut template = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or(ApplicationError::NotificationTemplateNotFound)?;

        template.name = req.name;
        template.notification_type = req.notification_type;
        template.channel = req.channel;
        template.locale = req.locale;
        template.subject_template = req.subject_template;
        template.body_template = req.body_template;
        template.variables_schema = req.variables_schema;
        template.updated_at = time::OffsetDateTime::now_utc();

        self.repo.update(&template).await?;
        Ok(NotificationTemplateResult::from(&template))
    }
}

#[derive(Clone)]
pub struct AdminDeleteTemplate {
    repo: Arc<dyn NotificationTemplateRepository>,
}

impl AdminDeleteTemplate {
    pub fn new(repo: Arc<dyn NotificationTemplateRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, id: Uuid) -> Result<(), ApplicationError> {
        self.repo.delete(id).await?;
        Ok(())
    }
}
