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
            | ApiError::Application(ApplicationError::DuplicateVerification) => {
                StatusCode::CONFLICT
            }
            ApiError::Application(ApplicationError::NotFound)
            | ApiError::Application(ApplicationError::PartyNotFound)
            | ApiError::Application(ApplicationError::VerificationNotFound)
            | ApiError::Application(ApplicationError::RoleNotFound)
            | ApiError::Application(ApplicationError::DealNotFound)
            | ApiError::Application(ApplicationError::DealParticipationNotFound)
            | ApiError::Application(ApplicationError::MessageNotFound)
            | ApiError::Application(ApplicationError::ConversationNotFound)
            | ApiError::Application(ApplicationError::ChatRoomNotFound)
            | ApiError::Application(ApplicationError::ChatRoomMembershipNotFound) => {
                StatusCode::NOT_FOUND
            }
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
            | ApiError::Application(ApplicationError::ReplyNotInSameContext) => {
                StatusCode::CONFLICT
            }
            ApiError::Application(ApplicationError::DealAccessDenied) => StatusCode::FORBIDDEN,
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
