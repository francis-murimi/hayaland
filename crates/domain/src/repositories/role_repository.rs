use crate::entities::Role;
use crate::errors::DomainError;
use async_trait::async_trait;

#[async_trait]
pub trait RoleRepository: Send + Sync {
    async fn find_by_name(&self, name: &str) -> Result<Option<Role>, DomainError>;
    async fn list(&self) -> Result<Vec<Role>, DomainError>;
    async fn save(&self, role: &Role) -> Result<(), DomainError>;
    async fn delete(&self, name: &str) -> Result<(), DomainError>;
}
