use crate::errors::DomainError;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// The kind of privileged action captured in the audit log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AdminActionType {
    AgreementUpdated,
    PartyUpdated,
    PartyDeleted,
    VerificationApproved,
    VerificationRejected,
    VerificationRevoked,
    DisputeResolved,
    DisputeRejected,
    DisputeEscalated,
    CatalogFlagsUpdated,
    NotificationSent,
    TemplateCreated,
    TemplateUpdated,
    TemplateDeleted,
    ConfigUpdated,
    DealStateChanged,
}

impl AdminActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AdminActionType::AgreementUpdated => "AGREEMENT_UPDATED",
            AdminActionType::PartyUpdated => "PARTY_UPDATED",
            AdminActionType::PartyDeleted => "PARTY_DELETED",
            AdminActionType::VerificationApproved => "VERIFICATION_APPROVED",
            AdminActionType::VerificationRejected => "VERIFICATION_REJECTED",
            AdminActionType::VerificationRevoked => "VERIFICATION_REVOKED",
            AdminActionType::DisputeResolved => "DISPUTE_RESOLVED",
            AdminActionType::DisputeRejected => "DISPUTE_REJECTED",
            AdminActionType::DisputeEscalated => "DISPUTE_ESCALATED",
            AdminActionType::CatalogFlagsUpdated => "CATALOG_FLAGS_UPDATED",
            AdminActionType::NotificationSent => "NOTIFICATION_SENT",
            AdminActionType::TemplateCreated => "TEMPLATE_CREATED",
            AdminActionType::TemplateUpdated => "TEMPLATE_UPDATED",
            AdminActionType::TemplateDeleted => "TEMPLATE_DELETED",
            AdminActionType::ConfigUpdated => "CONFIG_UPDATED",
            AdminActionType::DealStateChanged => "DEAL_STATE_CHANGED",
        }
    }
}

impl TryFrom<&str> for AdminActionType {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "AGREEMENT_UPDATED" => Ok(AdminActionType::AgreementUpdated),
            "PARTY_UPDATED" => Ok(AdminActionType::PartyUpdated),
            "PARTY_DELETED" => Ok(AdminActionType::PartyDeleted),
            "VERIFICATION_APPROVED" => Ok(AdminActionType::VerificationApproved),
            "VERIFICATION_REJECTED" => Ok(AdminActionType::VerificationRejected),
            "VERIFICATION_REVOKED" => Ok(AdminActionType::VerificationRevoked),
            "DISPUTE_RESOLVED" => Ok(AdminActionType::DisputeResolved),
            "DISPUTE_REJECTED" => Ok(AdminActionType::DisputeRejected),
            "DISPUTE_ESCALATED" => Ok(AdminActionType::DisputeEscalated),
            "CATALOG_FLAGS_UPDATED" => Ok(AdminActionType::CatalogFlagsUpdated),
            "NOTIFICATION_SENT" => Ok(AdminActionType::NotificationSent),
            "TEMPLATE_CREATED" => Ok(AdminActionType::TemplateCreated),
            "TEMPLATE_UPDATED" => Ok(AdminActionType::TemplateUpdated),
            "TEMPLATE_DELETED" => Ok(AdminActionType::TemplateDeleted),
            "CONFIG_UPDATED" => Ok(AdminActionType::ConfigUpdated),
            "DEAL_STATE_CHANGED" => Ok(AdminActionType::DealStateChanged),
            _ => Err(DomainError::InvalidAdminActionType {
                message: format!("unknown admin action type: {value}"),
            }),
        }
    }
}

/// The category of entity an admin action targeted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AdminActionTargetType {
    Party,
    Deal,
    Agreement,
    Verification,
    Dispute,
    CatalogItem,
    NotificationTemplate,
    PlatformConfig,
    User,
}

impl AdminActionTargetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AdminActionTargetType::Party => "PARTY",
            AdminActionTargetType::Deal => "DEAL",
            AdminActionTargetType::Agreement => "AGREEMENT",
            AdminActionTargetType::Verification => "VERIFICATION",
            AdminActionTargetType::Dispute => "DISPUTE",
            AdminActionTargetType::CatalogItem => "CATALOG_ITEM",
            AdminActionTargetType::NotificationTemplate => "NOTIFICATION_TEMPLATE",
            AdminActionTargetType::PlatformConfig => "PLATFORM_CONFIG",
            AdminActionTargetType::User => "USER",
        }
    }
}

impl TryFrom<&str> for AdminActionTargetType {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "PARTY" => Ok(AdminActionTargetType::Party),
            "DEAL" => Ok(AdminActionTargetType::Deal),
            "AGREEMENT" => Ok(AdminActionTargetType::Agreement),
            "VERIFICATION" => Ok(AdminActionTargetType::Verification),
            "DISPUTE" => Ok(AdminActionTargetType::Dispute),
            "CATALOG_ITEM" => Ok(AdminActionTargetType::CatalogItem),
            "NOTIFICATION_TEMPLATE" => Ok(AdminActionTargetType::NotificationTemplate),
            "PLATFORM_CONFIG" => Ok(AdminActionTargetType::PlatformConfig),
            "USER" => Ok(AdminActionTargetType::User),
            _ => Err(DomainError::InvalidAdminActionTargetType {
                message: format!("unknown admin action target type: {value}"),
            }),
        }
    }
}

/// A single privileged action recorded for compliance and operational safety.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdminAction {
    pub id: Uuid,
    pub admin_user_id: Uuid,
    pub action_type: AdminActionType,
    pub target_type: AdminActionTargetType,
    pub target_id: Uuid,
    pub before_snapshot: Option<serde_json::Value>,
    pub after_snapshot: Option<serde_json::Value>,
    pub reason: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: OffsetDateTime,
}

impl AdminAction {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Uuid,
        admin_user_id: Uuid,
        action_type: AdminActionType,
        target_type: AdminActionTargetType,
        target_id: Uuid,
        before_snapshot: Option<serde_json::Value>,
        after_snapshot: Option<serde_json::Value>,
        reason: Option<String>,
        ip_address: Option<String>,
    ) -> Self {
        Self {
            id,
            admin_user_id,
            action_type,
            target_type,
            target_id,
            before_snapshot,
            after_snapshot,
            reason,
            ip_address,
            created_at: OffsetDateTime::now_utc(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_action_type_round_trip() {
        for t in [
            AdminActionType::AgreementUpdated,
            AdminActionType::PartyUpdated,
            AdminActionType::VerificationApproved,
            AdminActionType::DisputeResolved,
            AdminActionType::CatalogFlagsUpdated,
            AdminActionType::NotificationSent,
        ] {
            assert_eq!(AdminActionType::try_from(t.as_str()).unwrap(), t);
        }
    }

    #[test]
    fn admin_action_target_type_round_trip() {
        for t in [
            AdminActionTargetType::Party,
            AdminActionTargetType::Deal,
            AdminActionTargetType::CatalogItem,
        ] {
            assert_eq!(AdminActionTargetType::try_from(t.as_str()).unwrap(), t);
        }
    }

    #[test]
    fn admin_action_type_rejects_unknown() {
        assert!(AdminActionType::try_from("UNKNOWN").is_err());
    }
}
