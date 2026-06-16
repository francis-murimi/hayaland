use actix_web::http::header;
use actix_web::{http::StatusCode, test, web::Data, App};
use api::routes;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

mod common;

#[sqlx::test(migrations = "../../migrations")]
async fn list_notifications_empty_then_with_sent(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let user = common::create_active_user(&pool, "notifylist@example.com").await;
    let token = common::auth_token(
        user,
        vec![
            "notifications:read".to_string(),
            "notifications:write".to_string(),
        ],
    )
    .await;

    let resp = test::TestRequest::get()
        .uri("/api/v1/notifications")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["data"].as_array().unwrap().is_empty());
    assert_eq!(body["unread_count"], 0);
    assert_eq!(body["total"], 0);

    let admin_token = common::auth_token(user, vec!["admin:notifications".to_string()]).await;
    let send_resp = test::TestRequest::post()
        .uri("/api/v1/admin/notifications/send")
        .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
        .set_json(json!({
            "target": { "type": "USER", "user_id": user },
            "notification_type": "ADMIN_BROADCAST",
            "priority": "NORMAL",
            "title": "Hello",
            "body": "World",
            "metadata": { "title": "Hello", "body": "World" }
        }))
        .send_request(&app)
        .await;
    assert_eq!(send_resp.status(), StatusCode::ACCEPTED);
    let send_body: serde_json::Value = test::read_body_json(send_resp).await;
    assert_eq!(send_body["sent_count"], 1);
    assert_eq!(send_body["notification_ids"].as_array().unwrap().len(), 1);

    let resp = test::TestRequest::get()
        .uri("/api/v1/notifications")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert_eq!(body["unread_count"], 1);
    assert_eq!(body["total"], 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn unread_count_reflects_unread(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let user = common::create_active_user(&pool, "notifyunread@example.com").await;
    let token = common::auth_token(user, vec!["notifications:read".to_string()]).await;

    let resp = test::TestRequest::get()
        .uri("/api/v1/notifications/unread-count")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["count"], 0);

    let admin_token = common::auth_token(user, vec!["admin:*".to_string()]).await;
    let send_resp = test::TestRequest::post()
        .uri("/api/v1/admin/notifications/send")
        .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
        .set_json(json!({
            "target": { "type": "USER", "user_id": user },
            "notification_type": "SYSTEM_MAINTENANCE",
            "priority": "LOW",
            "metadata": { "body": "maint" }
        }))
        .send_request(&app)
        .await;
    assert_eq!(send_resp.status(), StatusCode::ACCEPTED);

    let resp = test::TestRequest::get()
        .uri("/api/v1/notifications/unread-count")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["count"], 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn mark_notification_read(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let user = common::create_active_user(&pool, "notifyread@example.com").await;
    let read_token = common::auth_token(user, vec!["notifications:read".to_string()]).await;
    let write_token = common::auth_token(user, vec!["notifications:write".to_string()]).await;
    let admin_token = common::auth_token(user, vec!["admin:notifications".to_string()]).await;

    let send_resp = test::TestRequest::post()
        .uri("/api/v1/admin/notifications/send")
        .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
        .set_json(json!({
            "target": { "type": "USER", "user_id": user },
            "notification_type": "SECURITY_ALERT",
            "priority": "CRITICAL",
            "metadata": { "body": "alert" }
        }))
        .send_request(&app)
        .await;
    assert_eq!(send_resp.status(), StatusCode::ACCEPTED);
    let send_body: serde_json::Value = test::read_body_json(send_resp).await;
    let notification_id = send_body["notification_ids"][0].as_str().unwrap();

    let resp = test::TestRequest::get()
        .uri("/api/v1/notifications")
        .insert_header((header::AUTHORIZATION, format!("Bearer {read_token}")))
        .send_request(&app)
        .await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["data"][0]["is_read"], false);

    let patch_resp = test::TestRequest::patch()
        .uri(&format!("/api/v1/notifications/{notification_id}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {write_token}")))
        .set_json(json!({ "is_read": true }))
        .send_request(&app)
        .await;
    assert_eq!(patch_resp.status(), StatusCode::NO_CONTENT);

    let resp = test::TestRequest::get()
        .uri("/api/v1/notifications")
        .insert_header((header::AUTHORIZATION, format!("Bearer {read_token}")))
        .send_request(&app)
        .await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["data"][0]["is_read"], true);
    assert_eq!(body["unread_count"], 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn delete_notification(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let user = common::create_active_user(&pool, "notifydelete@example.com").await;
    let read_token = common::auth_token(user, vec!["notifications:read".to_string()]).await;
    let write_token = common::auth_token(user, vec!["notifications:write".to_string()]).await;
    let admin_token = common::auth_token(user, vec!["admin:notifications".to_string()]).await;

    let send_resp = test::TestRequest::post()
        .uri("/api/v1/admin/notifications/send")
        .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
        .set_json(json!({
            "target": { "type": "USER", "user_id": user },
            "notification_type": "CUSTOM",
            "priority": "NORMAL",
            "metadata": { "body": "to delete" }
        }))
        .send_request(&app)
        .await;
    let send_body: serde_json::Value = test::read_body_json(send_resp).await;
    let notification_id = send_body["notification_ids"][0].as_str().unwrap();

    let delete_resp = test::TestRequest::delete()
        .uri(&format!("/api/v1/notifications/{notification_id}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {write_token}")))
        .send_request(&app)
        .await;
    assert_eq!(delete_resp.status(), StatusCode::NO_CONTENT);

    let resp = test::TestRequest::get()
        .uri("/api/v1/notifications")
        .insert_header((header::AUTHORIZATION, format!("Bearer {read_token}")))
        .send_request(&app)
        .await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["data"].as_array().unwrap().is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn get_default_preferences(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let user = common::create_active_user(&pool, "notifyprefsget@example.com").await;
    let token = common::auth_token(user, vec!["notifications:read".to_string()]).await;

    let resp = test::TestRequest::get()
        .uri("/api/v1/notifications/preferences")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["user_id"], user.to_string());
    assert_eq!(body["channels"]["in_app"], true);
    assert_eq!(body["channels"]["email"], true);
    assert_eq!(body["channels"]["push"], false);
    assert_eq!(body["channels"]["sms"], false);
    assert_eq!(body["quiet_hours"]["enabled"], false);
}

#[sqlx::test(migrations = "../../migrations")]
async fn update_preferences_and_reject_mismatched_user(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let user_a = common::create_active_user(&pool, "notifyprefsa@example.com").await;
    let user_b = common::create_active_user(&pool, "notifyprefsb@example.com").await;
    let token_a = common::auth_token(user_a, vec!["notifications:write".to_string()]).await;

    let update = json!({
        "user_id": user_a,
        "channels": { "in_app": true, "email": false, "push": true, "sms": false },
        "per_type": {},
        "quiet_hours": { "enabled": false, "start": "22:00", "end": "07:00", "timezone": "UTC", "except_critical": true }
    });

    let resp = test::TestRequest::put()
        .uri("/api/v1/notifications/preferences")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token_a}")))
        .set_json(update)
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["channels"]["email"], false);
    assert_eq!(body["channels"]["push"], true);

    let mismatched = json!({
        "user_id": user_b,
        "channels": { "in_app": true, "email": true, "push": false, "sms": false },
        "per_type": {},
        "quiet_hours": { "enabled": false, "start": "22:00", "end": "07:00", "timezone": "UTC", "except_critical": true }
    });
    let resp = test::TestRequest::put()
        .uri("/api/v1/notifications/preferences")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token_a}")))
        .set_json(mismatched)
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../migrations")]
async fn admin_send_requires_notification_admin_scope(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let user = common::create_active_user(&pool, "notifyadminsend@example.com").await;
    let token = common::auth_token(user, vec!["notifications:read".to_string()]).await;

    let resp = test::TestRequest::post()
        .uri("/api/v1/admin/notifications/send")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(json!({
            "target": { "type": "USER", "user_id": user },
            "notification_type": "ADMIN_BROADCAST",
            "priority": "NORMAL",
            "metadata": { "title": "T", "body": "B" }
        }))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let admin_token = common::auth_token(user, vec!["admin:notifications".to_string()]).await;
    let resp = test::TestRequest::post()
        .uri("/api/v1/admin/notifications/send")
        .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
        .set_json(json!({
            "target": { "type": "USER", "user_id": user },
            "notification_type": "ADMIN_BROADCAST",
            "priority": "NORMAL",
            "metadata": { "title": "T", "body": "B" }
        }))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
}

#[sqlx::test(migrations = "../../migrations")]
async fn admin_notification_template_crud(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let admin = common::create_active_user(&pool, "notifytemplateadmin@example.com").await;
    let admin_token = common::auth_token(admin, vec!["admin:*".to_string()]).await;

    let list_resp = test::TestRequest::get()
        .uri("/api/v1/admin/notification-templates")
        .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
        .send_request(&app)
        .await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    let list_body: serde_json::Value = test::read_body_json(list_resp).await;
    let initial_len = list_body.as_array().unwrap().len();

    let create_resp = test::TestRequest::post()
        .uri("/api/v1/admin/notification-templates")
        .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
        .set_json(json!({
            "name": "Test Template",
            "notification_type": "CUSTOM",
            "channel": "EMAIL",
            "locale": "en",
            "subject_template": "Subject {{title}}",
            "body_template": "Body {{body}}",
            "variables_schema": { "title": "string", "body": "string" }
        }))
        .send_request(&app)
        .await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let created: serde_json::Value = test::read_body_json(create_resp).await;
    let template_id = created["id"].as_str().unwrap();
    assert_eq!(created["name"], "Test Template");

    let list_resp = test::TestRequest::get()
        .uri("/api/v1/admin/notification-templates")
        .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
        .send_request(&app)
        .await;
    let list_body: serde_json::Value = test::read_body_json(list_resp).await;
    assert_eq!(list_body.as_array().unwrap().len(), initial_len + 1);

    let get_resp = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/admin/notification-templates/{template_id}"
        ))
        .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
        .send_request(&app)
        .await;
    assert_eq!(get_resp.status(), StatusCode::OK);
    let got: serde_json::Value = test::read_body_json(get_resp).await;
    assert_eq!(got["name"], "Test Template");

    let update_resp = test::TestRequest::put()
        .uri(&format!(
            "/api/v1/admin/notification-templates/{template_id}"
        ))
        .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
        .set_json(json!({
            "name": "Updated Template",
            "notification_type": "CUSTOM",
            "channel": "EMAIL",
            "locale": "en",
            "subject_template": "Updated {{title}}",
            "body_template": "Updated {{body}}",
            "variables_schema": { "title": "string", "body": "string" }
        }))
        .send_request(&app)
        .await;
    assert_eq!(update_resp.status(), StatusCode::OK);
    let updated: serde_json::Value = test::read_body_json(update_resp).await;
    assert_eq!(updated["name"], "Updated Template");

    let delete_resp = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1/admin/notification-templates/{template_id}"
        ))
        .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
        .send_request(&app)
        .await;
    assert_eq!(delete_resp.status(), StatusCode::NO_CONTENT);

    let get_resp = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/admin/notification-templates/{template_id}"
        ))
        .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
        .send_request(&app)
        .await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../../migrations")]
async fn admin_template_endpoints_require_admin_scope(pool: PgPool) {
    let state = common::build_state(pool.clone()).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let user = common::create_active_user(&pool, "notifytemplateuser@example.com").await;
    let token = common::auth_token(user, vec!["notifications:read".to_string()]).await;
    let template_id = Uuid::now_v7();

    let resp = test::TestRequest::get()
        .uri("/api/v1/admin/notification-templates")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let resp = test::TestRequest::post()
        .uri("/api/v1/admin/notification-templates")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(json!({
            "name": "T",
            "notification_type": "ADMIN_BROADCAST",
            "channel": "EMAIL",
            "locale": "en",
            "subject_template": "S",
            "body_template": "B"
        }))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let resp = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/admin/notification-templates/{template_id}"
        ))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let resp = test::TestRequest::put()
        .uri(&format!(
            "/api/v1/admin/notification-templates/{template_id}"
        ))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(json!({
            "name": "T",
            "notification_type": "ADMIN_BROADCAST",
            "channel": "EMAIL",
            "locale": "en",
            "subject_template": "S",
            "body_template": "B"
        }))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let resp = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1/admin/notification-templates/{template_id}"
        ))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
