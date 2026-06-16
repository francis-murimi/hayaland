use crate::errors::DomainError;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Classification of a dispute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DisputeType {
    NonPayment,
    NonDelivery,
    QualityIssue,
    BreachOfTerms,
    Communication,
    ScopeDisagreement,
    DeliveryDelay,
    ForceMajeure,
    Fraud,
    Other,
}

impl DisputeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DisputeType::NonPayment => "NON_PAYMENT",
            DisputeType::NonDelivery => "NON_DELIVERY",
            DisputeType::QualityIssue => "QUALITY_ISSUE",
            DisputeType::BreachOfTerms => "BREACH_OF_TERMS",
            DisputeType::Communication => "COMMUNICATION",
            DisputeType::ScopeDisagreement => "SCOPE_DISAGREEMENT",
            DisputeType::DeliveryDelay => "DELIVERY_DELAY",
            DisputeType::ForceMajeure => "FORCE_MAJEURE",
            DisputeType::Fraud => "FRAUD",
            DisputeType::Other => "OTHER",
        }
    }
}

impl TryFrom<&str> for DisputeType {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "NON_PAYMENT" => Ok(DisputeType::NonPayment),
            "NON_DELIVERY" => Ok(DisputeType::NonDelivery),
            "QUALITY_ISSUE" => Ok(DisputeType::QualityIssue),
            "BREACH_OF_TERMS" => Ok(DisputeType::BreachOfTerms),
            "COMMUNICATION" => Ok(DisputeType::Communication),
            "SCOPE_DISAGREEMENT" => Ok(DisputeType::ScopeDisagreement),
            "DELIVERY_DELAY" => Ok(DisputeType::DeliveryDelay),
            "FORCE_MAJEURE" => Ok(DisputeType::ForceMajeure),
            "FRAUD" => Ok(DisputeType::Fraud),
            "OTHER" => Ok(DisputeType::Other),
            _ => Err(DomainError::InvalidDisputeType {
                message: format!("unknown dispute type: {value}"),
            }),
        }
    }
}

/// Lifecycle status of a dispute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DisputeStatus {
    Open,
    UnderReview,
    Mediation,
    Escalated,
    Resolved,
    Rejected,
}

impl DisputeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DisputeStatus::Open => "OPEN",
            DisputeStatus::UnderReview => "UNDER_REVIEW",
            DisputeStatus::Mediation => "MEDIATION",
            DisputeStatus::Escalated => "ESCALATED",
            DisputeStatus::Resolved => "RESOLVED",
            DisputeStatus::Rejected => "REJECTED",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, DisputeStatus::Resolved | DisputeStatus::Rejected)
    }
}

impl TryFrom<&str> for DisputeStatus {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "OPEN" => Ok(DisputeStatus::Open),
            "UNDER_REVIEW" => Ok(DisputeStatus::UnderReview),
            "MEDIATION" => Ok(DisputeStatus::Mediation),
            "ESCALATED" => Ok(DisputeStatus::Escalated),
            "RESOLVED" => Ok(DisputeStatus::Resolved),
            "REJECTED" => Ok(DisputeStatus::Rejected),
            _ => Err(DomainError::InvalidDisputeStatus {
                message: format!("unknown dispute status: {value}"),
            }),
        }
    }
}

/// How a dispute was resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResolutionType {
    Amicable,
    Mediated,
    Arbitrated,
    Withdrawn,
}

impl ResolutionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResolutionType::Amicable => "AMICABLE",
            ResolutionType::Mediated => "MEDIATED",
            ResolutionType::Arbitrated => "ARBITRATED",
            ResolutionType::Withdrawn => "WITHDRAWN",
        }
    }
}

impl TryFrom<&str> for ResolutionType {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "AMICABLE" => Ok(ResolutionType::Amicable),
            "MEDIATED" => Ok(ResolutionType::Mediated),
            "ARBITRATED" => Ok(ResolutionType::Arbitrated),
            "WITHDRAWN" => Ok(ResolutionType::Withdrawn),
            _ => Err(DomainError::InvalidDisputeResolution {
                message: format!("unknown resolution type: {value}"),
            }),
        }
    }
}

/// Who the resolution favored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResolutionOutcome {
    InFavorOfRaised,
    InFavorOfAgainst,
    Split,
    Dismissed,
}

impl ResolutionOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResolutionOutcome::InFavorOfRaised => "IN_FAVOR_OF_RAISED",
            ResolutionOutcome::InFavorOfAgainst => "IN_FAVOR_OF_AGAINST",
            ResolutionOutcome::Split => "SPLIT",
            ResolutionOutcome::Dismissed => "DISMISSED",
        }
    }
}

impl TryFrom<&str> for ResolutionOutcome {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "IN_FAVOR_OF_RAISED" => Ok(ResolutionOutcome::InFavorOfRaised),
            "IN_FAVOR_OF_AGAINST" => Ok(ResolutionOutcome::InFavorOfAgainst),
            "SPLIT" => Ok(ResolutionOutcome::Split),
            "DISMISSED" => Ok(ResolutionOutcome::Dismissed),
            _ => Err(DomainError::InvalidDisputeResolution {
                message: format!("unknown resolution outcome: {value}"),
            }),
        }
    }
}

/// Impact level of a resolved dispute, used for trust-score penalties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DisputeSeverity {
    Low,
    Medium,
    High,
}

impl DisputeSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            DisputeSeverity::Low => "LOW",
            DisputeSeverity::Medium => "MEDIUM",
            DisputeSeverity::High => "HIGH",
        }
    }
}

impl TryFrom<&str> for DisputeSeverity {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "LOW" => Ok(DisputeSeverity::Low),
            "MEDIUM" => Ok(DisputeSeverity::Medium),
            "HIGH" => Ok(DisputeSeverity::High),
            _ => Err(DomainError::InvalidDisputeResolution {
                message: format!("unknown dispute severity: {value}"),
            }),
        }
    }
}

/// A response posted to a dispute by a participating party.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisputeResponse {
    pub id: Uuid,
    pub dispute_id: Uuid,
    pub party_id: Uuid,
    pub user_id: Uuid,
    pub message: String,
    pub created_at: OffsetDateTime,
}

impl DisputeResponse {
    pub fn new(id: Uuid, dispute_id: Uuid, party_id: Uuid, user_id: Uuid, message: String) -> Self {
        Self {
            id,
            dispute_id,
            party_id,
            user_id,
            message,
            created_at: OffsetDateTime::now_utc(),
        }
    }
}

/// The dispute aggregate root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dispute {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub raised_by_party_id: Uuid,
    pub raised_by_user_id: Uuid,
    pub against_party_id: Option<Uuid>,
    pub dispute_type: DisputeType,
    pub dispute_status: DisputeStatus,
    pub resolution_type: Option<ResolutionType>,
    pub resolution_outcome: Option<ResolutionOutcome>,
    pub severity: Option<DisputeSeverity>,
    pub description: String,
    pub evidence_urls: Vec<String>,
    pub admin_notes: Option<String>,
    pub resolution_notes: Option<String>,
    pub resolved_by_user_id: Option<Uuid>,
    pub resolved_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl Dispute {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Uuid,
        deal_id: Uuid,
        raised_by_party_id: Uuid,
        raised_by_user_id: Uuid,
        against_party_id: Option<Uuid>,
        dispute_type: DisputeType,
        description: String,
        evidence_urls: Vec<String>,
    ) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id,
            deal_id,
            raised_by_party_id,
            raised_by_user_id,
            against_party_id,
            dispute_type,
            dispute_status: DisputeStatus::Open,
            resolution_type: None,
            resolution_outcome: None,
            severity: None,
            description,
            evidence_urls,
            admin_notes: None,
            resolution_notes: None,
            resolved_by_user_id: None,
            resolved_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn submit_for_review(&mut self) -> Result<(), DomainError> {
        if self.dispute_status != DisputeStatus::Open {
            return Err(DomainError::InvalidDisputeStatus {
                message: format!(
                    "cannot submit dispute for review from status {}",
                    self.dispute_status.as_str()
                ),
            });
        }
        self.dispute_status = DisputeStatus::UnderReview;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    pub fn escalate(&mut self) -> Result<(), DomainError> {
        if !matches!(
            self.dispute_status,
            DisputeStatus::Open | DisputeStatus::UnderReview | DisputeStatus::Mediation
        ) {
            return Err(DomainError::InvalidDisputeStatus {
                message: format!(
                    "cannot escalate dispute from status {}",
                    self.dispute_status.as_str()
                ),
            });
        }
        self.dispute_status = DisputeStatus::Escalated;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    pub fn resolve(
        &mut self,
        resolution_type: ResolutionType,
        resolution_outcome: ResolutionOutcome,
        severity: DisputeSeverity,
        resolution_notes: Option<String>,
        resolved_by_user_id: Uuid,
    ) -> Result<(), DomainError> {
        if self.dispute_status.is_terminal() {
            return Err(DomainError::InvalidDisputeStatus {
                message: "dispute is already resolved or rejected".to_string(),
            });
        }
        let now = OffsetDateTime::now_utc();
        self.dispute_status = DisputeStatus::Resolved;
        self.resolution_type = Some(resolution_type);
        self.resolution_outcome = Some(resolution_outcome);
        self.severity = Some(severity);
        self.resolution_notes = resolution_notes;
        self.resolved_by_user_id = Some(resolved_by_user_id);
        self.resolved_at = Some(now);
        self.updated_at = now;
        Ok(())
    }

    pub fn reject(&mut self, reason: String, resolved_by_user_id: Uuid) -> Result<(), DomainError> {
        if self.dispute_status.is_terminal() {
            return Err(DomainError::InvalidDisputeStatus {
                message: "dispute is already resolved or rejected".to_string(),
            });
        }
        let now = OffsetDateTime::now_utc();
        self.dispute_status = DisputeStatus::Rejected;
        self.resolution_notes = Some(reason);
        self.resolved_by_user_id = Some(resolved_by_user_id);
        self.resolved_at = Some(now);
        self.updated_at = now;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_dispute() -> Dispute {
        Dispute::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            None,
            DisputeType::QualityIssue,
            "Quality was not as agreed.".to_string(),
            vec!["https://example.com/evidence.jpg".to_string()],
        )
    }

    #[test]
    fn dispute_new_starts_open() {
        let dispute = sample_dispute();
        assert_eq!(dispute.dispute_status, DisputeStatus::Open);
        assert!(dispute.resolution_type.is_none());
        assert!(dispute.resolved_at.is_none());
    }

    #[test]
    fn submit_for_review_moves_open_to_under_review() {
        let mut dispute = sample_dispute();
        assert!(dispute.submit_for_review().is_ok());
        assert_eq!(dispute.dispute_status, DisputeStatus::UnderReview);
    }

    #[test]
    fn submit_for_review_fails_when_not_open() {
        let mut dispute = sample_dispute();
        dispute.submit_for_review().unwrap();
        let err = dispute.submit_for_review().unwrap_err();
        assert!(matches!(err, DomainError::InvalidDisputeStatus { .. }));
    }

    #[test]
    fn escalate_from_open() {
        let mut dispute = sample_dispute();
        assert!(dispute.escalate().is_ok());
        assert_eq!(dispute.dispute_status, DisputeStatus::Escalated);
    }

    #[test]
    fn escalate_from_under_review() {
        let mut dispute = sample_dispute();
        dispute.submit_for_review().unwrap();
        assert!(dispute.escalate().is_ok());
        assert_eq!(dispute.dispute_status, DisputeStatus::Escalated);
    }

    #[test]
    fn escalate_fails_when_terminal() {
        let mut dispute = sample_dispute();
        dispute
            .resolve(
                ResolutionType::Mediated,
                ResolutionOutcome::Split,
                DisputeSeverity::Medium,
                None,
                Uuid::now_v7(),
            )
            .unwrap();
        let err = dispute.escalate().unwrap_err();
        assert!(matches!(err, DomainError::InvalidDisputeStatus { .. }));
    }

    #[test]
    fn resolve_sets_all_fields() {
        let mut dispute = sample_dispute();
        dispute.submit_for_review().unwrap();
        let resolver = Uuid::now_v7();
        assert!(dispute
            .resolve(
                ResolutionType::Mediated,
                ResolutionOutcome::Split,
                DisputeSeverity::High,
                Some("Partial refund agreed.".to_string()),
                resolver,
            )
            .is_ok());
        assert_eq!(dispute.dispute_status, DisputeStatus::Resolved);
        assert_eq!(dispute.resolution_type, Some(ResolutionType::Mediated));
        assert_eq!(dispute.resolution_outcome, Some(ResolutionOutcome::Split));
        assert_eq!(dispute.severity, Some(DisputeSeverity::High));
        assert_eq!(dispute.resolved_by_user_id, Some(resolver));
        assert!(dispute.resolved_at.is_some());
    }

    #[test]
    fn resolve_fails_when_already_resolved() {
        let mut dispute = sample_dispute();
        dispute
            .resolve(
                ResolutionType::Amicable,
                ResolutionOutcome::InFavorOfRaised,
                DisputeSeverity::Low,
                None,
                Uuid::now_v7(),
            )
            .unwrap();
        let err = dispute
            .resolve(
                ResolutionType::Arbitrated,
                ResolutionOutcome::Dismissed,
                DisputeSeverity::Low,
                None,
                Uuid::now_v7(),
            )
            .unwrap_err();
        assert!(matches!(err, DomainError::InvalidDisputeStatus { .. }));
    }

    #[test]
    fn reject_moves_to_rejected() {
        let mut dispute = sample_dispute();
        let resolver = Uuid::now_v7();
        assert!(dispute
            .reject("No evidence provided.".to_string(), resolver)
            .is_ok());
        assert_eq!(dispute.dispute_status, DisputeStatus::Rejected);
        assert_eq!(dispute.resolved_by_user_id, Some(resolver));
        assert!(dispute.resolved_at.is_some());
    }

    #[test]
    fn reject_fails_when_resolved() {
        let mut dispute = sample_dispute();
        dispute
            .resolve(
                ResolutionType::Amicable,
                ResolutionOutcome::InFavorOfRaised,
                DisputeSeverity::Low,
                None,
                Uuid::now_v7(),
            )
            .unwrap();
        let err = dispute
            .reject("No evidence provided.".to_string(), Uuid::now_v7())
            .unwrap_err();
        assert!(matches!(err, DomainError::InvalidDisputeStatus { .. }));
    }

    #[test]
    fn dispute_type_round_trips() {
        let cases = [
            DisputeType::NonPayment,
            DisputeType::NonDelivery,
            DisputeType::QualityIssue,
            DisputeType::BreachOfTerms,
            DisputeType::Communication,
            DisputeType::ScopeDisagreement,
            DisputeType::DeliveryDelay,
            DisputeType::ForceMajeure,
            DisputeType::Fraud,
            DisputeType::Other,
        ];
        for case in cases {
            assert_eq!(DisputeType::try_from(case.as_str()).unwrap(), case);
        }
    }

    #[test]
    fn dispute_status_round_trips() {
        let cases = [
            DisputeStatus::Open,
            DisputeStatus::UnderReview,
            DisputeStatus::Mediation,
            DisputeStatus::Escalated,
            DisputeStatus::Resolved,
            DisputeStatus::Rejected,
        ];
        for case in cases {
            assert_eq!(DisputeStatus::try_from(case.as_str()).unwrap(), case);
        }
    }

    #[test]
    fn resolution_type_round_trips() {
        let cases = [
            ResolutionType::Amicable,
            ResolutionType::Mediated,
            ResolutionType::Arbitrated,
            ResolutionType::Withdrawn,
        ];
        for case in cases {
            assert_eq!(ResolutionType::try_from(case.as_str()).unwrap(), case);
        }
    }

    #[test]
    fn resolution_outcome_round_trips() {
        let cases = [
            ResolutionOutcome::InFavorOfRaised,
            ResolutionOutcome::InFavorOfAgainst,
            ResolutionOutcome::Split,
            ResolutionOutcome::Dismissed,
        ];
        for case in cases {
            assert_eq!(ResolutionOutcome::try_from(case.as_str()).unwrap(), case);
        }
    }

    #[test]
    fn severity_round_trips() {
        let cases = [
            DisputeSeverity::Low,
            DisputeSeverity::Medium,
            DisputeSeverity::High,
        ];
        for case in cases {
            assert_eq!(DisputeSeverity::try_from(case.as_str()).unwrap(), case);
        }
    }

    #[test]
    fn invalid_dispute_type_returns_error() {
        let err = DisputeType::try_from("UNKNOWN").unwrap_err();
        assert!(matches!(err, DomainError::InvalidDisputeType { .. }));
    }

    #[test]
    fn invalid_dispute_status_returns_error() {
        let err = DisputeStatus::try_from("UNKNOWN").unwrap_err();
        assert!(matches!(err, DomainError::InvalidDisputeStatus { .. }));
    }

    #[test]
    fn invalid_resolution_type_returns_error() {
        let err = ResolutionType::try_from("UNKNOWN").unwrap_err();
        assert!(matches!(err, DomainError::InvalidDisputeResolution { .. }));
    }

    #[test]
    fn invalid_resolution_outcome_returns_error() {
        let err = ResolutionOutcome::try_from("UNKNOWN").unwrap_err();
        assert!(matches!(err, DomainError::InvalidDisputeResolution { .. }));
    }

    #[test]
    fn invalid_severity_returns_error() {
        let err = DisputeSeverity::try_from("UNKNOWN").unwrap_err();
        assert!(matches!(err, DomainError::InvalidDisputeResolution { .. }));
    }

    #[test]
    fn dispute_response_new_stores_fields() {
        let id = Uuid::now_v7();
        let dispute_id = Uuid::now_v7();
        let party_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();
        let response = DisputeResponse::new(
            id,
            dispute_id,
            party_id,
            user_id,
            "We dispute this claim.".to_string(),
        );
        assert_eq!(response.id, id);
        assert_eq!(response.dispute_id, dispute_id);
        assert_eq!(response.party_id, party_id);
        assert_eq!(response.user_id, user_id);
        assert_eq!(response.message, "We dispute this claim.");
    }
}
