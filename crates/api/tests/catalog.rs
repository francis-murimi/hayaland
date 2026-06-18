use actix_web::http::header;
use actix_web::{http::StatusCode, test, web::Data, App};
use api::routes;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

mod common;

const AGRICULTURE_DOMAIN_ID: &str = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
const FARMLAND_RESOURCE_TYPE_ID: &str = "f6a7b8c9-d0e1-2345-fabc-456789012345";
const CROP_PRODUCE_NEED_TYPE_ID: &str = "a7b8c9d0-e1f2-3456-abcd-567890123456";
const AGRO_INPUTS_ENHANCEMENT_TYPE_ID: &str = "b8c9d0e1-f2a3-4567-bcde-678901234567";

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
            "resourceTypeId": FARMLAND_RESOURCE_TYPE_ID,
            "resourceName": name,
            "quantity": "10",
            "quantityUnit": "acres"
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    Uuid::parse_str(body["id"].as_str().unwrap()).unwrap()
}

async fn create_deal_between(
    app: &impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
    >,
    token: &str,
    supplier_party_id: Uuid,
    consumer_party_id: Uuid,
    enhancer_party_id: Uuid,
) -> Uuid {
    let req = test::TestRequest::post()
        .uri("/api/v1/deals")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", supplier_party_id.to_string()))
        .set_json(json!({
            "title": "Test deal",
            "domain_category_id": AGRICULTURE_DOMAIN_ID,
            "consumer_party_id": consumer_party_id,
            "enhancer_party_id": enhancer_party_id
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    Uuid::parse_str(body["id"].as_str().unwrap()).unwrap()
}

#[sqlx::test(migrations = "../../migrations")]
async fn anonymous_list_catalog_returns_200(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    for endpoint in ["/api/v1/resources", "/api/v1/needs", "/api/v1/enhancements"] {
        let resp = test::TestRequest::get()
            .uri(endpoint)
            .send_request(&app)
            .await;
        assert_eq!(resp.status(), StatusCode::OK, "{endpoint} should be public");
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(body["items"].is_array());
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn anonymous_post_resource_returns_401(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let resp = test::TestRequest::post()
        .uri("/api/v1/resources")
        .set_json(json!({
            "resourceTypeId": FARMLAND_RESOURCE_TYPE_ID,
            "resourceName": "Anonymous resource",
            "quantity": "1",
            "quantityUnit": "acre"
        }))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../../migrations")]
async fn anonymous_get_resource_returns_public_view(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let user = common::create_active_user(&pool, "owner@example.com").await;
    let token = common::auth_token(
        user,
        vec!["parties:write".to_string(), "catalog:write".to_string()],
    )
    .await;
    let party_id = create_party(&app, &token, "owner-party@example.com", vec!["SUPPLIER"]).await;
    let resource_id = create_resource(&app, &token, party_id, "Public farmland").await;

    let resp = test::TestRequest::get()
        .uri(&format!("/api/v1/resources/{resource_id}"))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["resource_name"], "Public farmland");
    assert!(body.get("admin_notes").is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn owner_can_crud_resource(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let user = common::create_active_user(&pool, "owner2@example.com").await;
    let token = common::auth_token(
        user,
        vec!["parties:write".to_string(), "catalog:write".to_string()],
    )
    .await;
    let party_id = create_party(&app, &token, "owner2-party@example.com", vec!["SUPPLIER"]).await;

    let resp = test::TestRequest::post()
        .uri("/api/v1/resources")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", party_id.to_string()))
        .set_json(json!({
            "resourceTypeId": FARMLAND_RESOURCE_TYPE_ID,
            "resourceName": "Owner farmland",
            "quantity": "5",
            "quantityUnit": "acres"
        }))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let resource_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();

    let resp = test::TestRequest::patch()
        .uri(&format!("/api/v1/resources/{resource_id}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", party_id.to_string()))
        .set_json(json!({ "resourceName": "Updated farmland" }))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["resource_name"], "Updated farmland");

    let resp = test::TestRequest::delete()
        .uri(&format!("/api/v1/resources/{resource_id}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", party_id.to_string()))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[sqlx::test(migrations = "../../migrations")]
async fn non_owner_cannot_mutate_resource(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let owner = common::create_active_user(&pool, "owner3@example.com").await;
    let owner_token = common::auth_token(
        owner,
        vec!["parties:write".to_string(), "catalog:write".to_string()],
    )
    .await;
    let owner_party = create_party(
        &app,
        &owner_token,
        "owner3-party@example.com",
        vec!["SUPPLIER"],
    )
    .await;
    let resource_id = create_resource(&app, &owner_token, owner_party, "Protected").await;

    let other = common::create_active_user(&pool, "other@example.com").await;
    let other_token = common::auth_token(
        other,
        vec!["parties:write".to_string(), "catalog:write".to_string()],
    )
    .await;
    let other_party = create_party(
        &app,
        &other_token,
        "other-party@example.com",
        vec!["SUPPLIER"],
    )
    .await;

    let resp = test::TestRequest::patch()
        .uri(&format!("/api/v1/resources/{resource_id}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {other_token}")))
        .insert_header(("X-Party-ID", other_party.to_string()))
        .set_json(json!({ "resourceName": "Hacked" }))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../migrations")]
async fn admin_hide_excludes_from_public_list(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let user = common::create_active_user(&pool, "adminuser@example.com").await;
    let token = common::auth_token(
        user,
        vec![
            "parties:write".to_string(),
            "catalog:write".to_string(),
            "admin:catalog".to_string(),
        ],
    )
    .await;
    let party_id = create_party(&app, &token, "admin-party@example.com", vec!["SUPPLIER"]).await;
    let resource_id = create_resource(&app, &token, party_id, "Hidden item").await;

    let resp = test::TestRequest::get()
        .uri("/api/v1/resources")
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let before = body["items"].as_array().unwrap().len();

    let resp = test::TestRequest::patch()
        .uri(&format!(
            "/api/v1/admin/catalog/resources/{resource_id}/flags"
        ))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(json!({ "platformHidden": true }))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = test::TestRequest::get()
        .uri("/api/v1/resources")
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let after = body["items"].as_array().unwrap().len();
    assert_eq!(after, before - 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn search_query_works(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let user = common::create_active_user(&pool, "searcher@example.com").await;
    let token = common::auth_token(
        user,
        vec!["parties:write".to_string(), "catalog:write".to_string()],
    )
    .await;
    let party_id = create_party(&app, &token, "search-party@example.com", vec!["SUPPLIER"]).await;
    create_resource(&app, &token, party_id, "Unique hay bales").await;

    let resp = test::TestRequest::get()
        .uri("/api/v1/resources/search?text=Unique%20hay")
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(!body["items"].as_array().unwrap().is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn contact_owner_creates_conversation_and_message(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let owner = common::create_active_user(&pool, "contact-owner@example.com").await;
    let owner_token = common::auth_token(
        owner,
        vec!["parties:write".to_string(), "catalog:write".to_string()],
    )
    .await;
    let owner_party = create_party(
        &app,
        &owner_token,
        "contact-owner-party@example.com",
        vec!["SUPPLIER"],
    )
    .await;
    let resource_id = create_resource(&app, &owner_token, owner_party, "Contactable").await;

    let inquirer = common::create_active_user(&pool, "inquirer@example.com").await;
    let inquirer_token = common::auth_token(
        inquirer,
        vec!["parties:write".to_string(), "catalog:write".to_string()],
    )
    .await;
    let inquirer_party = create_party(
        &app,
        &inquirer_token,
        "inquirer-party@example.com",
        vec!["CONSUMER"],
    )
    .await;

    let resp = test::TestRequest::post()
        .uri(&format!("/api/v1/catalog/resources/{resource_id}/contact"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {inquirer_token}")))
        .insert_header(("X-Party-ID", inquirer_party.to_string()))
        .set_json(json!({ "message": "Is this still available?" }))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let conversation_id = Uuid::parse_str(body["conversation_id"].as_str().unwrap()).unwrap();
    let message_id = Uuid::parse_str(body["message_id"].as_str().unwrap()).unwrap();
    assert_ne!(conversation_id, Uuid::nil());
    assert_ne!(message_id, Uuid::nil());

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE conversation_id = $1 AND id = $2")
            .bind(conversation_id)
            .bind(message_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn owner_opt_out_blocks_contact(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let owner = common::create_active_user(&pool, "optout-owner@example.com").await;
    let owner_token = common::auth_token(
        owner,
        vec!["parties:write".to_string(), "catalog:write".to_string()],
    )
    .await;
    let owner_party = create_party(
        &app,
        &owner_token,
        "optout-party@example.com",
        vec!["SUPPLIER"],
    )
    .await;
    let resource_id = create_resource(&app, &owner_token, owner_party, "Not contactable").await;

    let resp = test::TestRequest::patch()
        .uri(&format!("/api/v1/parties/{owner_party}/catalog-settings"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {owner_token}")))
        .insert_header(("X-Party-ID", owner_party.to_string()))
        .set_json(json!({ "acceptsCatalogInquiries": false }))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let inquirer = common::create_active_user(&pool, "inquirer2@example.com").await;
    let inquirer_token = common::auth_token(
        inquirer,
        vec!["parties:write".to_string(), "catalog:write".to_string()],
    )
    .await;
    let inquirer_party = create_party(
        &app,
        &inquirer_token,
        "inquirer2-party@example.com",
        vec!["CONSUMER"],
    )
    .await;

    let resp = test::TestRequest::post()
        .uri(&format!("/api/v1/catalog/resources/{resource_id}/contact"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {inquirer_token}")))
        .insert_header(("X-Party-ID", inquirer_party.to_string()))
        .set_json(json!({ "message": "Hello" }))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../../migrations")]
async fn deal_binding_creates_deal_bound_copy(pool: PgPool) {
    let _ = tracing_subscriber::fmt::try_init();
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let supplier = common::create_active_user(&pool, "supplier@example.com").await;
    let supplier_token = common::auth_token(
        supplier,
        vec![
            "parties:write".to_string(),
            "catalog:write".to_string(),
            "deals:write".to_string(),
        ],
    )
    .await;
    let supplier_party = create_party(
        &app,
        &supplier_token,
        "supplier-party@example.com",
        vec!["SUPPLIER"],
    )
    .await;
    let resource_id =
        create_resource(&app, &supplier_token, supplier_party, "Bindable farmland").await;

    let consumer = common::create_active_user(&pool, "consumer@example.com").await;
    let consumer_token = common::auth_token(consumer, vec!["parties:write".to_string()]).await;
    let consumer_party = create_party(
        &app,
        &consumer_token,
        "consumer-party@example.com",
        vec!["CONSUMER"],
    )
    .await;

    let enhancer = common::create_active_user(&pool, "enhancer@example.com").await;
    let enhancer_token = common::auth_token(enhancer, vec!["parties:write".to_string()]).await;
    let enhancer_party = create_party(
        &app,
        &enhancer_token,
        "enhancer-party@example.com",
        vec!["ENHANCER"],
    )
    .await;

    let deal_id = create_deal_between(
        &app,
        &supplier_token,
        supplier_party,
        consumer_party,
        enhancer_party,
    )
    .await;

    let resp = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/resource"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {supplier_token}")))
        .insert_header(("X-Party-ID", supplier_party.to_string()))
        .set_json(json!({ "itemId": resource_id }))
        .send_request(&app)
        .await;
    let status = resp.status();
    if status != StatusCode::CREATED {
        let body = test::read_body(resp).await;
        let text = String::from_utf8_lossy(&body);
        eprintln!("bind status: {status:?}, body: {text}");
        panic!("expected 201");
    }
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["catalog_item_id"], resource_id.to_string());

    let resp = test::TestRequest::get()
        .uri(&format!("/api/v1/deals/{deal_id}/resource"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {supplier_token}")))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 1);
    assert_eq!(body["items"][0]["resource_name"], "Bindable farmland");
}

async fn create_need(
    app: &impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
    >,
    token: &str,
    party_id: Uuid,
    description: &str,
) -> Uuid {
    let req = test::TestRequest::post()
        .uri("/api/v1/needs")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", party_id.to_string()))
        .set_json(json!({
            "needCategoryId": CROP_PRODUCE_NEED_TYPE_ID,
            "needDescription": description,
            "requiredQuantity": "100",
            "quantityUnit": "kg"
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
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
    name: &str,
) -> Uuid {
    let req = test::TestRequest::post()
        .uri("/api/v1/enhancements")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", party_id.to_string()))
        .set_json(json!({
            "enhancementTypeId": AGRO_INPUTS_ENHANCEMENT_TYPE_ID,
            "enhancementName": name
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    Uuid::parse_str(body["id"].as_str().unwrap()).unwrap()
}

#[sqlx::test(migrations = "../../migrations")]
async fn need_lifecycle(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let user = common::create_active_user(&pool, "need-owner@example.com").await;
    let token = common::auth_token(
        user,
        vec!["parties:write".to_string(), "catalog:write".to_string()],
    )
    .await;
    let party_id = create_party(
        &app,
        &token,
        "need-owner-party@example.com",
        vec!["CONSUMER"],
    )
    .await;

    let resp = test::TestRequest::post()
        .uri("/api/v1/needs")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", party_id.to_string()))
        .set_json(json!({
            "needCategoryId": CROP_PRODUCE_NEED_TYPE_ID,
            "needDescription": "I need fresh tomatoes",
            "requiredQuantity": "50",
            "quantityUnit": "kg",
            "priority": "HIGH"
        }))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let need_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();

    let resp = test::TestRequest::get()
        .uri(&format!("/api/v1/needs/{need_id}"))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["need_description"], "I need fresh tomatoes");
    assert!(body.get("admin_notes").is_none());

    let resp = test::TestRequest::get()
        .uri("/api/v1/needs")
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 1);

    let resp = test::TestRequest::get()
        .uri("/api/v1/needs/search?text=tomatoes")
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(!body["items"].as_array().unwrap().is_empty());

    let resp = test::TestRequest::patch()
        .uri(&format!("/api/v1/needs/{need_id}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", party_id.to_string()))
        .set_json(json!({ "needDescription": "I need ripe tomatoes" }))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["need_description"], "I need ripe tomatoes");

    let resp = test::TestRequest::delete()
        .uri(&format!("/api/v1/needs/{need_id}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", party_id.to_string()))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[sqlx::test(migrations = "../../migrations")]
async fn enhancement_lifecycle(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let user = common::create_active_user(&pool, "enhancement-owner@example.com").await;
    let token = common::auth_token(
        user,
        vec!["parties:write".to_string(), "catalog:write".to_string()],
    )
    .await;
    let party_id = create_party(
        &app,
        &token,
        "enhancement-owner-party@example.com",
        vec!["ENHANCER"],
    )
    .await;

    let resp = test::TestRequest::post()
        .uri("/api/v1/enhancements")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", party_id.to_string()))
        .set_json(json!({
            "enhancementTypeId": AGRO_INPUTS_ENHANCEMENT_TYPE_ID,
            "enhancementName": "Soil analysis",
            "description": "Professional soil testing service",
            "estimatedCompletionDays": 7
        }))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let enhancement_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();

    let resp = test::TestRequest::get()
        .uri(&format!("/api/v1/enhancements/{enhancement_id}"))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["enhancement_name"], "Soil analysis");
    assert!(body.get("admin_notes").is_none());

    let resp = test::TestRequest::get()
        .uri("/api/v1/enhancements")
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 1);

    let resp = test::TestRequest::patch()
        .uri(&format!("/api/v1/enhancements/{enhancement_id}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", party_id.to_string()))
        .set_json(json!({ "enhancementName": "Advanced soil analysis" }))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["enhancement_name"], "Advanced soil analysis");

    let resp = test::TestRequest::delete()
        .uri(&format!("/api/v1/enhancements/{enhancement_id}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", party_id.to_string()))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[sqlx::test(migrations = "../../migrations")]
async fn admin_flags_need_and_enhancement(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let user = common::create_active_user(&pool, "admin2@example.com").await;
    let token = common::auth_token(
        user,
        vec![
            "parties:write".to_string(),
            "catalog:write".to_string(),
            "admin:catalog".to_string(),
        ],
    )
    .await;
    let consumer_party = create_party(
        &app,
        &token,
        "admin-consumer-party@example.com",
        vec!["CONSUMER"],
    )
    .await;
    let enhancer_party = create_party(
        &app,
        &token,
        "admin-enhancer-party@example.com",
        vec!["ENHANCER"],
    )
    .await;
    let need_id = create_need(&app, &token, consumer_party, "Hidden need").await;
    let enhancement_id =
        create_enhancement(&app, &token, enhancer_party, "Featured enhancement").await;

    let resp = test::TestRequest::patch()
        .uri(&format!("/api/v1/admin/catalog/needs/{need_id}/flags"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(json!({ "platformHidden": true }))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = test::TestRequest::get()
        .uri("/api/v1/needs")
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["items"].as_array().unwrap().is_empty());

    let resp = test::TestRequest::patch()
        .uri(&format!(
            "/api/v1/admin/catalog/enhancements/{enhancement_id}/flags"
        ))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(json!({ "platformFeatured": true, "adminNotes": "promoted" }))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["platform_featured"], true);
    assert_eq!(body["admin_notes"], "promoted");
}

#[sqlx::test(migrations = "../../migrations")]
async fn discovery_and_categories_are_public(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let user = common::create_active_user(&pool, "discover@example.com").await;
    let token = common::auth_token(
        user,
        vec!["parties:write".to_string(), "catalog:write".to_string()],
    )
    .await;
    let supplier_party = create_party(
        &app,
        &token,
        "discover-supplier-party@example.com",
        vec!["SUPPLIER"],
    )
    .await;
    create_resource(&app, &token, supplier_party, "Discoverable farmland").await;

    let resp = test::TestRequest::get()
        .uri("/api/v1/discovery/domains")
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let domains = body["domains"].as_array().unwrap();
    assert!(!domains.is_empty());
    let agriculture = domains
        .iter()
        .find(|d| d["categoryCode"] == "agriculture")
        .unwrap();
    assert!(agriculture["resourceCount"].as_i64().unwrap() > 0);

    let domain_id = agriculture["id"].as_str().unwrap();
    let resp = test::TestRequest::get()
        .uri(&format!("/api/v1/discovery/domains/{domain_id}"))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["categoryCode"], "agriculture");
    assert!(!body["childCategories"].as_array().unwrap().is_empty());

    for endpoint in [
        "/api/v1/catalog/categories",
        "/api/v1/resources/categories",
        "/api/v1/needs/categories",
        "/api/v1/enhancements/categories",
    ] {
        let resp = test::TestRequest::get()
            .uri(endpoint)
            .send_request(&app)
            .await;
        assert_eq!(resp.status(), StatusCode::OK, "{endpoint} should be public");
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(body["categories"].is_array());
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn contact_need_and_enhancement_owners(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let consumer = common::create_active_user(&pool, "need-contact-owner@example.com").await;
    let consumer_token = common::auth_token(
        consumer,
        vec!["parties:write".to_string(), "catalog:write".to_string()],
    )
    .await;
    let consumer_party = create_party(
        &app,
        &consumer_token,
        "need-contact-party@example.com",
        vec!["CONSUMER"],
    )
    .await;
    let need_id = create_need(&app, &consumer_token, consumer_party, "Contactable need").await;

    let enhancer = common::create_active_user(&pool, "enhancement-contact-owner@example.com").await;
    let enhancer_token = common::auth_token(
        enhancer,
        vec!["parties:write".to_string(), "catalog:write".to_string()],
    )
    .await;
    let enhancer_party = create_party(
        &app,
        &enhancer_token,
        "enhancement-contact-party@example.com",
        vec!["ENHANCER"],
    )
    .await;
    let enhancement_id = create_enhancement(
        &app,
        &enhancer_token,
        enhancer_party,
        "Contactable enhancement",
    )
    .await;

    let inquirer = common::create_active_user(&pool, "catalog-inquirer@example.com").await;
    let inquirer_token = common::auth_token(
        inquirer,
        vec!["parties:write".to_string(), "catalog:write".to_string()],
    )
    .await;
    let inquirer_party = create_party(
        &app,
        &inquirer_token,
        "catalog-inquirer-party@example.com",
        vec!["SUPPLIER"],
    )
    .await;

    for (endpoint, key) in [
        (format!("/api/v1/catalog/needs/{need_id}/contact"), "need"),
        (
            format!("/api/v1/catalog/enhancements/{enhancement_id}/contact"),
            "enhancement",
        ),
    ] {
        let resp = test::TestRequest::post()
            .uri(&endpoint)
            .insert_header((header::AUTHORIZATION, format!("Bearer {inquirer_token}")))
            .insert_header(("X-Party-ID", inquirer_party.to_string()))
            .set_json(json!({ "message": format!("Interested in your {key}") }))
            .send_request(&app)
            .await;
        assert_eq!(resp.status(), StatusCode::CREATED, "contact {key} failed");
        let body: serde_json::Value = test::read_body_json(resp).await;
        let conversation_id = Uuid::parse_str(body["conversation_id"].as_str().unwrap()).unwrap();
        assert_ne!(conversation_id, Uuid::nil());
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn deal_binding_need_and_enhancement(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let supplier = common::create_active_user(&pool, "deal-supplier2@example.com").await;
    let supplier_token = common::auth_token(
        supplier,
        vec![
            "parties:write".to_string(),
            "catalog:write".to_string(),
            "deals:write".to_string(),
        ],
    )
    .await;
    let supplier_party = create_party(
        &app,
        &supplier_token,
        "deal-supplier2-party@example.com",
        vec!["SUPPLIER"],
    )
    .await;
    let resource_id =
        create_resource(&app, &supplier_token, supplier_party, "Resource for deal").await;

    let consumer = common::create_active_user(&pool, "deal-consumer2@example.com").await;
    let consumer_token = common::auth_token(
        consumer,
        vec![
            "parties:write".to_string(),
            "catalog:write".to_string(),
            "deals:write".to_string(),
        ],
    )
    .await;
    let consumer_party = create_party(
        &app,
        &consumer_token,
        "deal-consumer2-party@example.com",
        vec!["CONSUMER"],
    )
    .await;
    let need_id = create_need(&app, &consumer_token, consumer_party, "Need for deal").await;

    let enhancer = common::create_active_user(&pool, "deal-enhancer2@example.com").await;
    let enhancer_token = common::auth_token(
        enhancer,
        vec![
            "parties:write".to_string(),
            "catalog:write".to_string(),
            "deals:write".to_string(),
        ],
    )
    .await;
    let enhancer_party = create_party(
        &app,
        &enhancer_token,
        "deal-enhancer2-party@example.com",
        vec!["ENHANCER"],
    )
    .await;
    let enhancement_id = create_enhancement(
        &app,
        &enhancer_token,
        enhancer_party,
        "Enhancement for deal",
    )
    .await;

    let deal_id = create_deal_between(
        &app,
        &supplier_token,
        supplier_party,
        consumer_party,
        enhancer_party,
    )
    .await;

    for (actor_token, actor_party, endpoint, item_id, name) in [
        (
            &supplier_token,
            supplier_party,
            format!("/api/v1/deals/{deal_id}/resource"),
            resource_id,
            "resource",
        ),
        (
            &consumer_token,
            consumer_party,
            format!("/api/v1/deals/{deal_id}/need"),
            need_id,
            "need",
        ),
        (
            &enhancer_token,
            enhancer_party,
            format!("/api/v1/deals/{deal_id}/enhancement"),
            enhancement_id,
            "enhancement",
        ),
    ] {
        let resp = test::TestRequest::post()
            .uri(&endpoint)
            .insert_header((header::AUTHORIZATION, format!("Bearer {actor_token}")))
            .insert_header(("X-Party-ID", actor_party.to_string()))
            .set_json(json!({ "itemId": item_id }))
            .send_request(&app)
            .await;
        let status = resp.status();
        if status != StatusCode::CREATED {
            let body = test::read_body(resp).await;
            let text = String::from_utf8_lossy(&body);
            panic!("bind {name} failed: {status:?}, {text}");
        }

        let resp = test::TestRequest::get()
            .uri(&endpoint)
            .insert_header((header::AUTHORIZATION, format!("Bearer {actor_token}")))
            .send_request(&app)
            .await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["items"].as_array().unwrap().len(), 1);
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn invalid_catalog_search_params_return_400(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    for endpoint in ["/api/v1/resources", "/api/v1/needs", "/api/v1/enhancements"] {
        let resp = test::TestRequest::get()
            .uri(&format!("{endpoint}?status=invalid"))
            .send_request(&app)
            .await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "{endpoint} status");

        let resp = test::TestRequest::get()
            .uri(&format!("{endpoint}?sort=invalid"))
            .send_request(&app)
            .await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "{endpoint} sort");
    }
}
