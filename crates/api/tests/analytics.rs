use actix_web::http::header;
use actix_web::http::StatusCode;
use actix_web::{test, web::Data, App};
use api::routes;
use sqlx::PgPool;

mod common;

#[sqlx::test(migrations = "../../migrations")]
async fn admin_dashboard_returns_200(pool: PgPool) {
    let user_id = common::create_active_user(&pool, "admin@example.com").await;
    let token = common::auth_token(user_id, vec!["admin:analytics".to_string()]).await;
    let state = common::build_state(pool).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/analytics/dashboard")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("total_deals").is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn non_admin_dashboard_returns_403(pool: PgPool) {
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
        .uri("/api/v1/admin/analytics/dashboard")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../migrations")]
async fn admin_refresh_metrics_returns_200(pool: PgPool) {
    let user_id = common::create_active_user(&pool, "admin2@example.com").await;
    let token = common::auth_token(user_id, vec!["admin:analytics".to_string()]).await;
    let state = common::build_state(pool).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/admin/analytics/refresh")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}
