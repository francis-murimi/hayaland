use crate::errors::ApiError;
use crate::AppState;
use actix_multipart::Multipart;
use actix_web::{web, HttpResponse};
use application::media::dto::{UploadMediaCommand, UploadMediaResult};
use application::users::token::AuthContext;
use futures_util::TryStreamExt;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Default, Deserialize)]
struct UploadFormFields {
    party_id: Option<Uuid>,
    purpose: Option<String>,
    related_entity_type: Option<String>,
    related_entity_id: Option<Uuid>,
    is_public: Option<bool>,
}

pub async fn upload_media(
    state: web::Data<AppState>,
    mut payload: Multipart,
    ctx: web::ReqData<AuthContext>,
) -> Result<HttpResponse, ApiError> {
    let mut fields = UploadFormFields::default();
    let mut results: Vec<UploadMediaResult> = Vec::new();
    let max_size = state.media_settings.max_size_bytes.max(1);

    while let Some(mut field) = payload
        .try_next()
        .await
        .map_err(|e| ApiError::Validation(format!("failed to read multipart field: {e}")))?
    {
        let content_disposition = field.content_disposition();
        let name = content_disposition
            .and_then(|cd| cd.get_name())
            .map(|s| s.to_string())
            .unwrap_or_default();

        if name != "file" {
            let mut value = String::new();
            while let Some(chunk) = field
                .try_next()
                .await
                .map_err(|e| ApiError::Validation(format!("failed to read form field: {e}")))?
            {
                value.push_str(std::str::from_utf8(&chunk).unwrap_or(""));
            }
            match name.as_str() {
                "party_id" => fields.party_id = Uuid::parse_str(&value).ok(),
                "purpose" => fields.purpose = Some(value),
                "related_entity_type" => fields.related_entity_type = Some(value),
                "related_entity_id" => fields.related_entity_id = Uuid::parse_str(&value).ok(),
                "is_public" => fields.is_public = value.parse().ok(),
                _ => {}
            }
            continue;
        }

        let filename = content_disposition
            .and_then(|cd| cd.get_filename())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "upload".to_string());
        let content_type = field
            .content_type()
            .map(|ct| ct.to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string());

        let mut bytes = Vec::new();
        while let Some(chunk) = field
            .try_next()
            .await
            .map_err(|e| ApiError::Validation(format!("failed to read file chunk: {e}")))?
        {
            if bytes.len() + chunk.len() > max_size {
                return Err(ApiError::Application(
                    application::errors::ApplicationError::MediaTooLarge,
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        if bytes.is_empty() {
            continue;
        }

        let cmd = UploadMediaCommand {
            actor_user_id: ctx.user_id,
            actor_party_id: fields.party_id,
            purpose: fields.purpose.clone().unwrap_or_default(),
            related_entity_type: fields.related_entity_type.clone(),
            related_entity_id: fields.related_entity_id,
            content_type,
            original_filename: filename,
            size_bytes: bytes.len() as i64,
            is_public: fields.is_public,
        };

        let result = state.upload_media.execute(cmd, bytes).await?;
        results.push(result);
    }

    if results.is_empty() {
        return Err(ApiError::Validation("no file provided".into()));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({ "items": results })))
}
