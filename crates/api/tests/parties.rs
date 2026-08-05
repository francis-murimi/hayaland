use actix_web::http::header;
use actix_web::http::StatusCode;
use actix_web::{test, web::Data, App};
use api::routes;
use sqlx::PgPool;

mod common;

#[sqlx::test(migrations = "../../migrations")]
async fn admin_can_list_parties(pool: PgPool) {
    let user_id = common::create_active_user(&pool, "listpartiesadmin@example.com").await;
    let token = common::auth_token_with_roles(
        user_id,
        vec!["admin".to_string()],
        vec!["admin:parties".to_string()],
    )
    .await;
    let _party = common::create_party_with_role(
        &pool,
        user_id,
        "listpartiesparty@example.com",
        "Listed Party",
        "SUPPLIER",
    )
    .await;

    let state = common::build_state(pool).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/parties")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["parties"].as_array().unwrap().len() >= 1);
    assert!(body["total"].as_i64().unwrap() >= 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn non_admin_cannot_list_parties(pool: PgPool) {
    let user_id = common::create_active_user(&pool, "listpartiesuser@example.com").await;
    let token = common::auth_token(user_id, vec!["parties:read".to_string()]).await;

    let state = common::build_state(pool).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/parties")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_parties_validates_bad_role(pool: PgPool) {
    let user_id = common::create_active_user(&pool, "listpartiesbadrole@example.com").await;
    let token = common::auth_token_with_roles(
        user_id,
        vec!["admin".to_string()],
        vec!["admin:parties".to_string()],
    )
    .await;

    let state = common::build_state(pool).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/parties?roles=INVALID_ROLE")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../../migrations")]
async fn search_parties_returns_results(pool: PgPool) {
    let user_id = common::create_active_user(&pool, "searchpartiesuser@example.com").await;
    let _party = common::create_party_with_role(
        &pool,
        user_id,
        "searchableparty@example.com",
        "Searchable Party",
        "CONSUMER",
    )
    .await;

    let token = common::auth_token_with_roles(
        user_id,
        vec!["admin".to_string()],
        vec!["parties:read".to_string()],
    )
    .await;

    let state = common::build_state(pool).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/parties/search?q=Searchable")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["parties"].as_array().unwrap().len() >= 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn nearby_parties_requires_location(pool: PgPool) {
    let user_id = common::create_active_user(&pool, "nearbyuser@example.com").await;
    let token = common::auth_token_with_roles(
        user_id,
        vec!["admin".to_string()],
        vec!["parties:read".to_string()],
    )
    .await;

    let state = common::build_state(pool).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/parties/nearby?radiusKm=10")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../../migrations")]
async fn nearby_parties_with_location_returns_results(pool: PgPool) {
    let user_id = common::create_active_user(&pool, "nearbywithloc@example.com").await;
    let party_id = common::create_party_with_role(
        &pool,
        user_id,
        "nearbyparty@example.com",
        "Nearby Party",
        "SUPPLIER",
    )
    .await;

    sqlx::query!(
        "UPDATE parties SET latitude = 40.0, longitude = -74.0, location_geo = ST_SetSRID(ST_MakePoint(-74.0, 40.0), 4326)::geography WHERE id = $1",
        party_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let token = common::auth_token_with_roles(
        user_id,
        vec!["admin".to_string()],
        vec!["parties:read".to_string()],
    )
    .await;

    let state = common::build_state(pool).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/parties/nearby?lat=40.0&lng=-74.0&radiusKm=10")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["parties"].as_array().unwrap().len() >= 1);
}
