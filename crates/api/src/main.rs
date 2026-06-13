use anyhow::Context;
use api::{run, AppState};
use application::deals::{
    CreateDeal, ExecuteTransition, GetDeal, ListDeals, SubmitDeal, UpdateDeal,
};
use application::email::resend_verification::ResendVerificationEmail;
use application::email::verify_email::VerifyEmail;
use application::parties::{
    AddPartyRole, CreateParty, GetParty, ListMyParties, ListPartyRoles, RemovePartyRole,
    SearchParties, SoftDeleteParty, UpdateParty,
};
use application::password_reset::request_password_reset::RequestPasswordReset;
use application::password_reset::reset_password::ResetPassword;
use application::roles::assign_user_roles::AssignUserRoles;
use application::roles::list_roles::ListRoles;
use application::roles::update_role_scopes::UpdateRoleScopes;
use application::users::authenticate_user::AuthenticateUser;
use application::users::create_user::CreateUser;
use application::users::deactivate_user::DeactivateUser;
use application::users::get_user::GetUser;
use application::users::list_users::ListUsers;
use application::users::update_user::UpdateUser;
use domain::repositories::{
    DealRepository, EmailVerificationRepository, PartyRepository, PasswordResetRepository,
    RoleRepository, UserRepository,
};
use infrastructure::{
    config, database,
    email::{run_worker, InMemoryEmailQueue, SmtpEmailSender},
    migrations,
    repositories::{
        PostgresDealRepository, PostgresEmailVerificationRepository, PostgresPartyRepository,
        PostgresPasswordResetRepository, PostgresRoleRepository, PostgresUserRepository,
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
    let password_reset_repo: Arc<dyn PasswordResetRepository> =
        Arc::new(PostgresPasswordResetRepository::new(pool.clone()));
    let role_repo: Arc<dyn RoleRepository> = Arc::new(PostgresRoleRepository::new(pool.clone()));
    let party_repo: Arc<dyn PartyRepository> = Arc::new(PostgresPartyRepository::new(pool.clone()));
    let deal_repo: Arc<dyn DealRepository> = Arc::new(PostgresDealRepository::new(pool));
    let hasher = Arc::new(Argon2PasswordHasher);
    let token_service = Arc::new(JwtTokenService::new(
        settings.auth.secret.expose_secret().to_string(),
        settings.auth.token_expiry_seconds,
    ));
    let email_sender =
        Arc::new(SmtpEmailSender::new(&settings.email).context("failed to create email sender")?);

    let (email_queue, receiver) = InMemoryEmailQueue::new();
    let email_queue: Arc<dyn application::email::queue::EmailQueue> = Arc::new(email_queue);
    tokio::spawn(run_worker(
        receiver,
        email_sender,
        settings.email.email_max_retries,
        settings.email.email_retry_base_delay_ms,
        settings.email.email_retry_max_delay_ms,
    ));

    let state = AppState {
        create_user: CreateUser::new(
            repo.clone(),
            verification_repo.clone(),
            email_queue.clone(),
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
            hasher.clone(),
            token_service.clone(),
        ),
        verify_email: VerifyEmail::new(repo.clone(), verification_repo.clone()),
        resend_verification_email: ResendVerificationEmail::new(
            repo.clone(),
            verification_repo,
            email_queue.clone(),
            settings.email.verification_base_url.clone(),
            settings.email.verification_token_expiry_seconds,
        ),
        request_password_reset: RequestPasswordReset::new(
            repo.clone(),
            password_reset_repo.clone(),
            email_queue.clone(),
            settings.email.verification_base_url.clone(),
            settings.email.password_reset_token_expiry_seconds,
        ),
        reset_password: ResetPassword::new(repo.clone(), password_reset_repo, hasher),
        list_roles: ListRoles::new(role_repo.clone()),
        update_role_scopes: UpdateRoleScopes::new(role_repo),
        create_party: CreateParty::new(party_repo.clone()),
        get_party: GetParty::new(party_repo.clone()),
        list_my_parties: ListMyParties::new(party_repo.clone()),
        search_parties: SearchParties::new(party_repo.clone()),
        update_party: UpdateParty::new(party_repo.clone()),
        delete_party: SoftDeleteParty::new(party_repo.clone()),
        add_party_role: AddPartyRole::new(party_repo.clone()),
        remove_party_role: RemovePartyRole::new(party_repo.clone()),
        list_party_roles: ListPartyRoles::new(party_repo.clone()),
        create_deal: CreateDeal::new(deal_repo.clone(), party_repo.clone()),
        get_deal: GetDeal::new(deal_repo.clone(), party_repo.clone()),
        list_deals: ListDeals::new(deal_repo.clone(), party_repo.clone()),
        update_deal: UpdateDeal::new(deal_repo.clone(), party_repo.clone()),
        submit_deal: SubmitDeal::new(deal_repo.clone(), party_repo.clone()),
        execute_transition: ExecuteTransition::new(deal_repo, party_repo),
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
