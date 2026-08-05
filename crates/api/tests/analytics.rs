use actix_web::http::header;
use actix_web::http::StatusCode;
use actix_web::{test, web::Data, App};
use api::routes;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

mod common;

fn agriculture_domain_id() -> Uuid {
    Uuid::parse_str("a1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap()
}

async fn create_party(pool: &PgPool, email: &str, name: &str) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        r#"
        INSERT INTO parties (id, party_type, display_name, email, is_active, created_at, updated_at)
        VALUES ($1, 'ORGANIZATION', $2, $3, true, $4, $4)
        "#,
        id,
        name,
        email,
        OffsetDateTime::now_utc()
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn insert_deal(pool: &PgPool, party_id: Uuid, status: &str) -> Uuid {
    let id = Uuid::now_v7();
    let reference = format!("REF-{id}");
    sqlx::query!(
        r#"
        INSERT INTO deals (
            id, deal_reference, deal_title, deal_description, domain_category_id,
            initiator_party_id, initiator_role, deal_status, is_public, total_deal_value,
            created_at, updated_at
        )
        VALUES ($1, $2, 'Refresh Deal', 'Refresh test deal', $3, $4, 'SUPPLIER', $5, false, 1000, $6, $6)
        "#,
        id,
        reference,
        agriculture_domain_id(),
        party_id,
        status,
        OffsetDateTime::now_utc()
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

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
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let party = create_party(&pool, "refresh_party@example.com", "Refresh Farm").await;
    insert_deal(&pool, party, "DRAFT").await;

    let req = test::TestRequest::post()
        .uri("/api/v1/admin/analytics/refresh")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[sqlx::test(migrations = "../../migrations")]
async fn admin_analytics_endpoints_return_metrics(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let admin_id = common::create_active_user(&pool, "analyticsadmin@example.com").await;
    let admin_token = common::auth_token_with_roles(
        admin_id,
        vec!["admin".to_string()],
        vec!["admin:analytics".to_string()],
    )
    .await;

    let owner_id = common::create_active_user(&pool, "analyticsowner@example.com").await;
    let supplier = common::create_party_with_role(
        &pool,
        owner_id,
        "analyticssupplier@example.com",
        "Analytics Supplier",
        "SUPPLIER",
    )
    .await;
    let consumer = common::create_party_with_role(
        &pool,
        owner_id,
        "analyticsconsumer@example.com",
        "Analytics Consumer",
        "CONSUMER",
    )
    .await;
    let enhancer = common::create_party_with_role(
        &pool,
        owner_id,
        "analyticsenhancer@example.com",
        "Analytics Enhancer",
        "ENHANCER",
    )
    .await;
    let category = common::create_category(&pool, "Analytics Domain", "ANALYTICS-DOM", "DOMAIN").await;
    let _deal_id = common::create_deal_with_parties(
        &pool, supplier, supplier, consumer, enhancer, category, "DRAFT",
    )
    .await;

    let refresh_req = test::TestRequest::post()
        .uri("/api/v1/admin/analytics/refresh")
        .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
        .to_request();
    let resp = test::call_service(&app, refresh_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let today = time::OffsetDateTime::now_utc().date();
    let from = today - time::Duration::days(7);
    let trends_uri = format!(
        "/api/v1/admin/analytics/trends?from={}&to={}",
        from, today
    );
    let resp = test::TestRequest::get()
        .uri(&trends_uri)
        .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
        .to_request();
    let resp = test::call_service(&app, resp).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.as_array().unwrap().len() >= 1);

    let activity_uri = format!(
        "/api/v1/admin/analytics/activity?from={}&to={}",
        from, today
    );
    let resp = test::TestRequest::get()
        .uri(&activity_uri)
        .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
        .to_request();
    let resp = test::call_service(&app, resp).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.as_array().unwrap().len() >= 1);

    let resp = test::TestRequest::get()
        .uri("/api/v1/admin/analytics/metrics?limit=10&offset=0")
        .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
        .to_request();
    let resp = test::call_service(&app, resp).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["items"].as_array().unwrap().len() >= 1);
    assert!(body["total"].as_i64().unwrap() >= 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn analytics_endpoints_require_admin_scope(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let user_id = common::create_active_user(&pool, "analyticsuser@example.com").await;
    let token = common::auth_token(user_id, vec!["users:read".to_string()]).await;

    let resp = test::TestRequest::get()
        .uri("/api/v1/admin/analytics/trends")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, resp).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
