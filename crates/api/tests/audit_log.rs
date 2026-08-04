use actix_web::http::header;
use actix_web::http::StatusCode;
use actix_web::{test, web::Data, App};
use api::routes;
use serde_json::json;
use sqlx::PgPool;

mod common;

#[sqlx::test(migrations = "../../migrations")]
async fn admin_can_list_audit_log(pool: PgPool) {
    let user_id = common::create_active_user(&pool, "admin@example.com").await;
    let token = common::auth_token(user_id, vec!["admin:audit".to_string()]).await;
    let state = common::build_state(pool).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/audit-log")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["items"].is_array());
}

#[sqlx::test(migrations = "../../migrations")]
async fn admin_can_record_audit_action(pool: PgPool) {
    let user_id = common::create_active_user(&pool, "admin2@example.com").await;
    let token = common::auth_token(user_id, vec!["admin:audit".to_string()]).await;
    let state = common::build_state(pool).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let target_id = uuid::Uuid::now_v7();
    let req = test::TestRequest::post()
        .uri("/api/v1/admin/audit-log")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(json!({
            "action_type": "PARTY_UPDATED",
            "target_type": "PARTY",
            "target_id": target_id,
            "reason": "test action"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["admin_user_id"].as_str().unwrap(), user_id.to_string());
    assert_eq!(body["action_type"].as_str().unwrap(), "PARTY_UPDATED");
}

#[sqlx::test(migrations = "../../migrations")]
async fn non_admin_audit_log_returns_403(pool: PgPool) {
    let user_id = common::create_active_user(&pool, "user@example.com").await;
    let token = common::auth_token(user_id, vec!["users:read".to_string()]).await;
    let state = common::build_state(pool).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/audit-log")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
