use actix_web::http::StatusCode;
use actix_web::{test, web::Data, App};
use api::routes;
use sqlx::PgPool;

mod common;

#[sqlx::test(migrations = "../../migrations")]
async fn public_search_returns_200_and_empty_results(pool: PgPool) {
    let state = common::build_state(pool).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/search?q=organic")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["items"].is_array());
    assert_eq!(body["total"].as_i64().unwrap(), 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn search_with_invalid_type_returns_400(pool: PgPool) {
    let state = common::build_state(pool).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/search?q=foo&type=invalid")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
