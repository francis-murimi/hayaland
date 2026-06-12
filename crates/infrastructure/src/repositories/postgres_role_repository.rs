use async_trait::async_trait;
use domain::entities::Role;
use domain::errors::DomainError;
use domain::repositories::RoleRepository;
use sqlx::{Error as SqlxError, PgPool};

pub struct PostgresRoleRepository {
    pool: PgPool,
}

impl PostgresRoleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RoleRepository for PostgresRoleRepository {
    async fn find_by_name(&self, name: &str) -> Result<Option<Role>, DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT name, scopes, is_builtin
            FROM role_definitions
            WHERE name = $1
            "#,
            name
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(|r| Role {
            name: r.name,
            scopes: r.scopes,
            is_builtin: r.is_builtin,
        }))
    }

    async fn list(&self) -> Result<Vec<Role>, DomainError> {
        let rows = sqlx::query!(
            r#"
            SELECT name, scopes, is_builtin
            FROM role_definitions
            ORDER BY name
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows
            .into_iter()
            .map(|r| Role {
                name: r.name,
                scopes: r.scopes,
                is_builtin: r.is_builtin,
            })
            .collect())
    }

    async fn save(&self, role: &Role) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO role_definitions (name, scopes, is_builtin)
            VALUES ($1, $2, $3)
            ON CONFLICT (name) DO UPDATE SET
                scopes = EXCLUDED.scopes,
                is_builtin = EXCLUDED.is_builtin
            "#,
            role.name,
            &role.scopes,
            role.is_builtin
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn delete(&self, name: &str) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            DELETE FROM role_definitions
            WHERE name = $1 AND is_builtin = false
            "#,
            name
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }
}

fn map_err(err: SqlxError) -> DomainError {
    DomainError::RepositoryError(err.to_string())
}
