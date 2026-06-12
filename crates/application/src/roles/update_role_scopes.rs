use crate::errors::ApplicationError;
use crate::roles::dto::RoleDto;
use domain::entities::Role;
use domain::repositories::RoleRepository;
use std::sync::Arc;
use tracing::{info, instrument};

#[derive(Clone)]
pub struct UpdateRoleScopes {
    repo: Arc<dyn RoleRepository>,
}

impl UpdateRoleScopes {
    pub fn new(repo: Arc<dyn RoleRepository>) -> Self {
        Self { repo }
    }

    #[instrument(skip(self, cmd), fields(role = %cmd.name))]
    pub async fn execute(
        &self,
        cmd: super::dto::UpdateRoleScopesCommand,
    ) -> Result<RoleDto, ApplicationError> {
        if cmd.name.trim().is_empty() {
            return Err(ApplicationError::Validation(vec![
                "role name cannot be empty".to_string(),
            ]));
        }
        if cmd.scopes.is_empty() {
            return Err(ApplicationError::Validation(vec![
                "role must have at least one scope".to_string(),
            ]));
        }

        let existing = self.repo.find_by_name(&cmd.name).await?;
        let role = match existing {
            Some(mut r) => {
                r.scopes = cmd.scopes;
                r
            }
            None => Role::new(cmd.name, cmd.scopes),
        };

        self.repo.save(&role).await?;
        info!(name = %role.name, scopes = ?role.scopes, "updated role scopes");
        Ok(RoleDto::from(role))
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

        async fn save(&self, role: &Role) -> Result<(), DomainError> {
            self.roles
                .lock()
                .unwrap()
                .insert(role.name.clone(), role.clone());
            Ok(())
        }

        async fn delete(&self, _name: &str) -> Result<(), DomainError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn creates_new_role() {
        let repo = Arc::new(FakeRoleRepo {
            roles: Mutex::new(HashMap::new()),
        });
        let dto = UpdateRoleScopes::new(repo.clone())
            .execute(crate::roles::dto::UpdateRoleScopesCommand {
                name: "manager".to_string(),
                scopes: vec!["users:read".to_string()],
            })
            .await
            .unwrap();
        assert_eq!(dto.name, "manager");
        assert!(repo.find_by_name("manager").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn updates_existing_role() {
        let repo = Arc::new(FakeRoleRepo {
            roles: Mutex::new(HashMap::from([(
                "manager".to_string(),
                Role::new("manager", vec!["users:read".to_string()]),
            )])),
        });
        let dto = UpdateRoleScopes::new(repo)
            .execute(crate::roles::dto::UpdateRoleScopesCommand {
                name: "manager".to_string(),
                scopes: vec!["users:read".to_string(), "users:write".to_string()],
            })
            .await
            .unwrap();
        assert!(dto.scopes.contains(&"users:write".to_string()));
    }
}
