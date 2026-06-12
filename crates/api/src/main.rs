use anyhow::Context;
use api::{run, AppState};
use application::roles::assign_user_roles::AssignUserRoles;
use application::roles::list_roles::ListRoles;
use application::roles::update_role_scopes::UpdateRoleScopes;
use application::users::authenticate_user::AuthenticateUser;
use application::users::create_user::CreateUser;
use application::users::deactivate_user::DeactivateUser;
use application::users::get_user::GetUser;
use application::users::list_users::ListUsers;
use application::users::update_user::UpdateUser;
use domain::repositories::{RoleRepository, UserRepository};
use infrastructure::{
    config, database, migrations,
    repositories::{PostgresRoleRepository, PostgresUserRepository},
    security::{Argon2PasswordHasher, JwtTokenService},
    telemetry,
};
use secrecy::ExposeSecret;
use std::net::TcpListener;
use std::sync::Arc;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let settings = config::configuration()?
        .with_database_url_fallback()
        .context("invalid configuration")?;

    telemetry::init_subscriber(&settings.log.level, settings.log.json);

    let pool = database::create_pool(&settings.database)
        .await
        .context("failed to create database pool")?;

    migrations::run_migrations(&pool)
        .await
        .context("failed to run migrations")?;

    let repo: Arc<dyn UserRepository> = Arc::new(PostgresUserRepository::new(pool.clone()));
    let role_repo: Arc<dyn RoleRepository> = Arc::new(PostgresRoleRepository::new(pool));
    let hasher = Arc::new(Argon2PasswordHasher);
    let token_service = Arc::new(JwtTokenService::new(
        settings.auth.secret.expose_secret().to_string(),
        settings.auth.token_expiry_seconds,
    ));

    let state = AppState {
        create_user: CreateUser::new(repo.clone(), hasher.clone()),
        get_user: GetUser::new(repo.clone()),
        list_users: ListUsers::new(repo.clone()),
        update_user: UpdateUser::new(repo.clone()),
        assign_user_roles: AssignUserRoles::new(repo.clone()),
        deactivate_user: DeactivateUser::new(repo.clone()),
        authenticate_user: AuthenticateUser::new(
            repo,
            role_repo.clone(),
            hasher,
            token_service.clone(),
        ),
        list_roles: ListRoles::new(role_repo.clone()),
        update_role_scopes: UpdateRoleScopes::new(role_repo),
        token_validator: token_service,
    };

    let address = format!("{}:{}", settings.server.host, settings.server.port);
    let listener = TcpListener::bind(&address).context("failed to bind port")?;

    tracing::info!(%address, "server listening");
    run(listener, state)?
        .await
        .context("server terminated unexpectedly")?;

    Ok(())
}
