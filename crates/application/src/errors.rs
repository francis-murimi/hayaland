use domain::errors::DomainError;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ApplicationError {
    #[error("validation failed: {0:?}")]
    Validation(Vec<String>),

    #[error("user not found")]
    NotFound,

    #[error("party not found")]
    PartyNotFound,

    #[error("role not found")]
    RoleNotFound,

    #[error("a user with this email already exists")]
    DuplicateEmail,

    #[error("a user with this username already exists")]
    DuplicateUsername,

    #[error("a party with this email already exists")]
    DuplicatePartyEmail,

    #[error("this role is already assigned to the party")]
    DuplicatePartyRole,

    #[error("a review already exists for this deal and party")]
    DuplicateReview,

    #[error("a verification already exists for this party and type")]
    DuplicateVerification,

    #[error("verification not found")]
    VerificationNotFound,

    #[error("party role has active deals and cannot be removed")]
    PartyRoleHasActiveDeals,

    #[error("party has active deals and cannot be deleted")]
    PartyHasActiveDeals,

    #[error("deal not found")]
    DealNotFound,

    #[error("deal participation not found")]
    DealParticipationNotFound,

    #[error("invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("invalid value distribution: {message}")]
    InvalidValueDistribution { message: String },

    #[error("win-win-win validation failed")]
    WinWinWinValidationFailed { violations: Vec<String> },

    #[error("deal access denied")]
    DealAccessDenied,

    #[error("resource not found")]
    ResourceNotFound,

    #[error("need not found")]
    NeedNotFound,

    #[error("enhancement not found")]
    EnhancementNotFound,

    #[error("catalog access denied")]
    CatalogAccessDenied,

    #[error("catalog item has active deals and cannot be deleted")]
    CatalogItemHasActiveDeals,

    #[error("dispute not found")]
    DisputeNotFound,

    #[error("a dispute already exists for this deal and party")]
    DisputeAlreadyExists,

    #[error("dispute access denied")]
    DisputeAccessDenied,

    #[error("weak password: {message}")]
    WeakPassword { message: String },

    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("account is inactive")]
    AccountInactive,

    #[error("unauthorized")]
    Unauthorized,

    #[error("forbidden")]
    Forbidden,

    #[error("admin users cannot be deactivated")]
    CannotDeactivateAdmin,

    #[error("cannot remove admin role from the first admin")]
    CannotRemoveFirstAdmin,

    #[error("failed to send email")]
    EmailSendFailed,

    #[error("invalid or expired verification token")]
    InvalidOrExpiredVerificationToken,

    #[error("invalid or expired password reset token")]
    InvalidOrExpiredPasswordResetToken,

    #[error("account is already verified")]
    AlreadyVerified,

    #[error("message not found")]
    MessageNotFound,

    #[error("conversation not found")]
    ConversationNotFound,

    #[error("chat room not found")]
    ChatRoomNotFound,

    #[error("chat room already exists")]
    ChatRoomAlreadyExists,

    #[error("chat room membership not found")]
    ChatRoomMembershipNotFound,

    #[error("already a member of this chat room")]
    AlreadyChatRoomMember,

    #[error("invalid message content: {0}")]
    InvalidMessageContent(String),

    #[error("invalid recipient: {0}")]
    InvalidRecipient(String),

    #[error("invalid reaction type: {0}")]
    InvalidReactionType(String),

    #[error("cannot edit this message")]
    CannotEditMessage,

    #[error("cannot delete this message")]
    CannotDeleteMessage,

    #[error("cannot manage this chat room")]
    CannotManageChatRoom,

    #[error("reply is not in the same conversation")]
    ReplyNotInSameContext,

    #[error("notification not found")]
    NotificationNotFound,

    #[error("notification template not found")]
    NotificationTemplateNotFound,

    #[error("a notification template with this name already exists")]
    DuplicateNotificationTemplate,

    #[error("failed to send push notification")]
    PushSendFailed,

    #[error("failed to send sms")]
    SmsSendFailed,

    #[error("infrastructure error: {0}")]
    Infrastructure(String),
}

impl From<DomainError> for ApplicationError {
    fn from(err: DomainError) -> Self {
        match err {
            DomainError::InvalidEmail { message }
            | DomainError::InvalidUsername { message }
            | DomainError::InvalidPasswordHash { message }
            | DomainError::InvalidDisplayName { message }
            | DomainError::InvalidPhone { message }
            | DomainError::InvalidLocation { message }
            | DomainError::InvalidPartyType { message }
            | DomainError::InvalidVerificationStatus { message }
            | DomainError::InvalidDealRole { message }
            | DomainError::InvalidPartyMembershipRole { message }
            | DomainError::InvalidSearchParameters { message }
            | DomainError::InvalidReviewRating { message }
            | DomainError::InvalidReviewText { message }
            | DomainError::InvalidResourceCondition { message }
            | DomainError::InvalidNeedPriority { message }
            | DomainError::InvalidCatalogSearchParameters { message }
            | DomainError::InvalidChatRoomName { message }
            | DomainError::InvalidChatRoomType { message }
            | DomainError::InvalidChatRoomMemberRole { message }
            | DomainError::InvalidMessageType { message }
            | DomainError::InvalidConversationType { message } => {
                ApplicationError::Validation(vec![message])
            }
            DomainError::ReviewNotFound
            | DomainError::TermNotFound
            | DomainError::MilestoneNotFound
            | DomainError::AgreementNotFound
            | DomainError::TransactionNotFound
            | DomainError::WalletNotFound
            | DomainError::MatchNotFound => ApplicationError::NotFound,
            DomainError::ResourceNotFound => ApplicationError::ResourceNotFound,
            DomainError::NeedNotFound => ApplicationError::NeedNotFound,
            DomainError::EnhancementNotFound => ApplicationError::EnhancementNotFound,
            DomainError::ReviewPeriodExpired => {
                ApplicationError::Validation(vec!["review period has expired".to_string()])
            }
            DomainError::DuplicateEmail => ApplicationError::DuplicateEmail,
            DomainError::DuplicateUsername => ApplicationError::DuplicateUsername,
            DomainError::DuplicatePartyEmail => ApplicationError::DuplicatePartyEmail,
            DomainError::DuplicatePartyRole => ApplicationError::DuplicatePartyRole,
            DomainError::DuplicateReview => ApplicationError::DuplicateReview,
            DomainError::DuplicateVerification => ApplicationError::DuplicateVerification,
            DomainError::VerificationNotFound => ApplicationError::VerificationNotFound,
            DomainError::PartyNotFound => ApplicationError::PartyNotFound,
            DomainError::RoleNotFound => ApplicationError::RoleNotFound,
            DomainError::PartyRoleHasActiveDeals => ApplicationError::PartyRoleHasActiveDeals,
            DomainError::PartyHasActiveDeals => ApplicationError::PartyHasActiveDeals,
            DomainError::CatalogItemHasActiveDeals => ApplicationError::CatalogItemHasActiveDeals,
            DomainError::CatalogAccessDenied => ApplicationError::CatalogAccessDenied,
            DomainError::InvalidDealStatus { message }
            | DomainError::InvalidParticipationStatus { message }
            | DomainError::InvalidDealTitle { message }
            | DomainError::InvalidValueDistribution { message } => {
                ApplicationError::Validation(vec![message])
            }
            DomainError::InvalidStateTransition { from, to } => {
                ApplicationError::InvalidStateTransition { from, to }
            }
            DomainError::InvalidVerificationType { message } => {
                ApplicationError::Validation(vec![message])
            }
            DomainError::MissingRejectionReason => {
                ApplicationError::Validation(vec!["rejection reason is required".to_string()])
            }
            DomainError::InvalidVerificationStateTransition { from, to } => {
                ApplicationError::Validation(vec![format!(
                    "invalid verification state transition from {from} to {to}"
                )])
            }
            DomainError::MissingVerificationEvidence => {
                ApplicationError::Validation(vec!["verification evidence is required".to_string()])
            }
            DomainError::DealNotFound => ApplicationError::DealNotFound,
            DomainError::DealParticipationNotFound => ApplicationError::DealParticipationNotFound,

            DomainError::InsufficientPermissions => ApplicationError::DealAccessDenied,
            DomainError::DisputeNotFound => ApplicationError::DisputeNotFound,
            DomainError::DisputeAlreadyExists => ApplicationError::DisputeAlreadyExists,
            DomainError::DisputeAccessDenied => ApplicationError::DisputeAccessDenied,
            DomainError::InvalidDisputeType { message }
            | DomainError::InvalidDisputeStatus { message }
            | DomainError::InvalidDisputeResolution { message } => {
                ApplicationError::Validation(vec![message])
            }
            DomainError::WinWinWinValidationFailed { violations } => {
                ApplicationError::WinWinWinValidationFailed { violations }
            }
            DomainError::Validation(messages) => ApplicationError::Validation(messages),
            DomainError::RepositoryError(msg) => ApplicationError::Infrastructure(msg),

            DomainError::InvalidMessageContent { message } => {
                ApplicationError::InvalidMessageContent(message)
            }
            DomainError::InvalidRecipient { message } => {
                ApplicationError::InvalidRecipient(message)
            }
            DomainError::InvalidReactionType { message } => {
                ApplicationError::InvalidReactionType(message)
            }
            DomainError::MessageNotFound => ApplicationError::MessageNotFound,
            DomainError::ConversationNotFound => ApplicationError::ConversationNotFound,
            DomainError::ChatRoomNotFound => ApplicationError::ChatRoomNotFound,
            DomainError::ChatRoomAlreadyExists => ApplicationError::ChatRoomAlreadyExists,
            DomainError::ChatRoomMembershipNotFound => ApplicationError::ChatRoomMembershipNotFound,
            DomainError::AlreadyChatRoomMember => ApplicationError::AlreadyChatRoomMember,
            DomainError::CannotEditMessage => ApplicationError::CannotEditMessage,
            DomainError::CannotDeleteMessage => ApplicationError::CannotDeleteMessage,
            DomainError::CannotManageChatRoom => ApplicationError::CannotManageChatRoom,
            DomainError::ReplyNotInSameContext => ApplicationError::ReplyNotInSameContext,
            DomainError::InvalidNotificationType { message }
            | DomainError::InvalidNotificationChannel { message }
            | DomainError::InvalidNotificationStatus { message }
            | DomainError::InvalidNotificationPriority { message } => {
                ApplicationError::Validation(vec![message])
            }
            DomainError::NotificationNotFound => ApplicationError::NotificationNotFound,
            DomainError::NotificationTemplateNotFound => {
                ApplicationError::NotificationTemplateNotFound
            }
            DomainError::DuplicateNotificationTemplate => {
                ApplicationError::DuplicateNotificationTemplate
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::errors::DomainError;

    #[test]
    fn validation_domain_errors_map_to_validation_application_error() {
        let cases = vec![
            DomainError::InvalidEmail {
                message: "bad".to_string(),
            },
            DomainError::InvalidUsername {
                message: "bad".to_string(),
            },
            DomainError::InvalidPasswordHash {
                message: "bad".to_string(),
            },
            DomainError::InvalidDisplayName {
                message: "bad".to_string(),
            },
            DomainError::InvalidPhone {
                message: "bad".to_string(),
            },
            DomainError::InvalidLocation {
                message: "bad".to_string(),
            },
            DomainError::InvalidPartyType {
                message: "bad".to_string(),
            },
            DomainError::InvalidVerificationStatus {
                message: "bad".to_string(),
            },
            DomainError::InvalidDealRole {
                message: "bad".to_string(),
            },
            DomainError::InvalidPartyMembershipRole {
                message: "bad".to_string(),
            },
            DomainError::InvalidSearchParameters {
                message: "bad".to_string(),
            },
            DomainError::InvalidDealStatus {
                message: "bad".to_string(),
            },
            DomainError::InvalidParticipationStatus {
                message: "bad".to_string(),
            },
            DomainError::InvalidDealTitle {
                message: "bad".to_string(),
            },
            DomainError::InvalidValueDistribution {
                message: "bad".to_string(),
            },
            DomainError::InvalidReviewRating {
                message: "bad".to_string(),
            },
            DomainError::InvalidReviewText {
                message: "bad".to_string(),
            },
            DomainError::ReviewPeriodExpired,
            DomainError::InvalidChatRoomName {
                message: "bad".to_string(),
            },
            DomainError::InvalidChatRoomType {
                message: "bad".to_string(),
            },
            DomainError::InvalidChatRoomMemberRole {
                message: "bad".to_string(),
            },
            DomainError::InvalidMessageType {
                message: "bad".to_string(),
            },
            DomainError::InvalidConversationType {
                message: "bad".to_string(),
            },
        ];

        for case in cases {
            let app_err: ApplicationError = case.into();
            assert!(
                matches!(app_err, ApplicationError::Validation(_)),
                "expected Validation variant for {app_err:?}"
            );
        }
    }

    #[test]
    fn duplicate_domain_errors_map_to_duplicate_application_errors() {
        assert!(matches!(
            ApplicationError::from(DomainError::DuplicateEmail),
            ApplicationError::DuplicateEmail
        ));
        assert!(matches!(
            ApplicationError::from(DomainError::DuplicateUsername),
            ApplicationError::DuplicateUsername
        ));
        assert!(matches!(
            ApplicationError::from(DomainError::DuplicatePartyEmail),
            ApplicationError::DuplicatePartyEmail
        ));
        assert!(matches!(
            ApplicationError::from(DomainError::DuplicatePartyRole),
            ApplicationError::DuplicatePartyRole
        ));
        assert!(matches!(
            ApplicationError::from(DomainError::DuplicateReview),
            ApplicationError::DuplicateReview
        ));
    }

    #[test]
    fn not_found_domain_errors_map_to_not_found_application_error() {
        let cases = vec![
            DomainError::TermNotFound,
            DomainError::MilestoneNotFound,
            DomainError::AgreementNotFound,
            DomainError::TransactionNotFound,
            DomainError::WalletNotFound,
            DomainError::MatchNotFound,
            DomainError::ReviewNotFound,
        ];

        for case in cases {
            assert!(matches!(
                ApplicationError::from(case),
                ApplicationError::NotFound
            ));
        }
    }

    #[test]
    fn message_domain_errors_map_to_message_application_errors() {
        assert!(matches!(
            ApplicationError::from(DomainError::MessageNotFound),
            ApplicationError::MessageNotFound
        ));
        assert!(matches!(
            ApplicationError::from(DomainError::ConversationNotFound),
            ApplicationError::ConversationNotFound
        ));
        assert!(matches!(
            ApplicationError::from(DomainError::ChatRoomNotFound),
            ApplicationError::ChatRoomNotFound
        ));
        assert!(matches!(
            ApplicationError::from(DomainError::ChatRoomMembershipNotFound),
            ApplicationError::ChatRoomMembershipNotFound
        ));
    }

    #[test]
    fn chat_room_duplicate_domain_errors_map_to_application_errors() {
        assert!(matches!(
            ApplicationError::from(DomainError::ChatRoomAlreadyExists),
            ApplicationError::ChatRoomAlreadyExists
        ));
        assert!(matches!(
            ApplicationError::from(DomainError::AlreadyChatRoomMember),
            ApplicationError::AlreadyChatRoomMember
        ));
    }

    #[test]
    fn action_domain_errors_map_to_application_errors() {
        assert!(matches!(
            ApplicationError::from(DomainError::CannotEditMessage),
            ApplicationError::CannotEditMessage
        ));
        assert!(matches!(
            ApplicationError::from(DomainError::CannotDeleteMessage),
            ApplicationError::CannotDeleteMessage
        ));
        assert!(matches!(
            ApplicationError::from(DomainError::CannotManageChatRoom),
            ApplicationError::CannotManageChatRoom
        ));
        assert!(matches!(
            ApplicationError::from(DomainError::ReplyNotInSameContext),
            ApplicationError::ReplyNotInSameContext
        ));
    }

    #[test]
    fn content_domain_errors_map_to_application_validation_errors() {
        let content: ApplicationError = DomainError::InvalidMessageContent {
            message: "empty".to_string(),
        }
        .into();
        assert!(matches!(
            content,
            ApplicationError::InvalidMessageContent(_)
        ));

        let recipient: ApplicationError = DomainError::InvalidRecipient {
            message: "missing".to_string(),
        }
        .into();
        assert!(matches!(recipient, ApplicationError::InvalidRecipient(_)));

        let reaction: ApplicationError = DomainError::InvalidReactionType {
            message: "unknown".to_string(),
        }
        .into();
        assert!(matches!(reaction, ApplicationError::InvalidReactionType(_)));
    }

    #[test]
    fn deal_specific_domain_errors_map_correctly() {
        assert!(matches!(
            ApplicationError::from(DomainError::DealNotFound),
            ApplicationError::DealNotFound
        ));
        assert!(matches!(
            ApplicationError::from(DomainError::DealParticipationNotFound),
            ApplicationError::DealParticipationNotFound
        ));
        assert!(matches!(
            ApplicationError::from(DomainError::InsufficientPermissions),
            ApplicationError::DealAccessDenied
        ));
        assert!(matches!(
            ApplicationError::from(DomainError::WinWinWinValidationFailed {
                violations: vec!["x".to_string()]
            }),
            ApplicationError::WinWinWinValidationFailed { .. }
        ));
    }

    #[test]
    fn state_transition_and_repository_errors_map_correctly() {
        assert_eq!(
            ApplicationError::from(DomainError::InvalidStateTransition {
                from: "DRAFT".to_string(),
                to: "COMMITTED".to_string(),
            }),
            ApplicationError::InvalidStateTransition {
                from: "DRAFT".to_string(),
                to: "COMMITTED".to_string(),
            }
        );

        assert_eq!(
            ApplicationError::from(DomainError::RepositoryError("boom".to_string())),
            ApplicationError::Infrastructure("boom".to_string())
        );
    }
}
