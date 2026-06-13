use crate::errors::DomainError;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// The type of a negotiable clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TermType {
    Price,
    DeliveryDate,
    QualityStandard,
    PaymentTerms,
    LiabilityCap,
    Custom,
}

impl TermType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TermType::Price => "PRICE",
            TermType::DeliveryDate => "DELIVERY_DATE",
            TermType::QualityStandard => "QUALITY_STANDARD",
            TermType::PaymentTerms => "PAYMENT_TERMS",
            TermType::LiabilityCap => "LIABILITY_CAP",
            TermType::Custom => "CUSTOM",
        }
    }
}

impl TryFrom<&str> for TermType {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "PRICE" => Ok(TermType::Price),
            "DELIVERY_DATE" => Ok(TermType::DeliveryDate),
            "QUALITY_STANDARD" => Ok(TermType::QualityStandard),
            "PAYMENT_TERMS" => Ok(TermType::PaymentTerms),
            "LIABILITY_CAP" => Ok(TermType::LiabilityCap),
            "CUSTOM" => Ok(TermType::Custom),
            _ => Err(DomainError::Validation(vec![format!(
                "unknown term type: {value}"
            )])),
        }
    }
}

/// The status of a term in the negotiation lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TermStatus {
    Proposed,
    Accepted,
    Rejected,
    Countered,
    Withdrawn,
}

impl TermStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TermStatus::Proposed => "PROPOSED",
            TermStatus::Accepted => "ACCEPTED",
            TermStatus::Rejected => "REJECTED",
            TermStatus::Countered => "COUNTERED",
            TermStatus::Withdrawn => "WITHDRAWN",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TermStatus::Accepted | TermStatus::Rejected | TermStatus::Withdrawn
        )
    }
}

impl TryFrom<&str> for TermStatus {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "PROPOSED" => Ok(TermStatus::Proposed),
            "ACCEPTED" => Ok(TermStatus::Accepted),
            "REJECTED" => Ok(TermStatus::Rejected),
            "COUNTERED" => Ok(TermStatus::Countered),
            "WITHDRAWN" => Ok(TermStatus::Withdrawn),
            _ => Err(DomainError::Validation(vec![format!(
                "unknown term status: {value}"
            )])),
        }
    }
}

/// A single negotiable clause within a deal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Term {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub proposed_by_party_id: Uuid,
    pub term_type: TermType,
    pub term_name: String,
    pub description: String,
    pub negotiation_status: TermStatus,
    pub parent_term_id: Option<Uuid>,
    pub version: i32,
    pub proposed_at: OffsetDateTime,
    pub resolved_at: Option<OffsetDateTime>,
    pub is_mandatory: bool,
    pub resolution: Option<String>,
    pub created_at: OffsetDateTime,
}

impl Term {
    pub fn new(
        id: Uuid,
        deal_id: Uuid,
        proposed_by_party_id: Uuid,
        term_type: TermType,
        term_name: impl Into<String>,
        description: impl Into<String>,
        is_mandatory: bool,
    ) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id,
            deal_id,
            proposed_by_party_id,
            term_type,
            term_name: term_name.into(),
            description: description.into(),
            negotiation_status: TermStatus::Proposed,
            parent_term_id: None,
            version: 1,
            proposed_at: now,
            resolved_at: None,
            is_mandatory,
            resolution: None,
            created_at: now,
        }
    }

    /// Create a counter-proposal that supersedes this term.
    pub fn counter(
        &self,
        new_id: Uuid,
        proposed_by_party_id: Uuid,
        description: impl Into<String>,
    ) -> Result<Self, DomainError> {
        if self.negotiation_status.is_terminal() {
            return Err(DomainError::Validation(vec![
                "cannot counter a term that is already resolved".to_string(),
            ]));
        }

        let now = OffsetDateTime::now_utc();
        Ok(Self {
            id: new_id,
            deal_id: self.deal_id,
            proposed_by_party_id,
            term_type: self.term_type,
            term_name: self.term_name.clone(),
            description: description.into(),
            negotiation_status: TermStatus::Proposed,
            parent_term_id: Some(self.id),
            version: self.version + 1,
            proposed_at: now,
            resolved_at: None,
            is_mandatory: self.is_mandatory,
            resolution: None,
            created_at: now,
        })
    }

    fn resolve(&mut self, status: TermStatus) -> Result<(), DomainError> {
        if self.negotiation_status.is_terminal() {
            return Err(DomainError::Validation(vec![format!(
                "term is already {}",
                self.negotiation_status.as_str()
            )]));
        }
        self.negotiation_status = status;
        self.resolved_at = Some(OffsetDateTime::now_utc());
        Ok(())
    }

    pub fn accept(&mut self) -> Result<(), DomainError> {
        self.resolve(TermStatus::Accepted)
    }

    pub fn reject(&mut self) -> Result<(), DomainError> {
        self.resolve(TermStatus::Rejected)
    }

    pub fn withdraw(&mut self) -> Result<(), DomainError> {
        if self.negotiation_status != TermStatus::Proposed {
            return Err(DomainError::Validation(vec![
                "only proposed terms can be withdrawn".to_string(),
            ]));
        }
        self.resolve(TermStatus::Withdrawn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_term() -> Term {
        Term::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            TermType::Price,
            "Price",
            "1000 points",
            true,
        )
    }

    #[test]
    fn new_term_is_proposed() {
        let term = sample_term();
        assert_eq!(term.negotiation_status, TermStatus::Proposed);
        assert_eq!(term.version, 1);
    }

    #[test]
    fn accept_resolves_term() {
        let mut term = sample_term();
        term.accept().unwrap();
        assert_eq!(term.negotiation_status, TermStatus::Accepted);
        assert!(term.resolved_at.is_some());
    }

    #[test]
    fn counter_creates_new_version() {
        let term = sample_term();
        let counter = term
            .counter(Uuid::now_v7(), term.proposed_by_party_id, "1100 points")
            .unwrap();
        assert_eq!(counter.version, 2);
        assert_eq!(counter.parent_term_id, Some(term.id));
        assert_eq!(counter.negotiation_status, TermStatus::Proposed);
    }

    #[test]
    fn cannot_counter_accepted_term() {
        let mut term = sample_term();
        term.accept().unwrap();
        let result = term.counter(Uuid::now_v7(), term.proposed_by_party_id, "x");
        assert!(result.is_err());
    }

    #[test]
    fn withdraw_only_from_proposed() {
        let mut term = sample_term();
        term.withdraw().unwrap();
        assert_eq!(term.negotiation_status, TermStatus::Withdrawn);

        let mut accepted = sample_term();
        accepted.accept().unwrap();
        assert!(accepted.withdraw().is_err());
    }
}
