use async_trait::async_trait;
use domain::entities::{Email, PasswordHash, User, Username};
use domain::errors::DomainError;
use domain::repositories::UserRepository;
use sqlx::{Error as SqlxError, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for PostgresUserRepository {
    async fn create(&self, user: &User) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            INSERT INTO users (id, email, username, password_hash, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            user.id,
            user.email.as_str(),
            user.username.as_str(),
            user.password_hash.as_str(),
            user.is_active,
            user.created_at,
            user.updated_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT id, email, username, password_hash, is_active, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(|r| {
            build_user(
                r.id,
                r.email,
                r.username,
                r.password_hash,
                r.is_active,
                r.created_at,
                r.updated_at,
            )
        }))
    }

    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT id, email, username, password_hash, is_active, created_at, updated_at
            FROM users
            WHERE email = $1
            "#,
            email.as_str()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(|r| {
            build_user(
                r.id,
                r.email,
                r.username,
                r.password_hash,
                r.is_active,
                r.created_at,
                r.updated_at,
            )
        }))
    }

    async fn find_by_username(&self, username: &Username) -> Result<Option<User>, DomainError> {
        let row = sqlx::query!(
            r#"
            SELECT id, email, username, password_hash, is_active, created_at, updated_at
            FROM users
            WHERE username = $1
            "#,
            username.as_str()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(row.map(|r| {
            build_user(
                r.id,
                r.email,
                r.username,
                r.password_hash,
                r.is_active,
                r.created_at,
                r.updated_at,
            )
        }))
    }

    async fn list(
        &self,
        limit: i64,
        offset: i64,
        active_only: Option<bool>,
    ) -> Result<Vec<User>, DomainError> {
        let rows = sqlx::query!(
            r#"
            SELECT id, email, username, password_hash, is_active, created_at, updated_at
            FROM users
            WHERE ($3::bool IS NULL OR is_active = $3)
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset,
            active_only
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(rows
            .into_iter()
            .map(|r| {
                build_user(
                    r.id,
                    r.email,
                    r.username,
                    r.password_hash,
                    r.is_active,
                    r.created_at,
                    r.updated_at,
                )
            })
            .collect())
    }

    async fn update(&self, user: &User) -> Result<(), DomainError> {
        sqlx::query!(
            r#"
            UPDATE users
            SET email = $1,
                username = $2,
                password_hash = $3,
                is_active = $4,
                created_at = $5,
                updated_at = $6
            WHERE id = $7
            "#,
            user.email.as_str(),
            user.username.as_str(),
            user.password_hash.as_str(),
            user.is_active,
            user.created_at,
            user.updated_at,
            user.id
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(())
    }
}

fn build_user(
    id: Uuid,
    email: String,
    username: String,
    password_hash: String,
    is_active: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
) -> User {
    let mut user = User::new(
        id,
        Email::new(&email).expect("stored email is valid"),
        Username::new(&username).expect("stored username is valid"),
        PasswordHash::new(password_hash).expect("stored hash is valid"),
    );
    user.is_active = is_active;
    user.created_at = created_at;
    user.updated_at = updated_at;
    user
}

fn map_err(err: SqlxError) -> DomainError {
    if let SqlxError::Database(db_err) = &err {
        match db_err.constraint() {
            Some("users_email_key") => return DomainError::DuplicateEmail,
            Some("users_username_key") => return DomainError::DuplicateUsername,
            _ => {}
        }
    }
    DomainError::RepositoryError(err.to_string())
}
