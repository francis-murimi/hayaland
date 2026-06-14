use domain::entities::DealStatus;
use serde_json::Value;
use time::Duration;

/// Resolved timeout configuration used by `ProcessDealTimeouts`.
#[derive(Debug, Clone)]
pub struct DealTimeoutConfig {
    pub draft_seconds: i64,
    pub suggested_seconds: i64,
    pub pending_review_seconds: i64,
    pub negotiating_seconds: i64,
    pub awaiting_party_seconds: i64,
    pub terms_locked_seconds: i64,
    pub committed_seconds: i64,
    pub on_hold_seconds: i64,
    pub disputed_seconds: i64,
}

impl Default for DealTimeoutConfig {
    fn default() -> Self {
        Self {
            draft_seconds: 604_800,
            suggested_seconds: 1_209_600,
            pending_review_seconds: 1_209_600,
            negotiating_seconds: 2_592_000,
            awaiting_party_seconds: 1_209_600,
            terms_locked_seconds: 1_209_600,
            committed_seconds: 259_200,
            on_hold_seconds: 2_592_000,
            disputed_seconds: 1_209_600,
        }
    }
}

impl DealTimeoutConfig {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        draft_seconds: i64,
        suggested_seconds: i64,
        pending_review_seconds: i64,
        negotiating_seconds: i64,
        awaiting_party_seconds: i64,
        terms_locked_seconds: i64,
        committed_seconds: i64,
        on_hold_seconds: i64,
        disputed_seconds: i64,
    ) -> Self {
        Self {
            draft_seconds,
            suggested_seconds,
            pending_review_seconds,
            negotiating_seconds,
            awaiting_party_seconds,
            terms_locked_seconds,
            committed_seconds,
            on_hold_seconds,
            disputed_seconds,
        }
    }

    /// Returns the effective timeout for a deal in the given status.
    ///
    /// `overrides` is the deal-level JSON object keyed by `DealStatus::as_str()`.
    /// A value of `null`, a missing key, or a non-positive value falls back to
    /// the global default. If the global default is also disabled (`None`),
    /// returns `None`.
    pub fn timeout_for(&self, status: DealStatus, overrides: Option<&Value>) -> Option<Duration> {
        let default_seconds = self.default_seconds_for(status)?;

        let key = status.as_str();
        let seconds = match overrides.and_then(|obj| obj.get(key)) {
            Some(Value::Null) => return None,
            Some(Value::Number(n)) => match n.as_i64() {
                Some(s) if s > 0 => s,
                Some(_) => return None,
                None => default_seconds,
            },
            Some(_) => default_seconds,
            None => default_seconds,
        };

        if seconds <= 0 {
            return None;
        }
        Some(Duration::seconds(seconds))
    }

    /// Returns the status a deal should transition to when its timeout elapses.
    pub fn transition_for(&self, status: DealStatus) -> Option<DealStatus> {
        match status {
            DealStatus::Draft => Some(DealStatus::Expired),
            DealStatus::Suggested => Some(DealStatus::Expired),
            DealStatus::PendingReview => Some(DealStatus::Expired),
            DealStatus::Negotiating => Some(DealStatus::OnHold),
            DealStatus::AwaitingParty => Some(DealStatus::OnHold),
            DealStatus::TermsLocked => Some(DealStatus::Cancelled),
            DealStatus::Committed => Some(DealStatus::Executing),
            DealStatus::OnHold => Some(DealStatus::Cancelled),
            DealStatus::Disputed => Some(DealStatus::OnHold),
            DealStatus::Executing
            | DealStatus::Completed
            | DealStatus::Cancelled
            | DealStatus::Expired => None,
        }
    }

    fn default_seconds_for(&self, status: DealStatus) -> Option<i64> {
        let seconds = match status {
            DealStatus::Draft => self.draft_seconds,
            DealStatus::Suggested => self.suggested_seconds,
            DealStatus::PendingReview => self.pending_review_seconds,
            DealStatus::Negotiating => self.negotiating_seconds,
            DealStatus::AwaitingParty => self.awaiting_party_seconds,
            DealStatus::TermsLocked => self.terms_locked_seconds,
            DealStatus::Committed => self.committed_seconds,
            DealStatus::OnHold => self.on_hold_seconds,
            DealStatus::Disputed => self.disputed_seconds,
            DealStatus::Executing
            | DealStatus::Completed
            | DealStatus::Cancelled
            | DealStatus::Expired => return None,
        };
        if seconds <= 0 {
            None
        } else {
            Some(seconds)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_positive() {
        let config = DealTimeoutConfig::default();
        assert!(config.draft_seconds > 0);
        assert!(config.negotiating_seconds > 0);
        assert!(config.committed_seconds > 0);
    }

    #[test]
    fn timeout_for_uses_defaults() {
        let config = DealTimeoutConfig::default();
        assert_eq!(
            config.timeout_for(DealStatus::Draft, None),
            Some(Duration::seconds(604_800))
        );
    }

    #[test]
    fn timeout_for_uses_per_deal_override() {
        let config = DealTimeoutConfig::default();
        let overrides = serde_json::json!({ "DRAFT": 1209600 });
        assert_eq!(
            config.timeout_for(DealStatus::Draft, Some(&overrides)),
            Some(Duration::seconds(1_209_600))
        );
    }

    #[test]
    fn timeout_for_null_override_disables_timeout() {
        let config = DealTimeoutConfig::default();
        let overrides = serde_json::json!({ "DRAFT": null });
        assert_eq!(
            config.timeout_for(DealStatus::Draft, Some(&overrides)),
            None
        );
    }

    #[test]
    fn timeout_for_zero_override_disables_timeout() {
        let config = DealTimeoutConfig::default();
        let overrides = serde_json::json!({ "DRAFT": 0 });
        assert_eq!(
            config.timeout_for(DealStatus::Draft, Some(&overrides)),
            None
        );
    }

    #[test]
    fn timeout_for_missing_override_falls_back() {
        let config = DealTimeoutConfig::default();
        let overrides = serde_json::json!({ "NEGOTIATING": 100 });
        assert_eq!(
            config.timeout_for(DealStatus::Draft, Some(&overrides)),
            Some(Duration::seconds(604_800))
        );
    }

    #[test]
    fn timeout_for_terminal_status_returns_none() {
        let config = DealTimeoutConfig::default();
        assert_eq!(config.timeout_for(DealStatus::Expired, None), None);
        assert_eq!(config.timeout_for(DealStatus::Completed, None), None);
    }

    #[test]
    fn transition_for_active_statuses() {
        let config = DealTimeoutConfig::default();
        assert_eq!(
            config.transition_for(DealStatus::Draft),
            Some(DealStatus::Expired)
        );
        assert_eq!(
            config.transition_for(DealStatus::Negotiating),
            Some(DealStatus::OnHold)
        );
        assert_eq!(
            config.transition_for(DealStatus::Committed),
            Some(DealStatus::Executing)
        );
        assert_eq!(
            config.transition_for(DealStatus::TermsLocked),
            Some(DealStatus::Cancelled)
        );
    }

    #[test]
    fn transition_for_terminal_statuses_is_none() {
        let config = DealTimeoutConfig::default();
        assert_eq!(config.transition_for(DealStatus::Completed), None);
        assert_eq!(config.transition_for(DealStatus::Cancelled), None);
        assert_eq!(config.transition_for(DealStatus::Expired), None);
    }
}
