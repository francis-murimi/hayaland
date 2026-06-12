pub mod dto;
pub mod errors;
pub mod handlers;
pub mod routes;

use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use application::users::authenticate_user::AuthenticateUser;
use application::users::create_user::CreateUser;
use application::users::deactivate_user::DeactivateUser;
use application::users::get_user::GetUser;
use application::users::list_users::ListUsers;
use application::users::update_user::UpdateUser;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

/// All application services shared by handlers.
pub struct AppState {
    pub create_user: CreateUser,
    pub get_user: GetUser,
    pub list_users: ListUsers,
    pub update_user: UpdateUser,
    pub deactivate_user: DeactivateUser,
    pub authenticate_user: AuthenticateUser,
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
