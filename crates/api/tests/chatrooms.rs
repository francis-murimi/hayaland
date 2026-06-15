use actix_web::http::header;
use actix_web::{http::StatusCode, test, web::Data, App};
use api::routes;
use sqlx::PgPool;
use uuid::Uuid;

mod common;

#[sqlx::test(migrations = "../../migrations")]
async fn create_and_join_public_chat_room(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let creator = common::create_user(&pool, "creator@example.com").await;
    let joiner = common::create_user(&pool, "joiner@example.com").await;

    let creator_token = common::auth_token(creator, vec!["chatrooms:write".to_string()]).await;
    let req = test::TestRequest::post()
        .uri("/api/v1/chatrooms")
        .insert_header((header::AUTHORIZATION, format!("Bearer {creator_token}")))
        .set_json(serde_json::json!({
            "name": "Public Room",
            "description": "A public room",
            "roomType": "PUBLIC"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let room_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();

    let req = test::TestRequest::get()
        .uri("/api/v1/chatrooms")
        .insert_header((header::AUTHORIZATION, format!("Bearer {creator_token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(!body.as_array().unwrap().is_empty());

    let joiner_token = common::auth_token(joiner, vec!["messages:write".to_string()]).await;
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/chatrooms/{room_id}/members"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {joiner_token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/chatrooms/{room_id}/messages"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {joiner_token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[sqlx::test(migrations = "../../migrations")]
async fn send_message_to_chat_room(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let creator = common::create_user(&pool, "room_creator@example.com").await;
    let creator_token = common::auth_token(creator, vec!["chatrooms:write".to_string()]).await;
    let req = test::TestRequest::post()
        .uri("/api/v1/chatrooms")
        .insert_header((header::AUTHORIZATION, format!("Bearer {creator_token}")))
        .set_json(serde_json::json!({
            "name": "Messaging Room",
            "roomType": "PUBLIC"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let room_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();

    let req = test::TestRequest::post()
        .uri("/api/v1/messages")
        .insert_header((header::AUTHORIZATION, format!("Bearer {creator_token}")))
        .set_json(serde_json::json!({
            "recipientType": "ROOM",
            "recipientRoomId": room_id,
            "messageType": "TEXT",
            "content": "hello room"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["contentPlaintext"], "hello room");
    assert_eq!(body["recipientRoomId"], room_id.to_string());
}

#[sqlx::test(migrations = "../../migrations")]
async fn update_and_delete_chat_room(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let creator = common::create_user(&pool, "room_admin@example.com").await;
    let creator_token = common::auth_token(
        creator,
        vec![
            "chatrooms:write".to_string(),
            "chatrooms:moderate".to_string(),
        ],
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/chatrooms")
        .insert_header((header::AUTHORIZATION, format!("Bearer {creator_token}")))
        .set_json(serde_json::json!({
            "name": "Updatable Room",
            "roomType": "PUBLIC"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let room_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/chatrooms/{room_id}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {creator_token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["name"], "Updatable Room");

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/chatrooms/{room_id}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {creator_token}")))
        .set_json(serde_json::json!({ "name": "Updated Room" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["name"], "Updated Room");

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/chatrooms/{room_id}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {creator_token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[sqlx::test(migrations = "../../migrations")]
async fn leave_chat_room(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let creator = common::create_user(&pool, "leave_creator@example.com").await;
    let member = common::create_user(&pool, "leaver@example.com").await;

    let creator_token = common::auth_token(creator, vec!["chatrooms:write".to_string()]).await;
    let req = test::TestRequest::post()
        .uri("/api/v1/chatrooms")
        .insert_header((header::AUTHORIZATION, format!("Bearer {creator_token}")))
        .set_json(serde_json::json!({
            "name": "Leavable Room",
            "roomType": "PUBLIC"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let room_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();

    let member_token = common::auth_token(member, vec!["messages:write".to_string()]).await;
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/chatrooms/{room_id}/members"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {member_token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/chatrooms/{room_id}/members/me"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {member_token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[sqlx::test(migrations = "../../migrations")]
async fn leave_and_manage_chat_room_membership(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let creator = common::create_user(&pool, "membership_creator@example.com").await;
    let member = common::create_user(&pool, "member@example.com").await;

    let creator_token = common::auth_token(
        creator,
        vec![
            "chatrooms:write".to_string(),
            "chatrooms:moderate".to_string(),
        ],
    )
    .await;
    let req = test::TestRequest::post()
        .uri("/api/v1/chatrooms")
        .insert_header((header::AUTHORIZATION, format!("Bearer {creator_token}")))
        .set_json(serde_json::json!({
            "name": "Membership Room",
            "roomType": "PUBLIC"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let room_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();

    let member_token = common::auth_token(member, vec!["messages:write".to_string()]).await;
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/chatrooms/{room_id}/members"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {member_token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let membership_id = body["id"].as_str().unwrap();

    let req = test::TestRequest::patch()
        .uri(&format!(
            "/api/v1/chatrooms/{room_id}/members/{membership_id}/role"
        ))
        .insert_header((header::AUTHORIZATION, format!("Bearer {creator_token}")))
        .set_json(serde_json::json!({ "role": "MODERATOR" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1/chatrooms/{room_id}/members/{membership_id}"
        ))
        .insert_header((header::AUTHORIZATION, format!("Bearer {creator_token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}
