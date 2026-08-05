use crate::errors::DomainError;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Lifecycle status of a match suggestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MatchStatus {
    Pending,
    Accepted,
    Declined,
    CounterProposed,
    Expired,
    ConvertedToDeal,
}

impl MatchStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            MatchStatus::Pending => "PENDING",
            MatchStatus::Accepted => "ACCEPTED",
            MatchStatus::Declined => "DECLINED",
            MatchStatus::CounterProposed => "COUNTER_PROPOSED",
            MatchStatus::Expired => "EXPIRED",
            MatchStatus::ConvertedToDeal => "CONVERTED_TO_DEAL",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            MatchStatus::Declined | MatchStatus::Expired | MatchStatus::ConvertedToDeal
        )
    }

    pub fn allows_response(&self) -> bool {
        matches!(self, MatchStatus::Pending | MatchStatus::CounterProposed)
    }
}

impl TryFrom<&str> for MatchStatus {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "PENDING" => Ok(MatchStatus::Pending),
            "ACCEPTED" => Ok(MatchStatus::Accepted),
            "DECLINED" => Ok(MatchStatus::Declined),
            "COUNTER_PROPOSED" => Ok(MatchStatus::CounterProposed),
            "EXPIRED" => Ok(MatchStatus::Expired),
            "CONVERTED_TO_DEAL" => Ok(MatchStatus::ConvertedToDeal),
            _ => Err(DomainError::InvalidMatchStatus {
                message: format!("unknown match status: {value}"),
            }),
        }
    }
}

/// Source of a match suggestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MatchGeneratedBy {
    Algorithm,
    PlatformAdmin,
    UserReferral,
}

impl MatchGeneratedBy {
    pub fn as_str(&self) -> &'static str {
        match self {
            MatchGeneratedBy::Algorithm => "ALGORITHM",
            MatchGeneratedBy::PlatformAdmin => "PLATFORM_ADMIN",
            MatchGeneratedBy::UserReferral => "USER_REFERRAL",
        }
    }
}

impl TryFrom<&str> for MatchGeneratedBy {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "ALGORITHM" => Ok(MatchGeneratedBy::Algorithm),
            "PLATFORM_ADMIN" => Ok(MatchGeneratedBy::PlatformAdmin),
            "USER_REFERRAL" => Ok(MatchGeneratedBy::UserReferral),
            _ => Err(DomainError::InvalidMatchStatus {
                message: format!("unknown match generated-by value: {value}"),
            }),
        }
    }
}

/// Weights for the seven compatibility dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MatchScoreWeights {
    pub resource_need_alignment: f64,
    pub value_alignment: f64,
    pub trust_score: f64,
    pub geographic_fit: f64,
    pub temporal_availability: f64,
    pub historical_success: f64,
    pub risk_profile: f64,
}

impl Default for MatchScoreWeights {
    fn default() -> Self {
        Self {
            resource_need_alignment: 0.25,
            value_alignment: 0.20,
            trust_score: 0.15,
            geographic_fit: 0.10,
            temporal_availability: 0.10,
            historical_success: 0.10,
            risk_profile: 0.10,
        }
    }
}

impl MatchScoreWeights {
    pub fn validate(&self) -> Result<(), DomainError> {
        let total = self.resource_need_alignment
            + self.value_alignment
            + self.trust_score
            + self.geographic_fit
            + self.temporal_availability
            + self.historical_success
            + self.risk_profile;
        if (total - 1.0).abs() > 1e-6 {
            return Err(DomainError::Validation(vec![format!(
                "match score weights must sum to 1.0, got {total}"
            )]));
        }
        Ok(())
    }
}

/// Per-dimension score breakdown for explainability.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MatchScoreBreakdown {
    pub resource_need_alignment: f64,
    pub value_alignment: f64,
    pub trust_score: f64,
    pub geographic_fit: f64,
    pub temporal_availability: f64,
    pub historical_success: f64,
    pub risk_profile: f64,
    pub weights: MatchScoreWeights,
}

impl MatchScoreBreakdown {
    pub fn new(scores: [f64; 7], weights: MatchScoreWeights) -> Self {
        Self {
            resource_need_alignment: scores[0],
            value_alignment: scores[1],
            trust_score: scores[2],
            geographic_fit: scores[3],
            temporal_availability: scores[4],
            historical_success: scores[5],
            risk_profile: scores[6],
            weights,
        }
    }

    /// Compute the weighted total score, clamped to [0.0, 1.0].
    pub fn total(&self) -> f64 {
        let score = self.resource_need_alignment * self.weights.resource_need_alignment
            + self.value_alignment * self.weights.value_alignment
            + self.trust_score * self.weights.trust_score
            + self.geographic_fit * self.weights.geographic_fit
            + self.temporal_availability * self.weights.temporal_availability
            + self.historical_success * self.weights.historical_success
            + self.risk_profile * self.weights.risk_profile;
        score.clamp(0.0, 1.0)
    }

    /// Clamp all dimension scores to [0.0, 1.0].
    pub fn clamped(&self) -> Self {
        Self {
            resource_need_alignment: self.resource_need_alignment.clamp(0.0, 1.0),
            value_alignment: self.value_alignment.clamp(0.0, 1.0),
            trust_score: self.trust_score.clamp(0.0, 1.0),
            geographic_fit: self.geographic_fit.clamp(0.0, 1.0),
            temporal_availability: self.temporal_availability.clamp(0.0, 1.0),
            historical_success: self.historical_success.clamp(0.0, 1.0),
            risk_profile: self.risk_profile.clamp(0.0, 1.0),
            weights: self.weights,
        }
    }
}

/// A platform-generated compatibility suggestion linking three parties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchSuggestion {
    pub id: Uuid,
    pub supplier_party_id: Uuid,
    pub consumer_party_id: Uuid,
    pub enhancer_party_id: Uuid,
    pub match_status: MatchStatus,
    pub match_score: f64,
    pub score_breakdown: MatchScoreBreakdown,
    pub match_reason: String,
    pub resource_category_id: Option<Uuid>,
    pub need_category_id: Option<Uuid>,
    pub enhancement_category_id: Option<Uuid>,
    pub suggested_deal_value: Option<Decimal>,
    pub generated_by: MatchGeneratedBy,
    pub expires_at: Option<OffsetDateTime>,
    pub converted_deal_id: Option<Uuid>,
    pub counter_notes: Option<String>,
    pub responded_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl MatchSuggestion {
    pub fn new(
        id: Uuid,
        supplier_party_id: Uuid,
        consumer_party_id: Uuid,
        enhancer_party_id: Uuid,
        score_breakdown: MatchScoreBreakdown,
        match_reason: String,
    ) -> Result<Self, DomainError> {
        if supplier_party_id == consumer_party_id
            || supplier_party_id == enhancer_party_id
            || consumer_party_id == enhancer_party_id
        {
            return Err(DomainError::Validation(vec![
                "match suggestion must reference three distinct parties".to_string(),
            ]));
        }

        let clamped = score_breakdown.clamped();
        let now = OffsetDateTime::now_utc();
        Ok(Self {
            id,
            supplier_party_id,
            consumer_party_id,
            enhancer_party_id,
            match_status: MatchStatus::Pending,
            match_score: clamped.total(),
            score_breakdown: clamped,
            match_reason,
            resource_category_id: None,
            need_category_id: None,
            enhancement_category_id: None,
            suggested_deal_value: None,
            generated_by: MatchGeneratedBy::Algorithm,
            expires_at: None,
            converted_deal_id: None,
            counter_notes: None,
            responded_at: None,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn is_participant(&self, party_id: Uuid) -> bool {
        self.supplier_party_id == party_id
            || self.consumer_party_id == party_id
            || self.enhancer_party_id == party_id
    }

    pub fn can_respond(&self) -> Result<(), DomainError> {
        if self.match_status.is_terminal() {
            return Err(DomainError::InvalidMatchResponse {
                message: format!(
                    "match suggestion is in terminal status {}",
                    self.match_status.as_str()
                ),
            });
        }
        if let Some(expires_at) = self.expires_at {
            if OffsetDateTime::now_utc() > expires_at {
                return Err(DomainError::MatchExpired);
            }
        }
        Ok(())
    }

    pub fn accept(&mut self) {
        self.match_status = MatchStatus::Accepted;
        self.responded_at = Some(OffsetDateTime::now_utc());
        self.updated_at = OffsetDateTime::now_utc();
    }

    pub fn decline(&mut self) {
        self.match_status = MatchStatus::Declined;
        self.responded_at = Some(OffsetDateTime::now_utc());
        self.updated_at = OffsetDateTime::now_utc();
    }

    pub fn counter_propose(&mut self, notes: Option<String>) {
        self.match_status = MatchStatus::CounterProposed;
        self.counter_notes = notes;
        self.responded_at = Some(OffsetDateTime::now_utc());
        self.updated_at = OffsetDateTime::now_utc();
    }

    pub fn mark_converted(&mut self, deal_id: Uuid) {
        self.match_status = MatchStatus::ConvertedToDeal;
        self.converted_deal_id = Some(deal_id);
        self.updated_at = OffsetDateTime::now_utc();
    }

    pub fn all_parties_accepted(&self) -> bool {
        // This method is a placeholder; the application layer tracks per-party
        // responses. The suggestion itself only knows the aggregate status.
        self.match_status == MatchStatus::Accepted
    }

    pub fn set_categories(
        &mut self,
        resource_category_id: Option<Uuid>,
        need_category_id: Option<Uuid>,
        enhancement_category_id: Option<Uuid>,
    ) {
        self.resource_category_id = resource_category_id;
        self.need_category_id = need_category_id;
        self.enhancement_category_id = enhancement_category_id;
        self.updated_at = OffsetDateTime::now_utc();
    }

    pub fn set_suggested_deal_value(&mut self, value: Decimal) {
        self.suggested_deal_value = Some(value);
        self.updated_at = OffsetDateTime::now_utc();
    }

    pub fn set_expires_at(&mut self, expires_at: OffsetDateTime) {
        self.expires_at = Some(expires_at);
        self.updated_at = OffsetDateTime::now_utc();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_breakdown() -> MatchScoreBreakdown {
        MatchScoreBreakdown::new(
            [0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3],
            MatchScoreWeights::default(),
        )
    }

    #[test]
    fn match_status_from_str() {
        assert_eq!(
            MatchStatus::try_from("PENDING").unwrap(),
            MatchStatus::Pending
        );
        assert_eq!(
            MatchStatus::try_from("CONVERTED_TO_DEAL").unwrap(),
            MatchStatus::ConvertedToDeal
        );
        assert!(MatchStatus::try_from("UNKNOWN").is_err());
    }

    #[test]
    fn generated_by_from_str() {
        assert_eq!(
            MatchGeneratedBy::try_from("ALGORITHM").unwrap(),
            MatchGeneratedBy::Algorithm
        );
        assert!(MatchGeneratedBy::try_from("BOT").is_err());
    }

    #[test]
    fn weights_must_sum_to_one() {
        let mut weights = MatchScoreWeights::default();
        weights.trust_score = 0.16;
        assert!(weights.validate().is_err());
    }

    #[test]
    fn default_weights_sum_to_one() {
        assert!(MatchScoreWeights::default().validate().is_ok());
    }

    #[test]
    fn breakdown_total_is_weighted_sum() {
        let b = sample_breakdown();
        let expected = 0.9 * 0.25
            + 0.8 * 0.20
            + 0.7 * 0.15
            + 0.6 * 0.10
            + 0.5 * 0.10
            + 0.4 * 0.10
            + 0.3 * 0.10;
        assert!((b.total() - expected).abs() < 1e-9);
    }

    #[test]
    fn new_rejects_duplicate_parties() {
        let id = Uuid::now_v7();
        assert!(MatchSuggestion::new(
            Uuid::now_v7(),
            id,
            id,
            Uuid::now_v7(),
            sample_breakdown(),
            "reason".to_string(),
        )
        .is_err());
    }

    #[test]
    fn new_clamps_negative_scores() {
        let b = MatchScoreBreakdown::new(
            [-1.0, 2.0, 0.5, 0.5, 0.5, 0.5, 0.5],
            MatchScoreWeights::default(),
        );
        let s = MatchSuggestion::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            b,
            "reason".to_string(),
        )
        .unwrap();
        assert_eq!(s.score_breakdown.resource_need_alignment, 0.0);
        assert_eq!(s.score_breakdown.value_alignment, 1.0);
    }

    #[test]
    fn is_participant_returns_true_for_each_party() {
        let s = MatchSuggestion::new(
            Uuid::now_v7(),
            Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
            Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
            Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap(),
            sample_breakdown(),
            "reason".to_string(),
        )
        .unwrap();
        assert!(s.is_participant(s.supplier_party_id));
        assert!(s.is_participant(s.consumer_party_id));
        assert!(s.is_participant(s.enhancer_party_id));
        assert!(!s.is_participant(Uuid::now_v7()));
    }

    #[test]
    fn can_respond_rejects_terminal_status() {
        let mut s = MatchSuggestion::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            sample_breakdown(),
            "reason".to_string(),
        )
        .unwrap();
        s.decline();
        assert!(s.can_respond().is_err());
    }

    #[test]
    fn accept_and_convert_lifecycle() {
        let mut s = MatchSuggestion::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            sample_breakdown(),
            "reason".to_string(),
        )
        .unwrap();
        s.accept();
        assert_eq!(s.match_status, MatchStatus::Accepted);
        assert!(s.responded_at.is_some());

        let deal_id = Uuid::now_v7();
        s.mark_converted(deal_id);
        assert_eq!(s.match_status, MatchStatus::ConvertedToDeal);
        assert_eq!(s.converted_deal_id, Some(deal_id));
    }

    #[test]
    fn counter_propose_sets_status() {
        let mut s = MatchSuggestion::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            sample_breakdown(),
            "reason".to_string(),
        )
        .unwrap();
        s.counter_propose(Some("different terms".to_string()));
        assert_eq!(s.match_status, MatchStatus::CounterProposed);
        assert_eq!(s.counter_notes, Some("different terms".to_string()));
        assert!(s.responded_at.is_some());
    }

    #[test]
    fn all_parties_accepted_reflects_status() {
        let mut s = MatchSuggestion::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            sample_breakdown(),
            "reason".to_string(),
        )
        .unwrap();
        assert!(!s.all_parties_accepted());
        s.accept();
        assert!(s.all_parties_accepted());
    }

    #[test]
    fn set_categories_and_value() {
        let mut s = MatchSuggestion::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            sample_breakdown(),
            "reason".to_string(),
        )
        .unwrap();
        let rid = Uuid::now_v7();
        let nid = Uuid::now_v7();
        let eid = Uuid::now_v7();
        s.set_categories(Some(rid), Some(nid), Some(eid));
        assert_eq!(s.resource_category_id, Some(rid));
        assert_eq!(s.need_category_id, Some(nid));
        assert_eq!(s.enhancement_category_id, Some(eid));

        s.set_suggested_deal_value(Decimal::from(1000));
        assert_eq!(s.suggested_deal_value, Some(Decimal::from(1000)));
    }

    #[test]
    fn set_expires_at_and_can_respond_rejects_expired() {
        let mut s = MatchSuggestion::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            sample_breakdown(),
            "reason".to_string(),
        )
        .unwrap();
        s.set_expires_at(OffsetDateTime::now_utc() - time::Duration::seconds(1));
        assert!(s.can_respond().is_err());
    }

    #[test]
    fn can_respond_allows_pending_and_counter_proposed() {
        let s = MatchSuggestion::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            sample_breakdown(),
            "reason".to_string(),
        )
        .unwrap();
        assert!(s.can_respond().is_ok());

        let mut countered = s.clone();
        countered.counter_propose(None);
        assert!(countered.can_respond().is_ok());
    }

    #[test]
    fn decline_rejects_response() {
        let mut s = MatchSuggestion::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            sample_breakdown(),
            "reason".to_string(),
        )
        .unwrap();
        s.decline();
        assert!(s.can_respond().is_err());
    }

    #[test]
    fn match_status_helpers() {
        assert!(MatchStatus::Declined.is_terminal());
        assert!(MatchStatus::Expired.is_terminal());
        assert!(MatchStatus::ConvertedToDeal.is_terminal());
        assert!(!MatchStatus::Pending.is_terminal());
        assert!(!MatchStatus::Accepted.is_terminal());
        assert!(!MatchStatus::CounterProposed.is_terminal());

        assert!(MatchStatus::Pending.allows_response());
        assert!(MatchStatus::CounterProposed.allows_response());
        assert!(!MatchStatus::Accepted.allows_response());
        assert!(!MatchStatus::Declined.allows_response());
    }
}
