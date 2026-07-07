use actix_web::http::header;
use actix_web::{http::StatusCode, test, web::Data, App};
use api::routes;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

mod common;

const CATEGORY_ID: &str = "f6a7b8c9-d0e1-2345-fabc-456789012345";

async fn create_party(
    app: &impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
    >,
    token: &str,
    email: &str,
    roles: Vec<&str>,
) -> Uuid {
    let req = test::TestRequest::post()
        .uri("/api/v1/parties")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(json!({
            "display_name": email.replace(['@', '.'], " "),
            "email": email,
            "party_type": "INDIVIDUAL",
            "roles": roles
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    Uuid::parse_str(body["id"].as_str().unwrap()).unwrap()
}

async fn create_resource(
    app: &impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
    >,
    token: &str,
    party_id: Uuid,
    name: &str,
) -> Uuid {
    let req = test::TestRequest::post()
        .uri("/api/v1/resources")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", party_id.to_string()))
        .set_json(json!({
            "resourceTypeId": CATEGORY_ID,
            "resourceName": name,
            "quantity": "10",
            "quantityUnit": "units"
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    let status = resp.status();
    if status != StatusCode::CREATED {
        let body = test::read_body(resp).await;
        let text = String::from_utf8_lossy(&body);
        panic!("create_resource failed: {} {}", status, text);
    }
    let body: serde_json::Value = test::read_body_json(resp).await;
    Uuid::parse_str(body["id"].as_str().unwrap()).unwrap()
}

async fn create_need(
    app: &impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
    >,
    token: &str,
    party_id: Uuid,
) -> Uuid {
    let req = test::TestRequest::post()
        .uri("/api/v1/needs")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", party_id.to_string()))
        .set_json(json!({
            "needCategoryId": CATEGORY_ID,
            "needDescription": "Looking for test resources",
            "requiredQuantity": "5",
            "quantityUnit": "units",
            "maxBudget": "100"
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    let status = resp.status();
    if status != StatusCode::CREATED {
        let body = test::read_body(resp).await;
        let text = String::from_utf8_lossy(&body);
        panic!("create_need failed: {} {}", status, text);
    }
    let body: serde_json::Value = test::read_body_json(resp).await;
    Uuid::parse_str(body["id"].as_str().unwrap()).unwrap()
}

async fn create_enhancement(
    app: &impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
    >,
    token: &str,
    party_id: Uuid,
) -> Uuid {
    let req = test::TestRequest::post()
        .uri("/api/v1/enhancements")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", party_id.to_string()))
        .set_json(json!({
            "enhancementTypeId": CATEGORY_ID,
            "enhancementName": "Test Enhancement"
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    let status = resp.status();
    if status != StatusCode::CREATED {
        let body = test::read_body(resp).await;
        let text = String::from_utf8_lossy(&body);
        panic!("create_enhancement failed: {} {}", status, text);
    }
    let body: serde_json::Value = test::read_body_json(resp).await;
    Uuid::parse_str(body["id"].as_str().unwrap()).unwrap()
}

#[sqlx::test(migrations = "../../migrations")]
async fn match_suggestion_end_to_end_lifecycle(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let user_id = common::create_active_user(&pool, "match_api_user@example.com").await;
    let token = common::auth_token(
        user_id,
        vec![
            "admin:*".to_string(),
            "matches:read".to_string(),
            "matches:write".to_string(),
            "catalog:write".to_string(),
            "parties:write".to_string(),
        ],
    )
    .await;

    let supplier = create_party(
        &app,
        &token,
        "match_api_supplier@example.com",
        vec!["SUPPLIER"],
    )
    .await;
    let consumer = create_party(
        &app,
        &token,
        "match_api_consumer@example.com",
        vec!["CONSUMER"],
    )
    .await;
    let enhancer = create_party(
        &app,
        &token,
        "match_api_enhancer@example.com",
        vec!["ENHANCER"],
    )
    .await;

    create_resource(&app, &token, supplier, "Resource").await;
    create_need(&app, &token, consumer).await;
    create_enhancement(&app, &token, enhancer).await;

    let generate_req = test::TestRequest::post()
        .uri("/api/v1/matches/generate")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(json!({ "maxSuggestions": 10 }))
        .to_request();
    let generate_resp = test::call_service(&app, generate_req).await;
    let generate_status = generate_resp.status();
    if generate_status != StatusCode::CREATED {
        let body = test::read_body(generate_resp).await;
        let text = String::from_utf8_lossy(&body);
        panic!("generate_matches failed: {} {}", generate_status, text);
    }
    let generated: Vec<serde_json::Value> = test::read_body_json(generate_resp).await;
    assert_eq!(generated.len(), 1);
    let match_id = Uuid::parse_str(generated[0]["id"].as_str().unwrap()).unwrap();

    let list_req = test::TestRequest::get()
        .uri("/api/v1/matches")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", consumer.to_string()))
        .to_request();
    let list_resp = test::call_service(&app, list_req).await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    let listed: Vec<serde_json::Value> = test::read_body_json(list_resp).await;
    assert_eq!(listed.len(), 1);

    let respond_req = test::TestRequest::post()
        .uri(&format!("/api/v1/matches/{match_id}/respond"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .set_json(json!({ "response": "accept" }))
        .to_request();
    let respond_resp = test::call_service(&app, respond_req).await;
    assert_eq!(respond_resp.status(), StatusCode::NO_CONTENT);

    let counts_req = test::TestRequest::get()
        .uri("/api/v1/matches/counts")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", supplier.to_string()))
        .to_request();
    let counts_resp = test::call_service(&app, counts_req).await;
    assert_eq!(counts_resp.status(), StatusCode::OK);
    let counts: serde_json::Value = test::read_body_json(counts_resp).await;
    assert_eq!(counts["accepted"], 1);

    let admin_counts_req = test::TestRequest::get()
        .uri("/api/v1/admin/matches/counts")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let admin_counts_resp = test::call_service(&app, admin_counts_req).await;
    assert_eq!(admin_counts_resp.status(), StatusCode::OK);
    let admin_counts: serde_json::Value = test::read_body_json(admin_counts_resp).await;
    assert_eq!(admin_counts["accepted"], 1);

    let admin_list_req = test::TestRequest::get()
        .uri("/api/v1/admin/matches")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let admin_list_resp = test::call_service(&app, admin_list_req).await;
    assert_eq!(admin_list_resp.status(), StatusCode::OK);
    let admin_list: Vec<serde_json::Value> = test::read_body_json(admin_list_resp).await;
    assert_eq!(admin_list.len(), 1);

    let admin_update_req = test::TestRequest::patch()
        .uri(&format!("/api/v1/admin/matches/{match_id}/status"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(json!({ "new_status": "DECLINED", "reason": "test" }))
        .to_request();
    let admin_update_resp = test::call_service(&app, admin_update_req).await;
    assert_eq!(admin_update_resp.status(), StatusCode::NO_CONTENT);

    let admin_delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/admin/parties/{consumer}/matches"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let admin_delete_resp = test::call_service(&app, admin_delete_req).await;
    assert_eq!(admin_delete_resp.status(), StatusCode::OK);
    let delete_body: serde_json::Value = test::read_body_json(admin_delete_resp).await;
    assert_eq!(delete_body["deleted"], 1);

    let admin_delete_all_req = test::TestRequest::delete()
        .uri("/api/v1/admin/matches")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let admin_delete_all_resp = test::call_service(&app, admin_delete_all_req).await;
    assert_eq!(admin_delete_all_resp.status(), StatusCode::OK);
    let delete_all_body: serde_json::Value = test::read_body_json(admin_delete_all_resp).await;
    assert_eq!(delete_all_body["deleted"], 0);
}
