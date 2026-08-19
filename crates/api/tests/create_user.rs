use actix_web::http::StatusCode;
use actix_web::{http::header, test, web::Data};
use api::routes;
use api::AppState;
use application::deals::dto::SetValueDistributionCommand;
use domain::entities::PartyType;
use domain::entities::{
    DealRole, DealStatus, DistributionModel, Email, PasswordHash, User, Username,
};
use domain::repositories::UserRepository;
use rust_decimal::Decimal;
use uuid::Uuid;

mod fakes;
use fakes::*;

#[actix_rt::test]
async fn health_returns_ok() {
    init_test_tracing();
    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(test_fixtures().state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get().uri("/api/v1/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_rt::test]
async fn create_user_returns_201() {
    init_test_tracing();
    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(test_fixtures().state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/users")
        .set_json(serde_json::json!({
            "email": "test@example.com",
            "username": "testuser",
            "password": "password123"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("id").is_some());
}

#[actix_rt::test]
async fn create_user_returns_400_for_invalid_input() {
    init_test_tracing();
    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(test_fixtures().state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/users")
        .set_json(serde_json::json!({
            "email": "not-an-email",
            "username": "ab",
            "password": "short"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn get_user_returns_401_when_unauthenticated() {
    init_test_tracing();
    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(test_fixtures().state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/users/{}", Uuid::nil()))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn get_user_returns_401_for_invalid_token() {
    init_test_tracing();
    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(test_fixtures().state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/users/{}", Uuid::nil()))
        .insert_header((header::AUTHORIZATION, "Bearer not-a-token"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn get_user_returns_200_when_authenticated() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let created = fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "get@example.com".to_string(),
            username: "getuser".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&fixtures, "get@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/users/{}", created.id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_rt::test]
async fn get_user_returns_404_when_missing() {
    init_test_tracing();
    let fixtures = test_fixtures();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "missing@example.com".to_string(),
            username: "missing".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&fixtures, "missing@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/users/{}", Uuid::nil()))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_rt::test]
async fn list_users_returns_401_when_unauthenticated() {
    init_test_tracing();
    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(test_fixtures().state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/users?page=1&per_page=10")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn list_users_returns_200_when_authenticated() {
    init_test_tracing();
    let fixtures = test_fixtures();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "list@example.com".to_string(),
            username: "listuser".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&fixtures, "list@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/users?page=1&per_page=10")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_rt::test]
async fn update_user_returns_200_for_owner() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let created = fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "update@example.com".to_string(),
            username: "updateuser".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&fixtures, "update@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/users/{}", created.id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(serde_json::json!({ "username": "updateduser" }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_rt::test]
async fn update_user_returns_403_for_non_owner() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner = fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "owner@example.com".to_string(),
            username: "owner".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "other@example.com".to_string(),
            username: "other".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&fixtures, "other@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/users/{}", owner.id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(serde_json::json!({ "username": "hacked" }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn deactivate_user_returns_200_and_blocks_login() {
    init_test_tracing();
    let fixtures = test_fixtures();
    // First user becomes the protected admin.
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "admin@example.com".to_string(),
            username: "admin".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let created = fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "inactive@example.com".to_string(),
            username: "inactive".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&fixtures, "inactive@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let deactivate = test::TestRequest::delete()
        .uri(&format!("/api/v1/users/{}", created.id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, deactivate).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let login = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "inactive@example.com",
            "password": "password123"
        }))
        .to_request();
    let resp = test::call_service(&app, login).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn deactivate_admin_returns_403() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let admin = fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "admin@example.com".to_string(),
            username: "admin".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&fixtures, "admin@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/users/{}", admin.id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn login_returns_401_for_unverified_user() {
    init_test_tracing();
    let fixtures = test_fixtures();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "unverified@example.com".to_string(),
            username: "unverified".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "unverified@example.com",
            "password": "password123"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn login_returns_200_for_verified_user() {
    init_test_tracing();
    let fixtures = test_fixtures();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "login@example.com".to_string(),
            username: "loginuser".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();

    let token = login(&fixtures, "login@example.com").await;
    assert!(token.starts_with("token-"));
}

#[actix_rt::test]
async fn login_returns_401_for_invalid_credentials() {
    init_test_tracing();
    let fixtures = test_fixtures();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "bad@example.com".to_string(),
            username: "baduser".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let _token = login(&fixtures, "bad@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "bad@example.com",
            "password": "wrongpassword"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn verify_email_activates_user() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let created = fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "verify@example.com".to_string(),
            username: "verify".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();

    let token = extract_token_for_email(&fixtures.queue, "verify@example.com");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/auth/verify-email?token={token}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "verified");
    assert_eq!(body["user_id"], created.id.to_string());

    let user = fixtures.repo.find_by_id(created.id).await.unwrap().unwrap();
    assert!(user.is_active);
}

#[actix_rt::test]
async fn verify_email_rejects_invalid_token() {
    init_test_tracing();
    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(test_fixtures().state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/verify-email?token=not-a-token")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn resend_verification_returns_202() {
    init_test_tracing();
    let fixtures = test_fixtures();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "resend@example.com".to_string(),
            username: "resend".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/resend-verification")
        .set_json(serde_json::json!({ "email": "resend@example.com" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
}

#[actix_rt::test]
async fn admin_can_list_roles() {
    init_test_tracing();
    let fixtures = test_fixtures();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "admin@example.com".to_string(),
            username: "admin".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&fixtures, "admin@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/roles")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let roles = body["roles"].as_array().unwrap();
    assert!(roles.iter().any(|r| r["name"] == "admin"));
}

#[actix_rt::test]
async fn admin_can_update_role_scopes() {
    init_test_tracing();
    let fixtures = test_fixtures();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "admin@example.com".to_string(),
            username: "admin".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&fixtures, "admin@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::put()
        .uri("/api/v1/roles/moderator")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(serde_json::json!({ "scopes": ["users:read", "users:write"] }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["name"], "moderator");
    assert!(body["scopes"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("users:read")));
}

#[actix_rt::test]
async fn admin_can_assign_roles_to_user() {
    init_test_tracing();
    let fixtures = test_fixtures();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "admin@example.com".to_string(),
            username: "admin".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let target = fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "target@example.com".to_string(),
            username: "target".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&fixtures, "admin@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/users/{}/roles", target.id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(serde_json::json!({ "roles": ["admin"] }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["roles"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("admin")));
}

#[actix_rt::test]
async fn non_admin_cannot_assign_roles() {
    init_test_tracing();
    let fixtures = test_fixtures();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "admin@example.com".to_string(),
            username: "admin".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let target = fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "target@example.com".to_string(),
            username: "target".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&fixtures, "target@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/users/{}/roles", target.id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(serde_json::json!({ "roles": ["admin"] }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn forgot_password_returns_202_for_existing_user() {
    init_test_tracing();
    let fixtures = test_fixtures();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "forgot@example.com".to_string(),
            username: "forgot".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/forgot-password")
        .set_json(serde_json::json!({ "email": "forgot@example.com" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
}

#[actix_rt::test]
async fn forgot_password_returns_202_for_unknown_email() {
    init_test_tracing();
    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(test_fixtures().state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/forgot-password")
        .set_json(serde_json::json!({ "email": "unknown@example.com" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
}

#[actix_rt::test]
async fn reset_password_changes_password_and_allows_login() {
    init_test_tracing();
    let fixtures = test_fixtures();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "reset@example.com".to_string(),
            username: "reset".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state.clone()))
            .configure(routes::configure),
    )
    .await;

    let forgot = test::TestRequest::post()
        .uri("/api/v1/auth/forgot-password")
        .set_json(serde_json::json!({ "email": "reset@example.com" }))
        .to_request();
    let resp = test::call_service(&app, forgot).await;
    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    let token = extract_reset_token_for_email(&fixtures.queue, "reset@example.com");

    let reset = test::TestRequest::post()
        .uri("/api/v1/auth/reset-password")
        .set_json(serde_json::json!({ "token": token, "password": "newpassword123" }))
        .to_request();
    let resp = test::call_service(&app, reset).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let login = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "reset@example.com",
            "password": "newpassword123"
        }))
        .to_request();
    let resp = test::call_service(&app, login).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_rt::test]
async fn reset_password_returns_400_for_invalid_token() {
    init_test_tracing();
    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(test_fixtures().state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/reset-password")
        .set_json(serde_json::json!({ "token": "not-a-token", "password": "newpassword123" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn reset_password_returns_400_for_short_password() {
    init_test_tracing();
    let fixtures = test_fixtures();
    fixtures
        .state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "short@example.com".to_string(),
            username: "short".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let forgot = test::TestRequest::post()
        .uri("/api/v1/auth/forgot-password")
        .set_json(serde_json::json!({ "email": "short@example.com" }))
        .to_request();
    let resp = test::call_service(&app, forgot).await;
    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    let token = extract_reset_token_for_email(&fixtures.queue, "short@example.com");

    let reset = test::TestRequest::post()
        .uri("/api/v1/auth/reset-password")
        .set_json(serde_json::json!({ "token": token, "password": "short" }))
        .to_request();
    let resp = test::call_service(&app, reset).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ============================================================================
// Party handler tests
// ============================================================================

fn seed_user(fixtures: &TestFixtures, email: &str, role: &str) -> Uuid {
    let id = Uuid::now_v7();
    let username = email.split('@').next().unwrap();
    let mut user = User::new(
        id,
        Email::new(email).unwrap(),
        Username::new(username).unwrap(),
        PasswordHash::new("hash".to_string()).unwrap(),
    );
    user.is_active = true;
    user.roles = vec![role.to_string()];
    fixtures.repo.users.lock().unwrap().insert(id, user);
    id
}

fn bearer(id: Uuid) -> String {
    format!("Bearer token-{id}")
}

macro_rules! create_party {
    ($app:expr, $token:expr, $email:expr) => {{
        let req = test::TestRequest::post()
            .uri("/api/v1/parties")
            .insert_header((header::AUTHORIZATION, $token.to_string()))
            .set_json(serde_json::json!({
                "display_name": "Green Acres Farm",
                "email": $email,
                "party_type": "ORGANIZATION",
                "roles": []
            }))
            .to_request();
        let resp = test::call_service($app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body: serde_json::Value = test::read_body_json(resp).await;
        Uuid::parse_str(body["id"].as_str().unwrap()).unwrap()
    }};
}

#[actix_rt::test]
async fn create_party_returns_201() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let id = create_party!(&app, &bearer(owner_id), "farm@example.com");
    assert!(!id.to_string().is_empty());
}

#[actix_rt::test]
async fn get_party_returns_200_for_owner() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner2@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let party_id = create_party!(&app, &bearer(owner_id), "farm2@example.com");

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/parties/{party_id}"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["email"], "farm2@example.com");
}

#[actix_rt::test]
async fn list_my_parties_returns_owned_parties() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner3@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    create_party!(&app, &bearer(owner_id), "farm3a@example.com");
    create_party!(&app, &bearer(owner_id), "farm3b@example.com");

    let req = test::TestRequest::get()
        .uri("/api/v1/parties/me")
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["parties"].as_array().unwrap().len(), 2);
}

#[actix_rt::test]
async fn list_parties_requires_admin() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let user_id = seed_user(&fixtures, "regular@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/parties")
        .insert_header((header::AUTHORIZATION, bearer(user_id)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn search_parties_returns_results_for_admin() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let admin_id = seed_user(&fixtures, "admin@example.com", "admin");
    let owner_id = seed_user(&fixtures, "owner4@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    create_party!(&app, &bearer(owner_id), "searchable@example.com");

    let req = test::TestRequest::get()
        .uri("/api/v1/parties/search?q=searchable")
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["parties"].as_array().unwrap().len(), 1);
}

#[actix_rt::test]
async fn update_party_returns_200_for_owner() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner5@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let party_id = create_party!(&app, &bearer(owner_id), "farm5@example.com");

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/parties/{party_id}"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .set_json(serde_json::json!({ "display_name": "Updated Farm" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["display_name"], "Updated Farm");
}

#[actix_rt::test]
async fn add_and_remove_party_role() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner6@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let party_id = create_party!(&app, &bearer(owner_id), "farm6@example.com");

    let add = test::TestRequest::post()
        .uri(&format!("/api/v1/parties/{party_id}/roles"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .set_json(serde_json::json!({
            "role_type": "SUPPLIER",
            "profile": {
                "type": "SUPPLIER",
                "resource_type_ids": [],
                "preferred_compensation": [],
                "insurance_verified": false
            }
        }))
        .to_request();
    let resp = test::call_service(&app, add).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let list = test::TestRequest::get()
        .uri(&format!("/api/v1/parties/{party_id}/roles"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .to_request();
    let resp = test::call_service(&app, list).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["roles"].as_array().unwrap().len(), 1);

    let remove = test::TestRequest::delete()
        .uri(&format!("/api/v1/parties/{party_id}/roles/SUPPLIER"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .to_request();
    let resp = test::call_service(&app, remove).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[actix_rt::test]
async fn delete_party_returns_204_for_owner() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner7@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let party_id = create_party!(&app, &bearer(owner_id), "farm7@example.com");

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/parties/{party_id}"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

async fn seed_party_with_location(
    state: &AppState,
    owner_id: Uuid,
    email: &str,
    lat: f64,
    lng: f64,
) -> Uuid {
    let result = state
        .create_party
        .execute(application::parties::dto::CreatePartyCommand {
            actor_user_id: owner_id,
            party_type: PartyType::Organization,
            display_name: "Located Farm".to_string(),
            email: email.to_string(),
            phone: None,
            tax_id: None,
            primary_domain_id: None,
            latitude: Some(lat),
            longitude: Some(lng),
            service_radius_km: Some(10.0),
            roles: vec![],
        })
        .await
        .unwrap();
    result.id
}

#[actix_rt::test]
async fn search_parties_with_radius_returns_results() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let admin_id = seed_user(&fixtures, "admin_radius@example.com", "admin");
    let owner_id = seed_user(&fixtures, "owner_radius@example.com", "user");

    seed_party_with_location(
        &fixtures.state,
        owner_id,
        "within@example.com",
        37.7749,
        -122.4194,
    )
    .await;
    seed_party_with_location(
        &fixtures.state,
        owner_id,
        "outside@example.com",
        40.7128,
        -74.0060,
    )
    .await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/parties/search?lat=37.7749&lng=-122.4194&radiusKm=10")
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["parties"].as_array().unwrap().len(), 1);
    assert_eq!(body["total"], 1);
}

#[actix_rt::test]
async fn nearby_parties_returns_parties_within_radius() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner_nearby@example.com", "user");

    seed_party_with_location(
        &fixtures.state,
        owner_id,
        "nearby_a@example.com",
        37.7750,
        -122.4195,
    )
    .await;
    seed_party_with_location(
        &fixtures.state,
        owner_id,
        "nearby_b@example.com",
        37.7760,
        -122.4200,
    )
    .await;
    seed_party_with_location(
        &fixtures.state,
        owner_id,
        "far_away@example.com",
        48.8566,
        2.3522,
    )
    .await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/parties/nearby?lat=37.7749&lng=-122.4194&radiusKm=1")
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["parties"].as_array().unwrap().len(), 2);
    assert_eq!(body["total"], 2);
}

#[actix_rt::test]
async fn nearby_parties_requires_radius_and_coordinates() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner_badgeo@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/parties/nearby?lat=37.7749")
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

fn role_profile(role: &str) -> serde_json::Value {
    match role {
        "SUPPLIER" => serde_json::json!({
            "type": "SUPPLIER",
            "resource_type_ids": [],
            "preferred_compensation": [],
            "insurance_verified": false
        }),
        "CONSUMER" => serde_json::json!({
            "type": "CONSUMER",
            "need_category_ids": [],
            "preferred_payment_terms": []
        }),
        "ENHANCER" => serde_json::json!({
            "type": "ENHANCER",
            "enhancement_type_ids": [],
            "skills": [],
            "equipment_owned": []
        }),
        _ => panic!("unsupported test role: {role}"),
    }
}

macro_rules! create_party_with_role {
    ($app:expr, $owner_id:expr, $email:expr, $role:expr) => {{
        let party_id = create_party!($app, &bearer($owner_id), $email);

        let add = test::TestRequest::post()
            .uri(&format!("/api/v1/parties/{party_id}/roles"))
            .insert_header((header::AUTHORIZATION, bearer($owner_id)))
            .set_json(serde_json::json!({
                "role_type": $role,
                "profile": role_profile($role)
            }))
            .to_request();
        let resp = test::call_service($app, add).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        party_id
    }};
}

macro_rules! create_three_party_deal {
    ($app:expr, $owner_id:expr, $supplier:expr, $consumer:expr, $enhancer:expr) => {{
        let category_id = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
        let req = test::TestRequest::post()
            .uri("/api/v1/deals")
            .insert_header((header::AUTHORIZATION, bearer($owner_id)))
            .insert_header(("X-Party-ID", $supplier.to_string()))
            .set_json(serde_json::json!({
                "title": "API Negotiation Deal",
                "domain_category_id": category_id,
                "consumer_party_id": $consumer,
                "enhancer_party_id": $enhancer
            }))
            .to_request();
        let resp = test::call_service($app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body: serde_json::Value = test::read_body_json(resp).await;
        Uuid::parse_str(body["id"].as_str().unwrap()).unwrap()
    }};
}

#[actix_rt::test]
async fn propose_and_list_terms_via_api() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "termowner@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let supplier = create_party_with_role!(&app, owner_id, "supplier-term@example.com", "SUPPLIER");
    let consumer = create_party_with_role!(&app, owner_id, "consumer-term@example.com", "CONSUMER");
    let enhancer = create_party_with_role!(&app, owner_id, "enhancer-term@example.com", "ENHANCER");
    let deal_id = create_three_party_deal!(&app, owner_id, supplier, consumer, enhancer);

    let propose = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/terms"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(serde_json::json!({
            "term_type": "PRICE",
            "term_name": "Unit price",
            "description": "100 points",
            "is_mandatory": true
        }))
        .to_request();
    let resp = test::call_service(&app, propose).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["term_name"], "Unit price");

    let list = test::TestRequest::get()
        .uri(&format!("/api/v1/deals/{deal_id}/terms"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .to_request();
    let resp = test::call_service(&app, list).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[actix_rt::test]
async fn set_and_get_value_distribution_via_api() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "valueowner@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let supplier =
        create_party_with_role!(&app, owner_id, "supplier-value@example.com", "SUPPLIER");
    let consumer =
        create_party_with_role!(&app, owner_id, "consumer-value@example.com", "CONSUMER");
    let enhancer =
        create_party_with_role!(&app, owner_id, "enhancer-value@example.com", "ENHANCER");
    let deal_id = create_three_party_deal!(&app, owner_id, supplier, consumer, enhancer);

    let set = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/value-distribution"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(serde_json::json!({
            "total_value": "10000",
            "distribution_model": "FIXED_PRICE",
            "supplier_share_percentage": "60",
            "enhancer_share_percentage": "30",
            "platform_fee_percentage": "10",
            "consumer_cost_percentage": "100",
            "payment_schedule": []
        }))
        .to_request();
    let resp = test::call_service(&app, set).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["total_value"], "10000");

    let get = test::TestRequest::get()
        .uri(&format!("/api/v1/deals/{deal_id}/value-distribution"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .to_request();
    let resp = test::call_service(&app, get).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["supplier_share_amount"], "6000");
}

#[actix_rt::test]
async fn validate_deal_via_api_returns_good() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "validateowner@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let supplier =
        create_party_with_role!(&app, owner_id, "supplier-validate@example.com", "SUPPLIER");
    let consumer =
        create_party_with_role!(&app, owner_id, "consumer-validate@example.com", "CONSUMER");
    let enhancer =
        create_party_with_role!(&app, owner_id, "enhancer-validate@example.com", "ENHANCER");
    let deal_id = create_three_party_deal!(&app, owner_id, supplier, consumer, enhancer);

    let set = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/value-distribution"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(serde_json::json!({
            "total_value": "10000",
            "distribution_model": "FIXED_PRICE",
            "supplier_share_percentage": "60",
            "enhancer_share_percentage": "30",
            "platform_fee_percentage": "10",
            "consumer_cost_percentage": "100",
            "payment_schedule": []
        }))
        .to_request();
    let resp = test::call_service(&app, set).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let validate = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/validate"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .to_request();
    let resp = test::call_service(&app, validate).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "GOOD");
    assert_eq!(body["blocked"], false);
}

#[actix_rt::test]
async fn submit_deal_without_value_distribution_returns_conflict() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "submitowner@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let supplier =
        create_party_with_role!(&app, owner_id, "supplier-submit@example.com", "SUPPLIER");
    let consumer =
        create_party_with_role!(&app, owner_id, "consumer-submit@example.com", "CONSUMER");
    let enhancer =
        create_party_with_role!(&app, owner_id, "enhancer-submit@example.com", "ENHANCER");
    let deal_id = create_three_party_deal!(&app, owner_id, supplier, consumer, enhancer);

    let submit = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/submit"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .to_request();
    let resp = test::call_service(&app, submit).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

// ============================================================================
// Agreement handler tests
// ============================================================================

async fn prepare_deal_for_agreement(
    fixtures: &TestFixtures,
    deal_id: Uuid,
    actor_user_id: Uuid,
    actor_party_id: Uuid,
) {
    fixtures
        .state
        .set_value_distribution
        .execute(SetValueDistributionCommand {
            actor_user_id,
            actor_party_id,
            is_admin: false,
            deal_id,
            total_value: Decimal::from(10000),
            distribution_model: DistributionModel::FixedPrice,
            supplier_share_percentage: Decimal::from(60),
            enhancer_share_percentage: Decimal::from(30),
            platform_fee_percentage: Decimal::from(10),
            consumer_cost_percentage: Decimal::from(100),
            payment_schedule: vec![],
        })
        .await
        .unwrap();

    let mut deals = fixtures.deal_repo.deals.lock().unwrap();
    let deal = deals.get_mut(&deal_id).expect("deal exists");
    deal.deal_status = DealStatus::TermsLocked;
}

#[actix_rt::test]
async fn get_agreement_visible_to_participant() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "agreementowner@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state.clone()))
            .configure(routes::configure),
    )
    .await;

    let supplier =
        create_party_with_role!(&app, owner_id, "supplier-agree@example.com", "SUPPLIER");
    let consumer =
        create_party_with_role!(&app, owner_id, "consumer-agree@example.com", "CONSUMER");
    let enhancer =
        create_party_with_role!(&app, owner_id, "enhancer-agree@example.com", "ENHANCER");
    let deal_id = create_three_party_deal!(&app, owner_id, supplier, consumer, enhancer);
    prepare_deal_for_agreement(&fixtures, deal_id, owner_id, supplier).await;

    fixtures
        .state
        .generate_agreement
        .execute(application::agreements::dto::GenerateAgreementCommand {
            actor_user_id: owner_id,
            actor_party_id: supplier,
            deal_id,
        })
        .await
        .unwrap();

    let get = test::TestRequest::get()
        .uri(&format!("/api/v1/deals/{deal_id}/agreement"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .to_request();
    let resp = test::call_service(&app, get).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["deal_id"].as_str().unwrap(), deal_id.to_string());
    assert!(body["agreement_text"]
        .as_str()
        .unwrap()
        .contains("API Negotiation Deal"));
}

#[actix_rt::test]
async fn get_agreement_hidden_from_outsider() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "agreementowner2@example.com", "user");
    let outsider_id = seed_user(&fixtures, "outsider@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state.clone()))
            .configure(routes::configure),
    )
    .await;

    let supplier =
        create_party_with_role!(&app, owner_id, "supplier-agree2@example.com", "SUPPLIER");
    let consumer =
        create_party_with_role!(&app, owner_id, "consumer-agree2@example.com", "CONSUMER");
    let enhancer =
        create_party_with_role!(&app, owner_id, "enhancer-agree2@example.com", "ENHANCER");
    let deal_id = create_three_party_deal!(&app, owner_id, supplier, consumer, enhancer);
    prepare_deal_for_agreement(&fixtures, deal_id, owner_id, supplier).await;

    fixtures
        .state
        .generate_agreement
        .execute(application::agreements::dto::GenerateAgreementCommand {
            actor_user_id: owner_id,
            actor_party_id: supplier,
            deal_id,
        })
        .await
        .unwrap();

    let get = test::TestRequest::get()
        .uri(&format!("/api/v1/deals/{deal_id}/agreement"))
        .insert_header((header::AUTHORIZATION, bearer(outsider_id)))
        .to_request();
    let resp = test::call_service(&app, get).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_rt::test]
async fn sign_agreement_records_signature_via_api() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "signowner@example.com", "user");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state.clone()))
            .configure(routes::configure),
    )
    .await;

    let supplier = create_party_with_role!(&app, owner_id, "supplier-sign@example.com", "SUPPLIER");
    let consumer = create_party_with_role!(&app, owner_id, "consumer-sign@example.com", "CONSUMER");
    let enhancer = create_party_with_role!(&app, owner_id, "enhancer-sign@example.com", "ENHANCER");
    let deal_id = create_three_party_deal!(&app, owner_id, supplier, consumer, enhancer);
    prepare_deal_for_agreement(&fixtures, deal_id, owner_id, supplier).await;

    fixtures
        .state
        .generate_agreement
        .execute(application::agreements::dto::GenerateAgreementCommand {
            actor_user_id: owner_id,
            actor_party_id: supplier,
            deal_id,
        })
        .await
        .unwrap();

    let sign = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/agreement/sign"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(serde_json::json!({}))
        .to_request();
    let resp = test::call_service(&app, sign).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["signatures"].as_array().unwrap().len(), 1);
    assert_eq!(
        body["signatures"][0]["party_id"].as_str().unwrap(),
        supplier.to_string()
    );
}

#[actix_rt::test]
async fn admin_can_get_and_update_agreement() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "admindealowner@example.com", "user");
    let admin_id = seed_user(&fixtures, "platformadmin@example.com", "admin");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state.clone()))
            .configure(routes::configure),
    )
    .await;

    let supplier =
        create_party_with_role!(&app, owner_id, "supplier-admin@example.com", "SUPPLIER");
    let consumer =
        create_party_with_role!(&app, owner_id, "consumer-admin@example.com", "CONSUMER");
    let enhancer =
        create_party_with_role!(&app, owner_id, "enhancer-admin@example.com", "ENHANCER");
    let deal_id = create_three_party_deal!(&app, owner_id, supplier, consumer, enhancer);
    prepare_deal_for_agreement(&fixtures, deal_id, owner_id, supplier).await;

    fixtures
        .state
        .generate_agreement
        .execute(application::agreements::dto::GenerateAgreementCommand {
            actor_user_id: owner_id,
            actor_party_id: supplier,
            deal_id,
        })
        .await
        .unwrap();

    let get = test::TestRequest::get()
        .uri(&format!("/api/v1/admin/deals/{deal_id}/agreement"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, get).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let patch = test::TestRequest::patch()
        .uri(&format!("/api/v1/admin/deals/{deal_id}/agreement"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .set_json(serde_json::json!({
            "governing_law": "California",
            "dispute_resolution": "Arbitration",
            "auto_renew": true
        }))
        .to_request();
    let resp = test::call_service(&app, patch).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["governing_law"].as_str().unwrap(), "California");
    assert_eq!(body["dispute_resolution"].as_str().unwrap(), "Arbitration");
    assert!(body["auto_renew"].as_bool().unwrap());
}

async fn seed_party(state: &AppState, owner_id: Uuid, email: &str, role: DealRole) -> Uuid {
    state
        .create_party
        .execute(application::parties::dto::CreatePartyCommand {
            actor_user_id: owner_id,
            party_type: PartyType::Organization,
            display_name: "Wallet Party".to_string(),
            email: email.to_string(),
            phone: None,
            tax_id: None,
            primary_domain_id: None,
            latitude: None,
            longitude: None,
            service_radius_km: None,
            roles: vec![role],
        })
        .await
        .unwrap()
        .id
}

async fn seed_deal(
    state: &AppState,
    actor_user_id: Uuid,
    actor_party_id: Uuid,
    consumer_party_id: Uuid,
    enhancer_party_id: Uuid,
) -> Uuid {
    let category_id = Uuid::parse_str("a1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap();
    state
        .create_deal
        .execute(application::deals::dto::CreateDealCommand {
            actor_user_id,
            actor_party_id,
            is_admin: false,
            title: "Wallet Deal".to_string(),
            description: None,
            domain_category_id: category_id,
            consumer_party_id,
            enhancer_party_id,
            expected_start_date: None,
            expected_end_date: None,
            timeline: None,
            latitude: None,
            longitude: None,
            timeout_overrides: None,
        })
        .await
        .unwrap()
        .id
}

#[actix_rt::test]
async fn wallet_endpoints_manage_points_for_party() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "wallet_owner@example.com", "user");
    let party_id = seed_party(
        &fixtures.state,
        owner_id,
        "wallet_party@example.com",
        DealRole::Supplier,
    )
    .await;
    let consumer_id = seed_party(
        &fixtures.state,
        owner_id,
        "consumer_wallet@example.com",
        DealRole::Consumer,
    )
    .await;
    let enhancer_id = seed_party(
        &fixtures.state,
        owner_id,
        "enhancer_wallet@example.com",
        DealRole::Enhancer,
    )
    .await;
    let deal_id = seed_deal(
        &fixtures.state,
        owner_id,
        party_id,
        consumer_id,
        enhancer_id,
    )
    .await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let get_wallet = test::TestRequest::get()
        .uri(&format!("/api/v1/parties/{party_id}/wallet"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .to_request();
    let resp = test::call_service(&app, get_wallet).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["partyId"], party_id.to_string());
    assert_eq!(body["balance"], "0");

    let deposit = test::TestRequest::post()
        .uri(&format!("/api/v1/parties/{party_id}/wallet/deposits"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .set_json(serde_json::json!({
            "dealId": deal_id,
            "amount": "150.00"
        }))
        .to_request();
    let resp = test::call_service(&app, deposit).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["amount"], "150.00");

    let get_wallet = test::TestRequest::get()
        .uri(&format!("/api/v1/parties/{party_id}/wallet"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .to_request();
    let resp = test::call_service(&app, get_wallet).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["balance"], "150.00");

    let get_deal_wallet = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/parties/{party_id}/deals/{deal_id}/wallet"
        ))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .to_request();
    let resp = test::call_service(&app, get_deal_wallet).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["deposited"], "150.00");

    let list_transactions = test::TestRequest::get()
        .uri(&format!("/api/v1/parties/{party_id}/wallet/transactions"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .to_request();
    let resp = test::call_service(&app, list_transactions).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["total"], 1);
    assert_eq!(body["transactions"].as_array().unwrap().len(), 1);
}

#[actix_rt::test]
async fn admin_can_get_party_and_roles() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner_admin_party@example.com", "user");
    let admin_id = seed_user(&fixtures, "admin_party@example.com", "admin");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let party_id = create_party!(&app, &bearer(owner_id), "farm-admin@example.com");

    let get = test::TestRequest::get()
        .uri(&format!("/api/v1/parties/{party_id}"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, get).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["email"], "farm-admin@example.com");

    let roles = test::TestRequest::get()
        .uri(&format!("/api/v1/parties/{party_id}/roles"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, roles).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_rt::test]
async fn admin_can_update_party() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner_admin_update_party@example.com", "user");
    let admin_id = seed_user(&fixtures, "admin_update_party@example.com", "admin");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let party_id = create_party!(&app, &bearer(owner_id), "farm-admin-update@example.com");

    let update = test::TestRequest::patch()
        .uri(&format!("/api/v1/admin/parties/{party_id}"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .set_json(serde_json::json!({
            "verification_status": "VERIFIED",
            "is_active": false
        }))
        .to_request();
    let resp = test::call_service(&app, update).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["verification_status"], "VERIFIED");
    assert_eq!(body["is_active"], false);
}

#[actix_rt::test]
async fn admin_can_delete_party_with_active_deals() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner_admin_delete_party@example.com", "user");
    let admin_id = seed_user(&fixtures, "admin_delete_party@example.com", "admin");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let supplier = create_party_with_role!(
        &app,
        owner_id,
        "supplier-admin-delete@example.com",
        "SUPPLIER"
    );
    let consumer = create_party_with_role!(
        &app,
        owner_id,
        "consumer-admin-delete@example.com",
        "CONSUMER"
    );
    let enhancer = create_party_with_role!(
        &app,
        owner_id,
        "enhancer-admin-delete@example.com",
        "ENHANCER"
    );
    create_three_party_deal!(&app, owner_id, supplier, consumer, enhancer);

    let delete = test::TestRequest::delete()
        .uri(&format!("/api/v1/parties/{supplier}"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, delete).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[actix_rt::test]
async fn admin_can_validate_deal() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner_admin_validate@example.com", "user");
    let admin_id = seed_user(&fixtures, "admin_validate@example.com", "admin");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let supplier = create_party_with_role!(
        &app,
        owner_id,
        "supplier-admin-validate@example.com",
        "SUPPLIER"
    );
    let consumer = create_party_with_role!(
        &app,
        owner_id,
        "consumer-admin-validate@example.com",
        "CONSUMER"
    );
    let enhancer = create_party_with_role!(
        &app,
        owner_id,
        "enhancer-admin-validate@example.com",
        "ENHANCER"
    );
    let deal_id = create_three_party_deal!(&app, owner_id, supplier, consumer, enhancer);

    let set = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/value-distribution"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(serde_json::json!({
            "total_value": "10000",
            "distribution_model": "FIXED_PRICE",
            "supplier_share_percentage": "60",
            "enhancer_share_percentage": "30",
            "platform_fee_percentage": "10",
            "consumer_cost_percentage": "100",
            "payment_schedule": []
        }))
        .to_request();
    let resp = test::call_service(&app, set).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let validate = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/validate"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, validate).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "GOOD");
}

#[actix_rt::test]
async fn admin_can_list_terms_and_value_distribution() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner_admin_terms@example.com", "user");
    let admin_id = seed_user(&fixtures, "admin_terms@example.com", "admin");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let supplier = create_party_with_role!(
        &app,
        owner_id,
        "supplier-admin-terms@example.com",
        "SUPPLIER"
    );
    let consumer = create_party_with_role!(
        &app,
        owner_id,
        "consumer-admin-terms@example.com",
        "CONSUMER"
    );
    let enhancer = create_party_with_role!(
        &app,
        owner_id,
        "enhancer-admin-terms@example.com",
        "ENHANCER"
    );
    let deal_id = create_three_party_deal!(&app, owner_id, supplier, consumer, enhancer);

    let propose = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/terms"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(serde_json::json!({
            "term_type": "PRICE",
            "term_name": "Admin visible term",
            "description": "100 points",
            "is_mandatory": true
        }))
        .to_request();
    let resp = test::call_service(&app, propose).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let set = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/value-distribution"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(serde_json::json!({
            "total_value": "10000",
            "distribution_model": "FIXED_PRICE",
            "supplier_share_percentage": "60",
            "enhancer_share_percentage": "30",
            "platform_fee_percentage": "10",
            "consumer_cost_percentage": "100",
            "payment_schedule": []
        }))
        .to_request();
    let resp = test::call_service(&app, set).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let list_terms = test::TestRequest::get()
        .uri(&format!("/api/v1/deals/{deal_id}/terms"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, list_terms).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 1);

    let get_value = test::TestRequest::get()
        .uri(&format!("/api/v1/deals/{deal_id}/value-distribution"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, get_value).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["total_value"], "10000");
}

#[actix_rt::test]
async fn admin_can_manage_deal_without_membership() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner_admin_deal@example.com", "user");
    let admin_id = seed_user(&fixtures, "admin_deal@example.com", "admin");

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let supplier = create_party_with_role!(
        &app,
        owner_id,
        "supplier-admin-deal@example.com",
        "SUPPLIER"
    );
    let consumer = create_party_with_role!(
        &app,
        owner_id,
        "consumer-admin-deal@example.com",
        "CONSUMER"
    );
    let enhancer = create_party_with_role!(
        &app,
        owner_id,
        "enhancer-admin-deal@example.com",
        "ENHANCER"
    );

    let category_id = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
    let create = test::TestRequest::post()
        .uri("/api/v1/deals")
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(serde_json::json!({
            "title": "Admin Managed Deal",
            "domain_category_id": category_id,
            "consumer_party_id": consumer,
            "enhancer_party_id": enhancer
        }))
        .to_request();
    let resp = test::call_service(&app, create).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let deal_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();

    let update = test::TestRequest::patch()
        .uri(&format!("/api/v1/deals/{deal_id}"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(serde_json::json!({
            "title": "Updated by admin"
        }))
        .to_request();
    let resp = test::call_service(&app, update).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["title"], "Updated by admin");

    let set = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/value-distribution"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(serde_json::json!({
            "total_value": "10000",
            "distribution_model": "FIXED_PRICE",
            "supplier_share_percentage": "60",
            "enhancer_share_percentage": "30",
            "platform_fee_percentage": "10",
            "consumer_cost_percentage": "100",
            "payment_schedule": []
        }))
        .to_request();
    let resp = test::call_service(&app, set).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let propose = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/terms"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(serde_json::json!({
            "term_type": "PRICE",
            "term_name": "Admin term",
            "description": "100 points",
            "is_mandatory": true
        }))
        .to_request();
    let resp = test::call_service(&app, propose).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let term_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();

    let accept = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/terms/{term_id}/accept"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .to_request();
    let resp = test::call_service(&app, accept).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let submit = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/submit"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .to_request();
    let resp = test::call_service(&app, submit).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["deal_status"], "SUGGESTED");
}

#[actix_rt::test]
async fn admin_can_read_wallet_and_transactions() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner_admin_wallet@example.com", "user");
    let admin_id = seed_user(&fixtures, "admin_wallet@example.com", "admin");

    let party_id = seed_party(
        &fixtures.state,
        owner_id,
        "wallet_admin_party@example.com",
        DealRole::Supplier,
    )
    .await;
    let consumer_id = seed_party(
        &fixtures.state,
        owner_id,
        "consumer_admin_wallet@example.com",
        DealRole::Consumer,
    )
    .await;
    let enhancer_id = seed_party(
        &fixtures.state,
        owner_id,
        "enhancer_admin_wallet@example.com",
        DealRole::Enhancer,
    )
    .await;
    let deal_id = seed_deal(
        &fixtures.state,
        owner_id,
        party_id,
        consumer_id,
        enhancer_id,
    )
    .await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let deposit = test::TestRequest::post()
        .uri(&format!("/api/v1/parties/{party_id}/wallet/deposits"))
        .insert_header((header::AUTHORIZATION, bearer(owner_id)))
        .set_json(serde_json::json!({
            "dealId": deal_id,
            "amount": "200.00"
        }))
        .to_request();
    let resp = test::call_service(&app, deposit).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let get_wallet = test::TestRequest::get()
        .uri(&format!("/api/v1/parties/{party_id}/wallet"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, get_wallet).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["balance"], "200.00");

    let get_deal_wallet = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/parties/{party_id}/deals/{deal_id}/wallet"
        ))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, get_deal_wallet).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["deposited"], "200.00");

    let list_transactions = test::TestRequest::get()
        .uri(&format!("/api/v1/parties/{party_id}/wallet/transactions"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, list_transactions).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["total"], 1);

    let list_deal_transactions = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/parties/{party_id}/deals/{deal_id}/transactions"
        ))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, list_deal_transactions).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["total"], 1);
}

#[actix_rt::test]
async fn admin_can_deposit_and_withdraw_for_party() {
    init_test_tracing();
    let fixtures = test_fixtures();
    let owner_id = seed_user(&fixtures, "owner_admin_payment@example.com", "user");
    let admin_id = seed_user(&fixtures, "admin_payment@example.com", "admin");

    let party_id = seed_party(
        &fixtures.state,
        owner_id,
        "payment_admin_party@example.com",
        DealRole::Supplier,
    )
    .await;
    let consumer_id = seed_party(
        &fixtures.state,
        owner_id,
        "consumer_admin_payment@example.com",
        DealRole::Consumer,
    )
    .await;
    let enhancer_id = seed_party(
        &fixtures.state,
        owner_id,
        "enhancer_admin_payment@example.com",
        DealRole::Enhancer,
    )
    .await;
    let deal_id = seed_deal(
        &fixtures.state,
        owner_id,
        party_id,
        consumer_id,
        enhancer_id,
    )
    .await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(fixtures.state))
            .configure(routes::configure),
    )
    .await;

    let deposit = test::TestRequest::post()
        .uri(&format!("/api/v1/parties/{party_id}/wallet/deposits"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .set_json(serde_json::json!({
            "dealId": deal_id,
            "amount": "300.00"
        }))
        .to_request();
    let resp = test::call_service(&app, deposit).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["amount"], "300.00");

    let withdraw = test::TestRequest::post()
        .uri(&format!("/api/v1/parties/{party_id}/wallet/withdrawals"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .set_json(serde_json::json!({
            "dealId": deal_id,
            "amount": "100.00"
        }))
        .to_request();
    let resp = test::call_service(&app, withdraw).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["amount"], "100.00");

    let get_wallet = test::TestRequest::get()
        .uri(&format!("/api/v1/parties/{party_id}/wallet"))
        .insert_header((header::AUTHORIZATION, bearer(admin_id)))
        .to_request();
    let resp = test::call_service(&app, get_wallet).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["balance"], "200.00");
}
