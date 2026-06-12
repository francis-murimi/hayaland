use anyhow::Context;
use api::{run, AppState};
use application::email::resend_verification::ResendVerificationEmail;
use application::email::verify_email::VerifyEmail;
use application::roles::assign_user_roles::AssignUserRoles;
use application::roles::list_roles::ListRoles;
use application::roles::update_role_scopes::UpdateRoleScopes;
use application::users::authenticate_user::AuthenticateUser;
use application::users::create_user::CreateUser;
use application::users::deactivate_user::DeactivateUser;
use application::users::get_user::GetUser;
use application::users::list_users::ListUsers;
use application::users::update_user::UpdateUser;
use domain::repositories::{EmailVerificationRepository, RoleRepository, UserRepository};
use infrastructure::{
    config, database,
    email::SmtpEmailSender,
    migrations,
    repositories::{
        PostgresEmailVerificationRepository, PostgresRoleRepository, PostgresUserRepository,
    },
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
        .context("failed to run database migrations")?;

    let repo: Arc<dyn UserRepository> = Arc::new(PostgresUserRepository::new(pool.clone()));
    let verification_repo: Arc<dyn EmailVerificationRepository> =
        Arc::new(PostgresEmailVerificationRepository::new(pool.clone()));
    let role_repo: Arc<dyn RoleRepository> = Arc::new(PostgresRoleRepository::new(pool));
    let hasher = Arc::new(Argon2PasswordHasher);
    let token_service = Arc::new(JwtTokenService::new(
        settings.auth.secret.expose_secret().to_string(),
        settings.auth.token_expiry_seconds,
    ));
    let email_sender =
        Arc::new(SmtpEmailSender::new(&settings.email).context("failed to create email sender")?);

    let state = AppState {
        create_user: CreateUser::new(
            repo.clone(),
            verification_repo.clone(),
            email_sender.clone(),
            hasher.clone(),
            settings.email.verification_base_url.clone(),
            settings.email.verification_token_expiry_seconds,
        ),
        get_user: GetUser::new(repo.clone()),
        list_users: ListUsers::new(repo.clone()),
        update_user: UpdateUser::new(repo.clone()),
        assign_user_roles: AssignUserRoles::new(repo.clone()),
        deactivate_user: DeactivateUser::new(repo.clone()),
        authenticate_user: AuthenticateUser::new(
            repo.clone(),
            role_repo.clone(),
            hasher,
            token_service.clone(),
        ),
        verify_email: VerifyEmail::new(repo.clone(), verification_repo.clone()),
        resend_verification_email: ResendVerificationEmail::new(
            repo,
            verification_repo,
            email_sender,
            settings.email.verification_base_url,
            settings.email.verification_token_expiry_seconds,
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
