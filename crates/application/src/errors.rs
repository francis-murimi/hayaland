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
            | DomainError::InvalidSearchParameters { message } => {
                ApplicationError::Validation(vec![message])
            }
            DomainError::DuplicateEmail => ApplicationError::DuplicateEmail,
            DomainError::DuplicateUsername => ApplicationError::DuplicateUsername,
            DomainError::DuplicatePartyEmail => ApplicationError::DuplicatePartyEmail,
            DomainError::DuplicatePartyRole => ApplicationError::DuplicatePartyRole,
            DomainError::PartyNotFound => ApplicationError::PartyNotFound,
            DomainError::RoleNotFound => ApplicationError::RoleNotFound,
            DomainError::PartyRoleHasActiveDeals => ApplicationError::PartyRoleHasActiveDeals,
            DomainError::PartyHasActiveDeals => ApplicationError::PartyHasActiveDeals,
            DomainError::InvalidDealStatus { message }
            | DomainError::InvalidParticipationStatus { message }
            | DomainError::InvalidDealTitle { message }
            | DomainError::InvalidValueDistribution { message } => {
                ApplicationError::Validation(vec![message])
            }
            DomainError::InvalidStateTransition { from, to } => {
                ApplicationError::InvalidStateTransition { from, to }
            }
            DomainError::DealNotFound => ApplicationError::DealNotFound,
            DomainError::DealParticipationNotFound => ApplicationError::DealParticipationNotFound,
            DomainError::TermNotFound
            | DomainError::MilestoneNotFound
            | DomainError::AgreementNotFound
            | DomainError::TransactionNotFound
            | DomainError::WalletNotFound
            | DomainError::MatchNotFound => ApplicationError::NotFound,
            DomainError::InsufficientPermissions => ApplicationError::DealAccessDenied,
            DomainError::WinWinWinValidationFailed { violations } => {
                ApplicationError::WinWinWinValidationFailed { violations }
            }
            DomainError::RepositoryError(msg) => ApplicationError::Infrastructure(msg),
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
        ];

        for case in cases {
            assert!(matches!(
                ApplicationError::from(case),
                ApplicationError::NotFound
            ));
        }
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
