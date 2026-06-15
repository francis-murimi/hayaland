use actix_web::http::header;
use actix_web::{http::StatusCode, test, web::Data, App};
use api::routes;
use sqlx::PgPool;
use uuid::Uuid;

mod common;

#[sqlx::test(migrations = "../../migrations")]
async fn send_and_list_direct_message(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let alice = common::create_user(&pool, "alice@example.com").await;
    let bob = common::create_user(&pool, "bob@example.com").await;

    let token = common::auth_token(alice, vec!["messages:write".to_string()]).await;
    let req = test::TestRequest::post()
        .uri("/api/v1/messages")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(serde_json::json!({
            "recipientType": "USER",
            "recipientUserId": bob,
            "messageType": "TEXT",
            "content": "hello bob"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["contentPlaintext"], "hello bob");
    assert_eq!(body["recipientUserId"], bob.to_string());

    let req = test::TestRequest::get()
        .uri("/api/v1/conversations")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(!body.as_array().unwrap().is_empty());

    let bob_token = common::auth_token(bob, vec!["messages:read".to_string()]).await;
    let req = test::TestRequest::get()
        .uri("/api/v1/messages/unread-count")
        .insert_header((header::AUTHORIZATION, format!("Bearer {bob_token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["count"], 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn edit_and_delete_message(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let alice = common::create_user(&pool, "alice2@example.com").await;
    let bob = common::create_user(&pool, "bob2@example.com").await;

    let token = common::auth_token(alice, vec!["messages:write".to_string()]).await;
    let req = test::TestRequest::post()
        .uri("/api/v1/messages")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(serde_json::json!({
            "recipientType": "USER",
            "recipientUserId": bob,
            "messageType": "TEXT",
            "content": "hello"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let message_id = body["id"].as_str().unwrap();

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/messages/{message_id}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(serde_json::json!({ "content": "updated" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["contentPlaintext"], "updated");

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/messages/{message_id}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[sqlx::test(migrations = "../../migrations")]
async fn get_list_mark_read_and_react(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let alice = common::create_active_user(&pool, "alice3@example.com").await;
    let bob = common::create_active_user(&pool, "bob3@example.com").await;

    let alice_token = common::auth_token(alice, vec!["messages:write".to_string()]).await;
    let req = test::TestRequest::post()
        .uri("/api/v1/messages")
        .insert_header((header::AUTHORIZATION, format!("Bearer {alice_token}")))
        .set_json(serde_json::json!({
            "recipientType": "USER",
            "recipientUserId": bob,
            "messageType": "TEXT",
            "content": "hey"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let message_id = body["id"].as_str().unwrap();
    let conversation_id = body["conversationId"].as_str().unwrap();

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/messages/{message_id}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {alice_token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["contentPlaintext"], "hey");

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/conversations/{conversation_id}/messages"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {alice_token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 1);

    let bob_token = common::auth_token(
        bob,
        vec!["messages:read".to_string(), "messages:write".to_string()],
    )
    .await;
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/messages/{message_id}/read"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {bob_token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/messages/{message_id}/reactions"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {bob_token}")))
        .set_json(serde_json::json!({ "reactionType": "LIKE" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/messages/{message_id}/reactions/LIKE"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {bob_token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[sqlx::test(migrations = "../../migrations")]
async fn pin_and_unpin_message(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let alice = common::create_active_user(&pool, "alice4@example.com").await;

    let token = common::auth_token(
        alice,
        vec!["chatrooms:write".to_string(), "messages:write".to_string()],
    )
    .await;
    let req = test::TestRequest::post()
        .uri("/api/v1/chatrooms")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(serde_json::json!({
            "name": "Pin Room",
            "roomType": "PUBLIC"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let room_id = body["id"].as_str().unwrap();

    let req = test::TestRequest::post()
        .uri("/api/v1/messages")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(serde_json::json!({
            "recipientType": "ROOM",
            "recipientRoomId": room_id,
            "messageType": "TEXT",
            "content": "pin me"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let message_id = body["id"].as_str().unwrap();

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/messages/{message_id}/pin"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["isPinned"], true);

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/messages/{message_id}/unpin"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["isPinned"], false);
}

#[sqlx::test(migrations = "../../migrations")]
async fn send_party_members_message(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let alice = common::create_active_user(&pool, "alice_party@example.com").await;
    let party_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO parties (id, party_type, display_name, email, is_active, created_at, updated_at)
         VALUES ($1, 'ORGANIZATION', 'Team', 'team@example.com', true, now(), now())",
        party_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO user_party_memberships (id, user_id, party_id, member_role, is_active, created_at)
         VALUES ($1, $2, $3, 'MEMBER', true, now())",
        Uuid::now_v7(),
        alice,
        party_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let token = common::auth_token(alice, vec!["messages:write".to_string()]).await;
    let req = test::TestRequest::post()
        .uri("/api/v1/messages")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", party_id.to_string()))
        .set_json(serde_json::json!({
            "recipientType": "PARTY_MEMBERS",
            "messageType": "TEXT",
            "content": "team update"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["contentPlaintext"], "team update");
    assert_eq!(body["senderPartyId"], party_id.to_string());
}

#[sqlx::test(migrations = "../../migrations")]
async fn admin_broadcast_sends_to_all_users(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let _alice = common::create_active_user(&pool, "alice5@example.com").await;
    let admin = common::create_active_user(&pool, "admin5@example.com").await;

    let token = common::auth_token(admin, vec!["admin:messages".to_string()]).await;
    let req = test::TestRequest::post()
        .uri("/api/v1/admin/messages/broadcast")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(serde_json::json!({
            "target": "ALL_USERS",
            "messageType": "ADMIN_BROADCAST",
            "subject": "Hello",
            "content": "Broadcast"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 2);
}
