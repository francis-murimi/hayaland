use actix_web::http::header;
use actix_web::http::StatusCode;
use actix_web::{test, web::Data, App};
use api::routes;
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::PgPool;

mod common;

#[sqlx::test(migrations = "../../migrations")]
async fn transaction_approval_flow(pool: PgPool) {
    let user_id = common::create_active_user(&pool, "paymentuser@example.com").await;
    let token = common::auth_token_with_roles(
        user_id,
        vec!["user".to_string()],
        vec!["payments:write".to_string(), "payments:read".to_string()],
    )
    .await;

    let party_a = common::create_party_with_role(
        &pool,
        user_id,
        "paymentparty_a@example.com",
        "Party A",
        "SUPPLIER",
    )
    .await;
    let party_b = common::create_party_with_role(
        &pool,
        user_id,
        "paymentparty_b@example.com",
        "Party B",
        "CONSUMER",
    )
    .await;
    let party_c = common::create_party_with_role(
        &pool,
        user_id,
        "paymentparty_c@example.com",
        "Party C",
        "ENHANCER",
    )
    .await;

    let category = common::create_category(&pool, "Payment Domain", "PAY-DOM", "DOMAIN").await;
    let deal_id = common::create_deal_with_parties(
        &pool, party_a, party_a, party_b, party_c, category, "DRAFT",
    )
    .await;

    let txn_id = common::create_pending_transaction(
        &pool,
        deal_id,
        None,
        Some(party_a),
        vec![party_a, party_b],
        Decimal::from(100),
        2,
    )
    .await;

    let state = common::build_state(pool).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let list_req = test::TestRequest::get()
        .uri("/api/v1/payments/transactions/pending-approvals")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", party_a.to_string()))
        .to_request();
    let resp = test::call_service(&app, list_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["total"], 1);

    let get_req = test::TestRequest::get()
        .uri(&format!("/api/v1/payments/transactions/{txn_id}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", party_a.to_string()))
        .to_request();
    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["id"], txn_id.to_string());
    assert_eq!(body["approvalsRequired"], 2);
    assert_eq!(body["approvalsReceived"], 0);

    let approve_req = test::TestRequest::post()
        .uri(&format!("/api/v1/payments/transactions/{txn_id}/approve"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", party_a.to_string()))
        .set_json(json!({ "comment": "looks good" }))
        .to_request();
    let resp = test::call_service(&app, approve_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "PENDING");

    let approve_req_b = test::TestRequest::post()
        .uri(&format!("/api/v1/payments/transactions/{txn_id}/approve"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", party_b.to_string()))
        .set_json(json!({}))
        .to_request();
    let resp = test::call_service(&app, approve_req_b).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "VERIFIED");

    let list_req = test::TestRequest::get()
        .uri("/api/v1/payments/transactions/pending-approvals")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", party_a.to_string()))
        .to_request();
    let resp = test::call_service(&app, list_req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["total"], 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn reject_transaction(pool: PgPool) {
    let user_id = common::create_active_user(&pool, "rejectpaymentuser@example.com").await;
    let token = common::auth_token_with_roles(
        user_id,
        vec!["user".to_string()],
        vec!["payments:write".to_string()],
    )
    .await;

    let party_a = common::create_party_with_role(
        &pool,
        user_id,
        "rejectparty_a@example.com",
        "Reject Party A",
        "SUPPLIER",
    )
    .await;
    let party_b = common::create_party_with_role(
        &pool,
        user_id,
        "rejectparty_b@example.com",
        "Reject Party B",
        "CONSUMER",
    )
    .await;
    let party_c = common::create_party_with_role(
        &pool,
        user_id,
        "rejectparty_c@example.com",
        "Reject Party C",
        "ENHANCER",
    )
    .await;

    let category = common::create_category(&pool, "Reject Domain", "REJ-DOM", "DOMAIN").await;
    let deal_id = common::create_deal_with_parties(
        &pool, party_a, party_a, party_b, party_c, category, "DRAFT",
    )
    .await;

    let txn_id = common::create_pending_transaction(
        &pool,
        deal_id,
        None,
        Some(party_a),
        vec![party_a, party_b],
        Decimal::from(50),
        1,
    )
    .await;

    let state = common::build_state(pool).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let reject_req = test::TestRequest::post()
        .uri(&format!("/api/v1/payments/transactions/{txn_id}/reject"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", party_b.to_string()))
        .set_json(json!({ "comment": "not acceptable" }))
        .to_request();
    let resp = test::call_service(&app, reject_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "REJECTED");
}
