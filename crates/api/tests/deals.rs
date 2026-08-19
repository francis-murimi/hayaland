use actix_web::http::header;
use actix_web::http::StatusCode;
use actix_web::{test, web::Data, App};
use api::routes;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

mod common;

async fn create_party_for_deals(pool: &PgPool, owner_id: Uuid, email: &str, role: &str) -> Uuid {
    common::create_party_with_role(pool, owner_id, email, role, role).await
}

async fn create_three_party_deal(pool: &PgPool, owner_id: Uuid) -> (Uuid, Uuid, Uuid, Uuid, Uuid) {
    let supplier =
        create_party_for_deals(pool, owner_id, "dealsupplier@example.com", "SUPPLIER").await;
    let consumer =
        create_party_for_deals(pool, owner_id, "dealconsumer@example.com", "CONSUMER").await;
    let enhancer =
        create_party_for_deals(pool, owner_id, "dealenhancer@example.com", "ENHANCER").await;
    let category = common::create_category(pool, "Deal Domain", "DEAL-DOM", "DOMAIN").await;
    let deal_id = common::create_deal_with_parties(
        pool, supplier, supplier, consumer, enhancer, category, "DRAFT",
    )
    .await;
    (deal_id, supplier, consumer, enhancer, category)
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_deal_requires_x_party_id(pool: PgPool) {
    let user_id = common::create_active_user(&pool, "createdealuser@example.com").await;
    let token = common::auth_token_with_roles(
        user_id,
        vec!["user".to_string()],
        vec!["deals:write".to_string(), "deals:read".to_string()],
    )
    .await;

    let category = common::create_category(&pool, "Create Domain", "CREATE-DOM", "DOMAIN").await;
    let consumer =
        create_party_for_deals(&pool, user_id, "createconsumer@example.com", "CONSUMER").await;
    let enhancer =
        create_party_for_deals(&pool, user_id, "createenhancer@example.com", "ENHANCER").await;

    let state = common::build_state(pool).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/deals")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(json!({
            "title": "Missing Header Deal",
            "domain_category_id": category,
            "consumer_party_id": consumer,
            "enhancer_party_id": enhancer
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_deal_rejects_invalid_timeout_overrides(pool: PgPool) {
    let user_id = common::create_active_user(&pool, "createdealinvalid@example.com").await;
    let token = common::auth_token_with_roles(
        user_id,
        vec!["user".to_string()],
        vec!["deals:write".to_string(), "deals:read".to_string()],
    )
    .await;

    let supplier =
        create_party_for_deals(&pool, user_id, "invalidsupplier@example.com", "SUPPLIER").await;
    let consumer =
        create_party_for_deals(&pool, user_id, "invalidconsumer@example.com", "CONSUMER").await;
    let enhancer =
        create_party_for_deals(&pool, user_id, "invalidenhancer@example.com", "ENHANCER").await;
    let category = common::create_category(&pool, "Invalid Domain", "INVALID-DOM", "DOMAIN").await;

    let state = common::build_state(pool).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/deals")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(json!({
            "title": "Bad Timeout Deal",
            "domain_category_id": category,
            "consumer_party_id": consumer,
            "enhancer_party_id": enhancer,
            "timeout_overrides": { "DRAFT": -1 }
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../../migrations")]
async fn execute_transition_cancels_deal(pool: PgPool) {
    let user_id = common::create_active_user(&pool, "transitionuser@example.com").await;
    let (deal_id, supplier, _consumer, _enhancer, _category) =
        create_three_party_deal(&pool, user_id).await;
    let token = common::auth_token_with_roles(
        user_id,
        vec!["user".to_string()],
        vec!["deals:write".to_string(), "deals:read".to_string()],
    )
    .await;

    let state = common::build_state(pool).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/transitions"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(json!({
            "new_status": "CANCELLED",
            "reason": "changed plans"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["deal_status"], "CANCELLED");
}

#[sqlx::test(migrations = "../../migrations")]
async fn execute_transition_validates_bad_status(pool: PgPool) {
    let user_id = common::create_active_user(&pool, "transitionbaduser@example.com").await;
    let (deal_id, supplier, _consumer, _enhancer, _category) =
        create_three_party_deal(&pool, user_id).await;
    let token = common::auth_token_with_roles(
        user_id,
        vec!["user".to_string()],
        vec!["deals:write".to_string(), "deals:read".to_string()],
    )
    .await;

    let state = common::build_state(pool).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/transitions"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(json!({
            "new_status": "COMPLETED"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[sqlx::test(migrations = "../../migrations")]
async fn terms_lifecycle(pool: PgPool) {
    let user_id = common::create_active_user(&pool, "termsuser@example.com").await;
    let (deal_id, supplier, consumer, _enhancer, _category) =
        create_three_party_deal(&pool, user_id).await;
    let token = common::auth_token_with_roles(
        user_id,
        vec!["user".to_string()],
        vec!["deals:write".to_string(), "deals:read".to_string()],
    )
    .await;

    let state = common::build_state(pool).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let propose_req = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/terms"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(json!({
            "term_type": "DELIVERY_DATE",
            "term_name": "Deliver in 30 days",
            "description": "Goods must be delivered within 30 days",
            "is_mandatory": true
        }))
        .to_request();
    let resp = test::call_service(&app, propose_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let term_id = body["id"].as_str().unwrap();

    let list_req = test::TestRequest::get()
        .uri(&format!("/api/v1/deals/{deal_id}/terms"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .to_request();
    let resp = test::call_service(&app, list_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 1);

    let counter_req = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/terms/{term_id}/counter"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", consumer.to_string()))
        .set_json(json!({ "description": "45 days is better" }))
        .to_request();
    let resp = test::call_service(&app, counter_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let counter_term_id = body["id"].as_str().unwrap();
    let parent_term_id = body["parent_term_id"].as_str().unwrap();

    let accept_req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1/deals/{deal_id}/terms/{counter_term_id}/accept"
        ))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .to_request();
    let resp = test::call_service(&app, accept_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let reject_req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1/deals/{deal_id}/terms/{parent_term_id}/reject"
        ))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .to_request();
    let resp = test::call_service(&app, reject_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let propose_req2 = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/terms"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(json!({
            "term_type": "PAYMENT_TERMS",
            "term_name": "Net 15",
            "description": "Payment within 15 days"
        }))
        .to_request();
    let resp = test::call_service(&app, propose_req2).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let term2_id = body["id"].as_str().unwrap();

    let withdraw_req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1/deals/{deal_id}/terms/{term2_id}/withdraw"
        ))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .to_request();
    let resp = test::call_service(&app, withdraw_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}
