use crate::errors::ApplicationError;
use crate::roles::dto::RoleDto;
use domain::repositories::RoleRepository;
use std::sync::Arc;
use tracing::instrument;

#[derive(Clone)]
pub struct ListRoles {
    repo: Arc<dyn RoleRepository>,
}

impl ListRoles {
    pub fn new(repo: Arc<dyn RoleRepository>) -> Self {
        Self { repo }
    }

    #[instrument(skip(self))]
    pub async fn execute(&self) -> Result<Vec<RoleDto>, ApplicationError> {
        let roles = self
            .repo
            .list()
            .await?
            .into_iter()
            .map(RoleDto::from)
            .collect();
        Ok(roles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::entities::Role;
    use domain::errors::DomainError;
    use domain::repositories::RoleRepository;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    struct FakeRoleRepo {
        roles: Mutex<HashMap<String, Role>>,
    }

    #[async_trait::async_trait]
    impl RoleRepository for FakeRoleRepo {
        async fn find_by_name(&self, name: &str) -> Result<Option<Role>, DomainError> {
            Ok(self.roles.lock().unwrap().get(name).cloned())
        }

        async fn list(&self) -> Result<Vec<Role>, DomainError> {
            Ok(self.roles.lock().unwrap().values().cloned().collect())
        }

        async fn save(&self, _role: &Role) -> Result<(), DomainError> {
            Ok(())
        }

        async fn delete(&self, _name: &str) -> Result<(), DomainError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn lists_roles() {
        let repo = Arc::new(FakeRoleRepo {
            roles: Mutex::new(HashMap::from([(
                "admin".to_string(),
                Role::builtin("admin", vec!["users:admin".to_string()]),
            )])),
        });
        let roles = ListRoles::new(repo).execute().await.unwrap();
        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0].name, "admin");
    }
}
