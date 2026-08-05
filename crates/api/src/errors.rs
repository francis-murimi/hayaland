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
            | ApiError::Application(ApplicationError::InvalidMessageContent(_))
            | ApiError::Application(ApplicationError::InvalidRecipient(_))
            | ApiError::Application(ApplicationError::InvalidReactionType(_))
            | ApiError::Validation(_) => StatusCode::BAD_REQUEST,
            ApiError::Application(ApplicationError::DuplicateEmail)
            | ApiError::Application(ApplicationError::DuplicateUsername)
            | ApiError::Application(ApplicationError::DuplicatePartyEmail)
            | ApiError::Application(ApplicationError::DuplicatePartyRole)
            | ApiError::Application(ApplicationError::DuplicateReview)
            | ApiError::Application(ApplicationError::DuplicateVerification)
            | ApiError::Application(ApplicationError::DisputeAlreadyExists)
            | ApiError::Application(ApplicationError::DuplicateNotificationTemplate) => {
                StatusCode::CONFLICT
            }
            ApiError::Application(ApplicationError::NotFound)
            | ApiError::Application(ApplicationError::PartyNotFound)
            | ApiError::Application(ApplicationError::VerificationNotFound)
            | ApiError::Application(ApplicationError::RoleNotFound)
            | ApiError::Application(ApplicationError::DealNotFound)
            | ApiError::Application(ApplicationError::DisputeNotFound)
            | ApiError::Application(ApplicationError::DealParticipationNotFound)
            | ApiError::Application(ApplicationError::MessageNotFound)
            | ApiError::Application(ApplicationError::ConversationNotFound)
            | ApiError::Application(ApplicationError::ChatRoomNotFound)
            | ApiError::Application(ApplicationError::ChatRoomMembershipNotFound)
            | ApiError::Application(ApplicationError::NotificationNotFound)
            | ApiError::Application(ApplicationError::NotificationTemplateNotFound)
            | ApiError::Application(ApplicationError::ResourceNotFound)
            | ApiError::Application(ApplicationError::NeedNotFound)
            | ApiError::Application(ApplicationError::EnhancementNotFound)
            | ApiError::Application(ApplicationError::MatchNotFound) => StatusCode::NOT_FOUND,
            ApiError::Application(ApplicationError::InvalidCredentials)
            | ApiError::Application(ApplicationError::AccountInactive)
            | ApiError::Application(ApplicationError::Unauthorized) => StatusCode::UNAUTHORIZED,
            ApiError::Application(ApplicationError::Forbidden)
            | ApiError::Application(ApplicationError::CannotDeactivateAdmin)
            | ApiError::Application(ApplicationError::CannotRemoveFirstAdmin)
            | ApiError::Application(ApplicationError::AlreadyVerified)
            | ApiError::Application(ApplicationError::CannotEditMessage)
            | ApiError::Application(ApplicationError::CannotDeleteMessage)
            | ApiError::Application(ApplicationError::CannotManageChatRoom)
            | ApiError::Forbidden => StatusCode::FORBIDDEN,
            ApiError::Application(ApplicationError::PartyHasActiveDeals)
            | ApiError::Application(ApplicationError::PartyRoleHasActiveDeals)
            | ApiError::Application(ApplicationError::InvalidStateTransition { .. })
            | ApiError::Application(ApplicationError::InvalidValueDistribution { .. })
            | ApiError::Application(ApplicationError::WinWinWinValidationFailed { .. })
            | ApiError::Application(ApplicationError::ChatRoomAlreadyExists)
            | ApiError::Application(ApplicationError::AlreadyChatRoomMember)
            | ApiError::Application(ApplicationError::ReplyNotInSameContext)
            | ApiError::Application(ApplicationError::CatalogItemHasActiveDeals) => {
                StatusCode::CONFLICT
            }
            ApiError::Application(ApplicationError::DealAccessDenied)
            | ApiError::Application(ApplicationError::DisputeAccessDenied)
            | ApiError::Application(ApplicationError::CatalogAccessDenied)
            | ApiError::Application(ApplicationError::PartyNotMatchParticipant) => {
                StatusCode::FORBIDDEN
            }
            ApiError::Application(ApplicationError::EmailSendFailed)
            | ApiError::Application(ApplicationError::PushSendFailed)
            | ApiError::Application(ApplicationError::SmsSendFailed)
            | ApiError::Application(ApplicationError::Infrastructure(_)) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            ApiError::Application(ApplicationError::InvalidOrExpiredVerificationToken)
            | ApiError::Application(ApplicationError::InvalidOrExpiredPasswordResetToken)
            | ApiError::Application(ApplicationError::InvalidMatchStatus(_))
            | ApiError::Application(ApplicationError::InvalidMatchResponse(_))
            | ApiError::Application(ApplicationError::MatchExpired)
            | ApiError::Application(ApplicationError::InvalidMediaContentType { .. })
            | ApiError::Application(ApplicationError::MediaTooLarge)
            | ApiError::Application(ApplicationError::InvalidSearchTarget { .. }) => {
                StatusCode::BAD_REQUEST
            }
            ApiError::Application(ApplicationError::MediaNotFound)
            | ApiError::Application(ApplicationError::AuditLogNotFound) => StatusCode::NOT_FOUND,
            ApiError::Application(ApplicationError::MediaStorageFailed { .. })
            | ApiError::Application(ApplicationError::AnalyticsError { .. }) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            ApiError::Application(ApplicationError::InsufficientEscrowFunds) => {
                StatusCode::PAYMENT_REQUIRED
            }
            ApiError::Application(ApplicationError::SettlementFailed { .. }) => {
                StatusCode::CONFLICT
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
            ApiError::Application(ApplicationError::DuplicateReview) => "duplicate_review",
            ApiError::Application(ApplicationError::DuplicateVerification) => {
                "duplicate_verification"
            }
            ApiError::Application(ApplicationError::ChatRoomAlreadyExists) => {
                "chat_room_already_exists"
            }
            ApiError::Application(ApplicationError::AlreadyChatRoomMember) => {
                "already_chat_room_member"
            }
            ApiError::Application(ApplicationError::VerificationNotFound) => {
                "verification_not_found"
            }
            ApiError::Application(ApplicationError::WeakPassword { .. }) => "weak_password",
            ApiError::Application(ApplicationError::NotFound) => "not_found",
            ApiError::Application(ApplicationError::PartyNotFound) => "party_not_found",
            ApiError::Application(ApplicationError::RoleNotFound) => "role_not_found",
            ApiError::Application(ApplicationError::MessageNotFound) => "message_not_found",
            ApiError::Application(ApplicationError::ConversationNotFound) => {
                "conversation_not_found"
            }
            ApiError::Application(ApplicationError::ChatRoomNotFound) => "chat_room_not_found",
            ApiError::Application(ApplicationError::ChatRoomMembershipNotFound) => {
                "chat_room_membership_not_found"
            }
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
            ApiError::Application(ApplicationError::CannotEditMessage) => "cannot_edit_message",
            ApiError::Application(ApplicationError::CannotDeleteMessage) => "cannot_delete_message",
            ApiError::Application(ApplicationError::CannotManageChatRoom) => {
                "cannot_manage_chat_room"
            }
            ApiError::Application(ApplicationError::ReplyNotInSameContext) => {
                "reply_not_in_same_context"
            }
            ApiError::Application(ApplicationError::InvalidMessageContent(_)) => {
                "invalid_message_content"
            }
            ApiError::Application(ApplicationError::InvalidRecipient(_)) => "invalid_recipient",
            ApiError::Application(ApplicationError::InvalidReactionType(_)) => {
                "invalid_reaction_type"
            }
            ApiError::Application(ApplicationError::PartyHasActiveDeals) => {
                "party_has_active_deals"
            }
            ApiError::Application(ApplicationError::PartyRoleHasActiveDeals) => {
                "party_role_has_active_deals"
            }
            ApiError::Application(ApplicationError::DealNotFound) => "deal_not_found",
            ApiError::Application(ApplicationError::DealParticipationNotFound) => {
                "deal_participation_not_found"
            }
            ApiError::Application(ApplicationError::DisputeNotFound) => "dispute_not_found",
            ApiError::Application(ApplicationError::DisputeAlreadyExists) => {
                "dispute_already_exists"
            }
            ApiError::Application(ApplicationError::DisputeAccessDenied) => "dispute_access_denied",
            ApiError::Application(ApplicationError::InvalidStateTransition { .. }) => {
                "invalid_state_transition"
            }
            ApiError::Application(ApplicationError::InvalidValueDistribution { .. }) => {
                "invalid_value_distribution"
            }
            ApiError::Application(ApplicationError::WinWinWinValidationFailed { .. }) => {
                "win_win_win_validation_failed"
            }
            ApiError::Application(ApplicationError::DealAccessDenied) => "deal_access_denied",
            ApiError::Application(ApplicationError::Infrastructure(_)) => "internal_error",
            ApiError::Application(ApplicationError::NotificationNotFound) => {
                "notification_not_found"
            }
            ApiError::Application(ApplicationError::NotificationTemplateNotFound) => {
                "notification_template_not_found"
            }
            ApiError::Application(ApplicationError::ResourceNotFound) => "resource_not_found",
            ApiError::Application(ApplicationError::NeedNotFound) => "need_not_found",
            ApiError::Application(ApplicationError::EnhancementNotFound) => "enhancement_not_found",
            ApiError::Application(ApplicationError::CatalogAccessDenied) => "catalog_access_denied",
            ApiError::Application(ApplicationError::CatalogItemHasActiveDeals) => {
                "catalog_item_has_active_deals"
            }
            ApiError::Application(ApplicationError::DuplicateNotificationTemplate) => {
                "duplicate_notification_template"
            }
            ApiError::Application(ApplicationError::MatchNotFound) => "match_not_found",
            ApiError::Application(ApplicationError::InvalidMatchStatus(_)) => {
                "invalid_match_status"
            }
            ApiError::Application(ApplicationError::InvalidMatchResponse(_)) => {
                "invalid_match_response"
            }
            ApiError::Application(ApplicationError::MatchExpired) => "match_expired",
            ApiError::Application(ApplicationError::PartyNotMatchParticipant) => {
                "party_not_match_participant"
            }
            ApiError::Application(ApplicationError::MediaNotFound) => "media_not_found",
            ApiError::Application(ApplicationError::InvalidMediaContentType { .. }) => {
                "invalid_media_content_type"
            }
            ApiError::Application(ApplicationError::MediaTooLarge) => "media_too_large",
            ApiError::Application(ApplicationError::MediaStorageFailed { .. }) => {
                "media_storage_failed"
            }
            ApiError::Application(ApplicationError::AuditLogNotFound) => "audit_log_not_found",
            ApiError::Application(ApplicationError::AnalyticsError { .. }) => "analytics_error",
            ApiError::Application(ApplicationError::InvalidSearchTarget { .. }) => {
                "invalid_search_target"
            }
            ApiError::Application(ApplicationError::PushSendFailed) => "push_send_failed",
            ApiError::Application(ApplicationError::SmsSendFailed) => "sms_send_failed",
            ApiError::Application(ApplicationError::InsufficientEscrowFunds) => {
                "insufficient_escrow_funds"
            }
            ApiError::Application(ApplicationError::SettlementFailed { .. }) => "settlement_failed",
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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::ResponseError;

    #[test]
    fn settlement_failed_maps_to_conflict() {
        let err = ApiError::Application(ApplicationError::SettlementFailed {
            reason: "insufficient escrow".to_string(),
        });
        assert_eq!(err.status_code(), StatusCode::CONFLICT);

        let response = err.error_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    fn all_application_errors() -> Vec<(ApplicationError, StatusCode, &'static str)> {
        vec![
            (
                ApplicationError::Validation(vec!["x".to_string()]),
                StatusCode::BAD_REQUEST,
                "validation_error",
            ),
            (
                ApplicationError::WeakPassword {
                    message: "x".to_string(),
                },
                StatusCode::BAD_REQUEST,
                "weak_password",
            ),
            (
                ApplicationError::InvalidMessageContent("x".to_string()),
                StatusCode::BAD_REQUEST,
                "invalid_message_content",
            ),
            (
                ApplicationError::InvalidRecipient("x".to_string()),
                StatusCode::BAD_REQUEST,
                "invalid_recipient",
            ),
            (
                ApplicationError::InvalidReactionType("x".to_string()),
                StatusCode::BAD_REQUEST,
                "invalid_reaction_type",
            ),
            (
                ApplicationError::DuplicateEmail,
                StatusCode::CONFLICT,
                "duplicate_email",
            ),
            (
                ApplicationError::DuplicateUsername,
                StatusCode::CONFLICT,
                "duplicate_username",
            ),
            (
                ApplicationError::DuplicatePartyEmail,
                StatusCode::CONFLICT,
                "duplicate_party_email",
            ),
            (
                ApplicationError::DuplicatePartyRole,
                StatusCode::CONFLICT,
                "duplicate_party_role",
            ),
            (
                ApplicationError::DuplicateReview,
                StatusCode::CONFLICT,
                "duplicate_review",
            ),
            (
                ApplicationError::DuplicateVerification,
                StatusCode::CONFLICT,
                "duplicate_verification",
            ),
            (
                ApplicationError::DisputeAlreadyExists,
                StatusCode::CONFLICT,
                "dispute_already_exists",
            ),
            (
                ApplicationError::DuplicateNotificationTemplate,
                StatusCode::CONFLICT,
                "duplicate_notification_template",
            ),
            (
                ApplicationError::NotFound,
                StatusCode::NOT_FOUND,
                "not_found",
            ),
            (
                ApplicationError::PartyNotFound,
                StatusCode::NOT_FOUND,
                "party_not_found",
            ),
            (
                ApplicationError::VerificationNotFound,
                StatusCode::NOT_FOUND,
                "verification_not_found",
            ),
            (
                ApplicationError::RoleNotFound,
                StatusCode::NOT_FOUND,
                "role_not_found",
            ),
            (
                ApplicationError::DealNotFound,
                StatusCode::NOT_FOUND,
                "deal_not_found",
            ),
            (
                ApplicationError::DisputeNotFound,
                StatusCode::NOT_FOUND,
                "dispute_not_found",
            ),
            (
                ApplicationError::DealParticipationNotFound,
                StatusCode::NOT_FOUND,
                "deal_participation_not_found",
            ),
            (
                ApplicationError::MessageNotFound,
                StatusCode::NOT_FOUND,
                "message_not_found",
            ),
            (
                ApplicationError::ConversationNotFound,
                StatusCode::NOT_FOUND,
                "conversation_not_found",
            ),
            (
                ApplicationError::ChatRoomNotFound,
                StatusCode::NOT_FOUND,
                "chat_room_not_found",
            ),
            (
                ApplicationError::ChatRoomMembershipNotFound,
                StatusCode::NOT_FOUND,
                "chat_room_membership_not_found",
            ),
            (
                ApplicationError::NotificationNotFound,
                StatusCode::NOT_FOUND,
                "notification_not_found",
            ),
            (
                ApplicationError::NotificationTemplateNotFound,
                StatusCode::NOT_FOUND,
                "notification_template_not_found",
            ),
            (
                ApplicationError::ResourceNotFound,
                StatusCode::NOT_FOUND,
                "resource_not_found",
            ),
            (
                ApplicationError::NeedNotFound,
                StatusCode::NOT_FOUND,
                "need_not_found",
            ),
            (
                ApplicationError::EnhancementNotFound,
                StatusCode::NOT_FOUND,
                "enhancement_not_found",
            ),
            (
                ApplicationError::MatchNotFound,
                StatusCode::NOT_FOUND,
                "match_not_found",
            ),
            (
                ApplicationError::InvalidCredentials,
                StatusCode::UNAUTHORIZED,
                "invalid_credentials",
            ),
            (
                ApplicationError::AccountInactive,
                StatusCode::UNAUTHORIZED,
                "account_inactive",
            ),
            (
                ApplicationError::Unauthorized,
                StatusCode::UNAUTHORIZED,
                "unauthorized",
            ),
            (
                ApplicationError::Forbidden,
                StatusCode::FORBIDDEN,
                "forbidden",
            ),
            (
                ApplicationError::CannotDeactivateAdmin,
                StatusCode::FORBIDDEN,
                "cannot_deactivate_admin",
            ),
            (
                ApplicationError::CannotRemoveFirstAdmin,
                StatusCode::FORBIDDEN,
                "cannot_remove_first_admin",
            ),
            (
                ApplicationError::AlreadyVerified,
                StatusCode::FORBIDDEN,
                "already_verified",
            ),
            (
                ApplicationError::CannotEditMessage,
                StatusCode::FORBIDDEN,
                "cannot_edit_message",
            ),
            (
                ApplicationError::CannotDeleteMessage,
                StatusCode::FORBIDDEN,
                "cannot_delete_message",
            ),
            (
                ApplicationError::CannotManageChatRoom,
                StatusCode::FORBIDDEN,
                "cannot_manage_chat_room",
            ),
            (
                ApplicationError::PartyHasActiveDeals,
                StatusCode::CONFLICT,
                "party_has_active_deals",
            ),
            (
                ApplicationError::PartyRoleHasActiveDeals,
                StatusCode::CONFLICT,
                "party_role_has_active_deals",
            ),
            (
                ApplicationError::InvalidStateTransition {
                    from: "A".to_string(),
                    to: "B".to_string(),
                },
                StatusCode::CONFLICT,
                "invalid_state_transition",
            ),
            (
                ApplicationError::InvalidValueDistribution {
                    message: "x".to_string(),
                },
                StatusCode::CONFLICT,
                "invalid_value_distribution",
            ),
            (
                ApplicationError::WinWinWinValidationFailed {
                    violations: vec!["x".to_string()],
                },
                StatusCode::CONFLICT,
                "win_win_win_validation_failed",
            ),
            (
                ApplicationError::ChatRoomAlreadyExists,
                StatusCode::CONFLICT,
                "chat_room_already_exists",
            ),
            (
                ApplicationError::AlreadyChatRoomMember,
                StatusCode::CONFLICT,
                "already_chat_room_member",
            ),
            (
                ApplicationError::ReplyNotInSameContext,
                StatusCode::CONFLICT,
                "reply_not_in_same_context",
            ),
            (
                ApplicationError::CatalogItemHasActiveDeals,
                StatusCode::CONFLICT,
                "catalog_item_has_active_deals",
            ),
            (
                ApplicationError::DealAccessDenied,
                StatusCode::FORBIDDEN,
                "deal_access_denied",
            ),
            (
                ApplicationError::DisputeAccessDenied,
                StatusCode::FORBIDDEN,
                "dispute_access_denied",
            ),
            (
                ApplicationError::CatalogAccessDenied,
                StatusCode::FORBIDDEN,
                "catalog_access_denied",
            ),
            (
                ApplicationError::PartyNotMatchParticipant,
                StatusCode::FORBIDDEN,
                "party_not_match_participant",
            ),
            (
                ApplicationError::EmailSendFailed,
                StatusCode::INTERNAL_SERVER_ERROR,
                "email_send_failed",
            ),
            (
                ApplicationError::PushSendFailed,
                StatusCode::INTERNAL_SERVER_ERROR,
                "push_send_failed",
            ),
            (
                ApplicationError::SmsSendFailed,
                StatusCode::INTERNAL_SERVER_ERROR,
                "sms_send_failed",
            ),
            (
                ApplicationError::Infrastructure("x".to_string()),
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
            ),
            (
                ApplicationError::InvalidOrExpiredVerificationToken,
                StatusCode::BAD_REQUEST,
                "invalid_or_expired_token",
            ),
            (
                ApplicationError::InvalidOrExpiredPasswordResetToken,
                StatusCode::BAD_REQUEST,
                "invalid_or_expired_token",
            ),
            (
                ApplicationError::InvalidMatchStatus("x".to_string()),
                StatusCode::BAD_REQUEST,
                "invalid_match_status",
            ),
            (
                ApplicationError::InvalidMatchResponse("x".to_string()),
                StatusCode::BAD_REQUEST,
                "invalid_match_response",
            ),
            (
                ApplicationError::MatchExpired,
                StatusCode::BAD_REQUEST,
                "match_expired",
            ),
            (
                ApplicationError::InvalidMediaContentType {
                    message: "x".to_string(),
                },
                StatusCode::BAD_REQUEST,
                "invalid_media_content_type",
            ),
            (
                ApplicationError::MediaTooLarge,
                StatusCode::BAD_REQUEST,
                "media_too_large",
            ),
            (
                ApplicationError::InvalidSearchTarget {
                    message: "x".to_string(),
                },
                StatusCode::BAD_REQUEST,
                "invalid_search_target",
            ),
            (
                ApplicationError::MediaNotFound,
                StatusCode::NOT_FOUND,
                "media_not_found",
            ),
            (
                ApplicationError::AuditLogNotFound,
                StatusCode::NOT_FOUND,
                "audit_log_not_found",
            ),
            (
                ApplicationError::MediaStorageFailed {
                    message: "x".to_string(),
                },
                StatusCode::INTERNAL_SERVER_ERROR,
                "media_storage_failed",
            ),
            (
                ApplicationError::AnalyticsError {
                    message: "x".to_string(),
                },
                StatusCode::INTERNAL_SERVER_ERROR,
                "analytics_error",
            ),
            (
                ApplicationError::InsufficientEscrowFunds,
                StatusCode::PAYMENT_REQUIRED,
                "insufficient_escrow_funds",
            ),
            (
                ApplicationError::SettlementFailed {
                    reason: "x".to_string(),
                },
                StatusCode::CONFLICT,
                "settlement_failed",
            ),
        ]
    }

    #[actix_rt::test]
    async fn every_application_error_maps_to_status_and_code() {
        for (app_err, expected_status, expected_code) in all_application_errors() {
            let err = ApiError::Application(app_err);
            assert_eq!(err.status_code(), expected_status, "status for {err:?}");

            let response = err.error_response();
            assert_eq!(response.status(), expected_status);
            let body = response.into_body();
            let bytes = actix_web::body::to_bytes(body).await.unwrap();
            let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(
                json["code"].as_str().unwrap(),
                expected_code,
                "code for {err:?}"
            );
            assert!(!json["message"].as_str().unwrap().is_empty());
        }
    }

    #[test]
    fn validation_and_forbidden_variants() {
        let err = ApiError::Validation("bad field".to_string());
        assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(
            err.error_response().status(),
            StatusCode::BAD_REQUEST
        );

        let err = ApiError::Forbidden;
        assert_eq!(err.status_code(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn from_validator_errors_flattens_field_messages() {
        let mut errors = validator::ValidationErrors::new();
        let error = validator::ValidationError::new("length");
        errors.add("email", error);

        let api_err = ApiError::from(errors);
        assert_eq!(api_err.status_code(), StatusCode::BAD_REQUEST);
        assert!(matches!(api_err, ApiError::Validation(_)));
        assert!(api_err.to_string().contains("email"));
    }
}
