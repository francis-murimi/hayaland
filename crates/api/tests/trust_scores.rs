mod common;

use actix_web::http::header;
use actix_web::{http::StatusCode, test, web::Data, App};
use api::routes;
use serde_json::json;

#[sqlx::test(migrations = "../../migrations")]
async fn get_trust_score_returns_default_for_new_party(pool: sqlx::PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let user_id = common::create_active_user(&pool, "trust-get@example.com").await;
    let token = common::auth_token(user_id, vec!["parties:write".to_string()]).await;

    let create_resp = test::TestRequest::post()
        .uri("/api/v1/parties")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(json!({
            "party_type": "ORGANIZATION",
            "display_name": "Trust Get Party",
            "email": "trust-get-party@example.com",
            "roles": [],
        }))
        .send_request(&app)
        .await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(create_resp).await;
    let party_id = body["id"].as_str().unwrap();

    let get_resp = test::TestRequest::get()
        .uri(&format!("/api/v1/parties/{party_id}/trust"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .send_request(&app)
        .await;
    assert_eq!(get_resp.status(), StatusCode::OK);
    let trust: serde_json::Value = test::read_body_json(get_resp).await;
    assert_eq!(trust["overall_score"], 0.0);
    assert!(trust["tier"].as_str().unwrap().contains("BRONZE"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn recalculate_trust_score_requires_admin_scope(pool: sqlx::PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let user_id = common::create_active_user(&pool, "trust-recalc@example.com").await;
    let token = common::auth_token(user_id, vec!["parties:write".to_string()]).await;

    let create_resp = test::TestRequest::post()
        .uri("/api/v1/parties")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(json!({
            "party_type": "ORGANIZATION",
            "display_name": "Trust Recalc Party",
            "email": "trust-recalc-party@example.com",
            "roles": [],
        }))
        .send_request(&app)
        .await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(create_resp).await;
    let party_id = body["id"].as_str().unwrap();

    let recalc_resp = test::TestRequest::post()
        .uri(&format!("/api/v1/parties/{party_id}/trust/recalculate"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .send_request(&app)
        .await;
    assert_eq!(recalc_resp.status(), StatusCode::FORBIDDEN);

    let admin_token = common::auth_token(user_id, vec!["admin:trust".to_string()]).await;
    let admin_recalc_resp = test::TestRequest::post()
        .uri(&format!("/api/v1/parties/{party_id}/trust/recalculate"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
        .send_request(&app)
        .await;
    assert_eq!(admin_recalc_resp.status(), StatusCode::OK);
    let trust: serde_json::Value = test::read_body_json(admin_recalc_resp).await;
    assert!(trust["overall_score"].as_f64().unwrap() > 0.0);
    assert!(trust["tier"].as_str().unwrap().contains("BRONZE"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn recalculate_trust_score_syncs_public_cache(pool: sqlx::PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let user_id = common::create_active_user(&pool, "trust-cache@example.com").await;
    let token = common::auth_token(user_id, vec!["parties:write".to_string()]).await;

    let create_resp = test::TestRequest::post()
        .uri("/api/v1/parties")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(json!({
            "party_type": "ORGANIZATION",
            "display_name": "Trust Cache Party",
            "email": "trust-cache-party@example.com",
            "roles": [],
        }))
        .send_request(&app)
        .await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(create_resp).await;
    let party_id = body["id"].as_str().unwrap();

    let admin_token = common::auth_token(user_id, vec!["admin:*".to_string()]).await;
    let recalc_resp = test::TestRequest::post()
        .uri(&format!("/api/v1/parties/{party_id}/trust/recalculate"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
        .send_request(&app)
        .await;
    assert_eq!(recalc_resp.status(), StatusCode::OK);

    let party_resp = test::TestRequest::get()
        .uri(&format!("/api/v1/parties/{party_id}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .send_request(&app)
        .await;
    assert_eq!(party_resp.status(), StatusCode::OK);
    let party: serde_json::Value = test::read_body_json(party_resp).await;
    assert!(party["trust_score"].as_f64().unwrap() > 0.0);
}
