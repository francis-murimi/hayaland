use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use application::errors::ApplicationError;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error(transparent)]
    Application(#[from] ApplicationError),

    #[error("validation failed: {0}")]
    Validation(String),

    #[error("forbidden")]
    Forbidden,
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::Application(ApplicationError::Validation(_))
            | ApiError::Application(ApplicationError::WeakPassword { .. })
            | ApiError::Validation(_) => StatusCode::BAD_REQUEST,
            ApiError::Application(ApplicationError::DuplicateEmail)
            | ApiError::Application(ApplicationError::DuplicateUsername)
            | ApiError::Application(ApplicationError::DuplicatePartyEmail)
            | ApiError::Application(ApplicationError::DuplicatePartyRole) => StatusCode::CONFLICT,
            ApiError::Application(ApplicationError::NotFound)
            | ApiError::Application(ApplicationError::PartyNotFound)
            | ApiError::Application(ApplicationError::RoleNotFound) => StatusCode::NOT_FOUND,
            ApiError::Application(ApplicationError::InvalidCredentials)
            | ApiError::Application(ApplicationError::AccountInactive)
            | ApiError::Application(ApplicationError::Unauthorized) => StatusCode::UNAUTHORIZED,
            ApiError::Application(ApplicationError::Forbidden)
            | ApiError::Application(ApplicationError::CannotDeactivateAdmin)
            | ApiError::Application(ApplicationError::CannotRemoveFirstAdmin)
            | ApiError::Application(ApplicationError::AlreadyVerified)
            | ApiError::Forbidden => StatusCode::FORBIDDEN,
            ApiError::Application(ApplicationError::PartyHasActiveDeals)
            | ApiError::Application(ApplicationError::PartyRoleHasActiveDeals) => {
                StatusCode::CONFLICT
            }
            ApiError::Application(ApplicationError::EmailSendFailed)
            | ApiError::Application(ApplicationError::Infrastructure(_)) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            ApiError::Application(ApplicationError::InvalidOrExpiredVerificationToken)
            | ApiError::Application(ApplicationError::InvalidOrExpiredPasswordResetToken) => {
                StatusCode::BAD_REQUEST
            }
        }
    }

    fn error_response(&self) -> HttpResponse {
        let code = match self {
            ApiError::Application(ApplicationError::Validation(_)) => "validation_error",
            ApiError::Application(ApplicationError::DuplicateEmail) => "duplicate_email",
            ApiError::Application(ApplicationError::DuplicateUsername) => "duplicate_username",
            ApiError::Application(ApplicationError::DuplicatePartyEmail) => "duplicate_party_email",
            ApiError::Application(ApplicationError::DuplicatePartyRole) => "duplicate_party_role",
            ApiError::Application(ApplicationError::WeakPassword { .. }) => "weak_password",
            ApiError::Application(ApplicationError::NotFound) => "not_found",
            ApiError::Application(ApplicationError::PartyNotFound) => "party_not_found",
            ApiError::Application(ApplicationError::RoleNotFound) => "role_not_found",
            ApiError::Application(ApplicationError::InvalidCredentials) => "invalid_credentials",
            ApiError::Application(ApplicationError::AccountInactive) => "account_inactive",
            ApiError::Application(ApplicationError::Unauthorized) => "unauthorized",
            ApiError::Application(ApplicationError::Forbidden) => "forbidden",
            ApiError::Application(ApplicationError::CannotDeactivateAdmin) => {
                "cannot_deactivate_admin"
            }
            ApiError::Application(ApplicationError::CannotRemoveFirstAdmin) => {
                "cannot_remove_first_admin"
            }
            ApiError::Application(ApplicationError::EmailSendFailed) => "email_send_failed",
            ApiError::Application(ApplicationError::InvalidOrExpiredVerificationToken)
            | ApiError::Application(ApplicationError::InvalidOrExpiredPasswordResetToken) => {
                "invalid_or_expired_token"
            }
            ApiError::Application(ApplicationError::AlreadyVerified) => "already_verified",
            ApiError::Application(ApplicationError::PartyHasActiveDeals) => {
                "party_has_active_deals"
            }
            ApiError::Application(ApplicationError::PartyRoleHasActiveDeals) => {
                "party_role_has_active_deals"
            }
            ApiError::Application(ApplicationError::Infrastructure(_)) => "internal_error",
            ApiError::Validation(_) => "validation_error",
            ApiError::Forbidden => "forbidden",
        };

        HttpResponse::build(self.status_code()).json(ErrorBody {
            code: code.to_string(),
            message: self.to_string(),
        })
    }
}

impl From<validator::ValidationErrors> for ApiError {
    fn from(errors: validator::ValidationErrors) -> Self {
        let messages: Vec<String> = errors
            .field_errors()
            .iter()
            .flat_map(|(field, errs)| {
                errs.iter().map(move |e| {
                    let msg = e.message.as_deref().unwrap_or("invalid value");
                    format!("{field}: {msg}")
                })
            })
            .collect();
        ApiError::Validation(messages.join("; "))
    }
}
