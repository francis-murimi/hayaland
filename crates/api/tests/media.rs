use actix_web::http::header;
use actix_web::http::StatusCode;
use actix_web::{test, web::Data, App};
use api::routes;
use sqlx::PgPool;

mod common;

fn build_multipart_request(
    token: &str,
    filename: &str,
    content: &[u8],
    purpose: &str,
) -> actix_web::test::TestRequest {
    let boundary = "BoundaryMediaTest";
    let mut body = Vec::new();
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"purpose\"\r\n\r\n{purpose}\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: image/png\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(content);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    test::TestRequest::post()
        .uri("/api/v1/media")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header((
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        ))
        .set_payload(body)
}

#[sqlx::test(migrations = "../../migrations")]
async fn media_upload_list_and_delete_lifecycle(pool: PgPool) {
    let user_id = common::create_active_user(&pool, "mediauser@example.com").await;
    let token = common::auth_token(user_id, vec!["users:read".to_string()]).await;
    let state = common::build_state(pool).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let file_content = b"fake image bytes";
    let req = build_multipart_request(&token, "test.png", file_content, "MESSAGE_ATTACHMENT");
    let resp = test::call_service(&app, req.to_request()).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    let media_id = items[0]["id"].as_str().unwrap();
    let storage_path = items[0]["storage_path"].as_str().unwrap();

    let list_req = test::TestRequest::get()
        .uri("/api/v1/media")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, list_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 1);

    let download_req = test::TestRequest::get()
        .uri(&format!("/uploads/{storage_path}"))
        .to_request();
    let resp = test::call_service(&app, download_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = test::read_body(resp).await;
    assert_eq!(bytes.to_vec(), file_content.to_vec());

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/media/{media_id}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let list_req = test::TestRequest::get()
        .uri("/api/v1/media")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, list_req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn unauthenticated_media_upload_returns_401(pool: PgPool) {
    let state = common::build_state(pool).await;
    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = build_multipart_request("", "test.png", b"x", "MESSAGE_ATTACHMENT");
    let resp = test::call_service(&app, req.to_request()).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
