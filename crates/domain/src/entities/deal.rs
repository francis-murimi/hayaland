use crate::errors::DomainError;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use super::DealRole;

/// Status of a deal in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DealStatus {
    Draft,
    Suggested,
    PendingReview,
    Negotiating,
    AwaitingParty,
    TermsLocked,
    Committed,
    Executing,
    OnHold,
    Completed,
    Disputed,
    Cancelled,
    Expired,
}

impl DealStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DealStatus::Draft => "DRAFT",
            DealStatus::Suggested => "SUGGESTED",
            DealStatus::PendingReview => "PENDING_REVIEW",
            DealStatus::Negotiating => "NEGOTIATING",
            DealStatus::AwaitingParty => "AWAITING_PARTY",
            DealStatus::TermsLocked => "TERMS_LOCKED",
            DealStatus::Committed => "COMMITTED",
            DealStatus::Executing => "EXECUTING",
            DealStatus::OnHold => "ON_HOLD",
            DealStatus::Completed => "COMPLETED",
            DealStatus::Disputed => "DISPUTED",
            DealStatus::Cancelled => "CANCELLED",
            DealStatus::Expired => "EXPIRED",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            DealStatus::Completed | DealStatus::Cancelled | DealStatus::Expired
        )
    }

    pub fn is_active(&self) -> bool {
        !self.is_terminal()
            && !matches!(
                self,
                DealStatus::Draft | DealStatus::Suggested | DealStatus::PendingReview
            )
    }
}

impl TryFrom<&str> for DealStatus {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "DRAFT" => Ok(DealStatus::Draft),
            "SUGGESTED" => Ok(DealStatus::Suggested),
            "PENDING_REVIEW" => Ok(DealStatus::PendingReview),
            "NEGOTIATING" => Ok(DealStatus::Negotiating),
            "AWAITING_PARTY" => Ok(DealStatus::AwaitingParty),
            "TERMS_LOCKED" => Ok(DealStatus::TermsLocked),
            "COMMITTED" => Ok(DealStatus::Committed),
            "EXECUTING" => Ok(DealStatus::Executing),
            "ON_HOLD" => Ok(DealStatus::OnHold),
            "COMPLETED" => Ok(DealStatus::Completed),
            "DISPUTED" => Ok(DealStatus::Disputed),
            "CANCELLED" => Ok(DealStatus::Cancelled),
            "EXPIRED" => Ok(DealStatus::Expired),
            _ => Err(DomainError::InvalidDealStatus {
                message: format!("unknown deal status: {value}"),
            }),
        }
    }
}

/// Status of a party's participation in a deal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ParticipationStatus {
    Invited,
    Pending,
    Accepted,
    Declined,
    Withdrawn,
}

impl ParticipationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ParticipationStatus::Invited => "INVITED",
            ParticipationStatus::Pending => "PENDING",
            ParticipationStatus::Accepted => "ACCEPTED",
            ParticipationStatus::Declined => "DECLINED",
            ParticipationStatus::Withdrawn => "WITHDRAWN",
        }
    }
}

impl TryFrom<&str> for ParticipationStatus {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "INVITED" => Ok(ParticipationStatus::Invited),
            "PENDING" => Ok(ParticipationStatus::Pending),
            "ACCEPTED" => Ok(ParticipationStatus::Accepted),
            "DECLINED" => Ok(ParticipationStatus::Declined),
            "WITHDRAWN" => Ok(ParticipationStatus::Withdrawn),
            _ => Err(DomainError::InvalidParticipationStatus {
                message: format!("unknown participation status: {value}"),
            }),
        }
    }
}

/// A validated deal title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DealTitle(String);

impl DealTitle {
    pub fn new(value: &str) -> Result<Self, DomainError> {
        let trimmed = value.trim();
        let len = trimmed.chars().count();
        if !(3..=200).contains(&len) {
            return Err(DomainError::InvalidDealTitle {
                message: "deal title must be between 3 and 200 characters".to_string(),
            });
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Link between a party and a deal in a specific role.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DealParticipation {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub party_id: Uuid,
    pub role: DealRole,
    pub participation_status: ParticipationStatus,
    pub is_initiator: bool,
    pub value_share_percentage: Option<rust_decimal::Decimal>,
    pub value_share_amount: Option<rust_decimal::Decimal>,
    pub invited_at: Option<OffsetDateTime>,
    pub responded_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

impl DealParticipation {
    pub fn new(
        id: Uuid,
        deal_id: Uuid,
        party_id: Uuid,
        role: DealRole,
        is_initiator: bool,
    ) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id,
            deal_id,
            party_id,
            role,
            participation_status: if is_initiator {
                ParticipationStatus::Accepted
            } else {
                ParticipationStatus::Invited
            },
            is_initiator,
            value_share_percentage: None,
            value_share_amount: None,
            invited_at: Some(now),
            responded_at: if is_initiator { Some(now) } else { None },
            created_at: now,
        }
    }
}

/// The central aggregate representing a 3-party deal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Deal {
    pub id: Uuid,
    pub deal_reference: String,
    pub deal_title: DealTitle,
    pub deal_description: Option<String>,
    pub domain_category_id: Uuid,
    pub initiator_party_id: Uuid,
    pub initiator_role: DealRole,
    pub deal_status: DealStatus,
    pub expected_start_date: Option<time::Date>,
    pub expected_end_date: Option<time::Date>,
    pub actual_start_date: Option<time::Date>,
    pub actual_end_date: Option<time::Date>,
    pub timeline: Option<serde_json::Value>,
    pub location: Option<super::GeoPoint>,
    pub location_address: Option<serde_json::Value>,
    pub total_deal_value: Option<rust_decimal::Decimal>,
    pub currency: String,
    pub platform_fee_percentage: rust_decimal::Decimal,
    pub platform_fee_amount: rust_decimal::Decimal,
    pub win_win_win_validated: bool,
    pub validation_checked_at: Option<OffsetDateTime>,
    pub validation_score: Option<rust_decimal::Decimal>,
    pub validation_result: Option<serde_json::Value>,
    pub is_public: bool,
    pub current_state_entered_at: OffsetDateTime,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl Deal {
    pub fn new(
        id: Uuid,
        deal_reference: String,
        title: DealTitle,
        domain_category_id: Uuid,
        initiator_party_id: Uuid,
        initiator_role: DealRole,
    ) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id,
            deal_reference,
            deal_title: title,
            deal_description: None,
            domain_category_id,
            initiator_party_id,
            initiator_role,
            deal_status: DealStatus::Draft,
            expected_start_date: None,
            expected_end_date: None,
            actual_start_date: None,
            actual_end_date: None,
            timeline: None,
            location: None,
            location_address: None,
            total_deal_value: None,
            currency: "POINTS".to_string(),
            platform_fee_percentage: rust_decimal::Decimal::ZERO,
            platform_fee_amount: rust_decimal::Decimal::ZERO,
            win_win_win_validated: false,
            validation_checked_at: None,
            validation_score: None,
            validation_result: None,
            is_public: false,
            current_state_entered_at: now,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn can_transition(&self, to: DealStatus) -> Result<(), DomainError> {
        use DealStatus::*;
        let allowed = match self.deal_status {
            Draft => [Suggested, Cancelled, Expired].to_vec(),
            Suggested => [PendingReview, Cancelled, Expired].to_vec(),
            PendingReview => [Negotiating, Cancelled, Expired].to_vec(),
            Negotiating => [TermsLocked, AwaitingParty, OnHold, Cancelled].to_vec(),
            AwaitingParty => [Negotiating, OnHold, Cancelled].to_vec(),
            TermsLocked => [Committed, Negotiating, Cancelled].to_vec(),
            Committed => [Executing, Cancelled].to_vec(),
            Executing => [Completed, Disputed, AwaitingParty, OnHold, Cancelled].to_vec(),
            OnHold => [Negotiating, Executing, Cancelled].to_vec(),
            Disputed => [Executing, Completed, OnHold, Cancelled].to_vec(),
            Completed | Cancelled | Expired => vec![],
        };

        if allowed.contains(&to) {
            Ok(())
        } else {
            Err(DomainError::InvalidStateTransition {
                from: self.deal_status.as_str().to_string(),
                to: to.as_str().to_string(),
            })
        }
    }

    pub fn transition(&mut self, to: DealStatus) -> Result<(), DomainError> {
        self.can_transition(to)?;
        self.deal_status = to;
        let now = OffsetDateTime::now_utc();
        self.current_state_entered_at = now;
        self.updated_at = now;
        Ok(())
    }

    pub fn set_timeline(
        &mut self,
        expected_start: Option<time::Date>,
        expected_end: Option<time::Date>,
        timeline: Option<serde_json::Value>,
    ) {
        self.expected_start_date = expected_start;
        self.expected_end_date = expected_end;
        self.timeline = timeline;
        self.updated_at = OffsetDateTime::now_utc();
    }

    pub fn set_platform_fee(
        &mut self,
        percentage: rust_decimal::Decimal,
        amount: rust_decimal::Decimal,
    ) {
        self.platform_fee_percentage = percentage;
        self.platform_fee_amount = amount;
        self.updated_at = OffsetDateTime::now_utc();
    }

    pub fn set_value_distribution(
        &mut self,
        total_value: rust_decimal::Decimal,
        validation_score: rust_decimal::Decimal,
    ) {
        self.total_deal_value = Some(total_value);
        self.validation_score = Some(validation_score);
        self.validation_checked_at = Some(OffsetDateTime::now_utc());
        self.win_win_win_validated = validation_score >= rust_decimal::Decimal::from(50);
        self.updated_at = OffsetDateTime::now_utc();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_deal() -> Deal {
        Deal::new(
            Uuid::now_v7(),
            "DL-2026-0001".to_string(),
            DealTitle::new("Sample Deal").unwrap(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            DealRole::Supplier,
        )
    }

    #[test]
    fn deal_starts_as_draft() {
        let deal = sample_deal();
        assert_eq!(deal.deal_status, DealStatus::Draft);
        assert!(!deal.win_win_win_validated);
        assert_eq!(deal.currency, "POINTS");
    }

    #[test]
    fn title_rejects_too_short() {
        assert!(DealTitle::new("ab").is_err());
    }

    #[test]
    fn title_rejects_too_long() {
        assert!(DealTitle::new(&"a".repeat(201)).is_err());
    }

    #[test]
    fn title_accepts_valid() {
        let title = DealTitle::new("Farm Share Crop Agreement").unwrap();
        assert_eq!(title.as_str(), "Farm Share Crop Agreement");
    }

    #[test]
    fn valid_transition_draft_to_suggested() {
        let mut deal = sample_deal();
        assert!(deal.transition(DealStatus::Suggested).is_ok());
        assert_eq!(deal.deal_status, DealStatus::Suggested);
    }

    #[test]
    fn invalid_transition_draft_to_completed() {
        let mut deal = sample_deal();
        assert!(deal.transition(DealStatus::Completed).is_err());
    }

    #[test]
    fn terminal_states_have_no_outgoing_transitions() {
        for status in [
            DealStatus::Completed,
            DealStatus::Cancelled,
            DealStatus::Expired,
        ] {
            let mut deal = sample_deal();
            deal.deal_status = status;
            assert!(deal.transition(DealStatus::Draft).is_err());
        }
    }

    #[test]
    fn deal_role_from_str() {
        assert_eq!(DealRole::try_from("SUPPLIER").unwrap(), DealRole::Supplier);
        assert_eq!(DealRole::try_from("CONSUMER").unwrap(), DealRole::Consumer);
        assert_eq!(DealRole::try_from("ENHANCER").unwrap(), DealRole::Enhancer);
        assert!(DealRole::try_from("UNKNOWN").is_err());
    }

    #[test]
    fn deal_status_from_str() {
        assert_eq!(
            DealStatus::try_from("NEGOTIATING").unwrap(),
            DealStatus::Negotiating
        );
        assert!(DealStatus::try_from("UNKNOWN").is_err());
    }

    #[test]
    fn participation_initiator_is_accepted() {
        let p = DealParticipation::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            DealRole::Supplier,
            true,
        );
        assert_eq!(p.participation_status, ParticipationStatus::Accepted);
        assert!(p.is_initiator);
    }

    #[test]
    fn participation_invited_is_not_accepted() {
        let p = DealParticipation::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            DealRole::Consumer,
            false,
        );
        assert_eq!(p.participation_status, ParticipationStatus::Invited);
        assert!(!p.is_initiator);
    }

    #[test]
    fn set_value_distribution_marks_validated_above_threshold() {
        let mut deal = sample_deal();
        deal.set_value_distribution(
            rust_decimal::Decimal::from(1000),
            rust_decimal::Decimal::from(75),
        );
        assert!(deal.win_win_win_validated);
        assert_eq!(deal.total_deal_value.unwrap(), 1000.into());
    }

    #[test]
    fn set_value_distribution_does_not_validate_below_threshold() {
        let mut deal = sample_deal();
        deal.set_value_distribution(
            rust_decimal::Decimal::from(1000),
            rust_decimal::Decimal::from(45),
        );
        assert!(!deal.win_win_win_validated);
    }
}
