pub mod dto;
pub mod errors;
pub mod handlers;
pub mod middleware;
pub mod routes;

use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
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
use application::users::token::TokenVerifier;
use application::users::update_user::UpdateUser;
use std::net::TcpListener;
use std::sync::Arc;
use tracing_actix_web::TracingLogger;

/// All application services shared by handlers.
#[derive(Clone)]
pub struct AppState {
    pub create_user: CreateUser,
    pub get_user: GetUser,
    pub list_users: ListUsers,
    pub update_user: UpdateUser,
    pub assign_user_roles: AssignUserRoles,
    pub deactivate_user: DeactivateUser,
    pub authenticate_user: AuthenticateUser,
    pub verify_email: VerifyEmail,
    pub resend_verification_email: ResendVerificationEmail,
    pub request_password_reset: RequestPasswordReset,
    pub reset_password: ResetPassword,
    pub list_roles: ListRoles,
    pub update_role_scopes: UpdateRoleScopes,
    pub create_party: CreateParty,
    pub get_party: GetParty,
    pub list_my_parties: ListMyParties,
    pub search_parties: SearchParties,
    pub update_party: UpdateParty,
    pub delete_party: SoftDeleteParty,
    pub add_party_role: AddPartyRole,
    pub remove_party_role: RemovePartyRole,
    pub list_party_roles: ListPartyRoles,
    pub create_deal: CreateDeal,
    pub get_deal: GetDeal,
    pub list_deals: ListDeals,
    pub update_deal: UpdateDeal,
    pub submit_deal: SubmitDeal,
    pub execute_transition: ExecuteTransition,
    pub token_validator: Arc<dyn TokenVerifier>,
}

/// Factory for the Actix HTTP server.
pub fn run(listener: TcpListener, state: AppState) -> Result<Server, std::io::Error> {
    let state = web::Data::new(state);

    let server = HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .configure(routes::configure)
            .wrap(TracingLogger::default())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
