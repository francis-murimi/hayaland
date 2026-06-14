use crate::errors::DomainError;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Decision recorded by a party for a pending transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApprovalDecision {
    Approved,
    Rejected,
}

impl ApprovalDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApprovalDecision::Approved => "APPROVED",
            ApprovalDecision::Rejected => "REJECTED",
        }
    }
}

impl TryFrom<&str> for ApprovalDecision {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "APPROVED" => Ok(ApprovalDecision::Approved),
            "REJECTED" => Ok(ApprovalDecision::Rejected),
            _ => Err(DomainError::Validation(vec![format!(
                "unknown approval decision: {value}"
            )])),
        }
    }
}

/// A single approval/rejection recorded against a transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransactionApproval {
    pub id: Uuid,
    pub transaction_id: Uuid,
    pub party_id: Uuid,
    pub approved_by_user_id: Uuid,
    pub decision: ApprovalDecision,
    pub comment: Option<String>,
    pub created_at: OffsetDateTime,
}

impl TransactionApproval {
    pub fn new(
        id: Uuid,
        transaction_id: Uuid,
        party_id: Uuid,
        approved_by_user_id: Uuid,
        decision: ApprovalDecision,
        comment: Option<String>,
    ) -> Self {
        Self {
            id,
            transaction_id,
            party_id,
            approved_by_user_id,
            decision,
            comment,
            created_at: OffsetDateTime::now_utc(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approval_decision_round_trips() {
        for decision in [ApprovalDecision::Approved, ApprovalDecision::Rejected] {
            let s = decision.as_str();
            assert_eq!(ApprovalDecision::try_from(s).unwrap(), decision);
        }
    }

    #[test]
    fn unknown_approval_decision_fails() {
        assert!(ApprovalDecision::try_from("MAYBE").is_err());
    }

    #[test]
    fn new_approval_sets_fields() {
        let id = Uuid::now_v7();
        let transaction_id = Uuid::now_v7();
        let party_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();

        let approval = TransactionApproval::new(
            id,
            transaction_id,
            party_id,
            user_id,
            ApprovalDecision::Approved,
            Some("looks good".to_string()),
        );

        assert_eq!(approval.id, id);
        assert_eq!(approval.transaction_id, transaction_id);
        assert_eq!(approval.party_id, party_id);
        assert_eq!(approval.approved_by_user_id, user_id);
        assert_eq!(approval.decision, ApprovalDecision::Approved);
        assert_eq!(approval.comment, Some("looks good".to_string()));
    }
}
