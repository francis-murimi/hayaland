use crate::deals::timeout_config::DealTimeoutConfig;
use crate::errors::ApplicationError;
use domain::entities::{Deal, DealStatus};
use domain::repositories::{DealRepository, MilestoneRepository};
use std::sync::Arc;
use time::OffsetDateTime;
use tracing::{info, warn};
use uuid::Uuid;

/// Summary returned by `ProcessDealTimeouts`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProcessDealTimeoutsResult {
    pub transitioned: Vec<Uuid>,
    pub blocked: Vec<Uuid>,
    pub skipped: Vec<Uuid>,
    pub errors: Vec<Uuid>,
}

/// Automatically advances deals whose time in a transient state has elapsed.
pub struct ProcessDealTimeouts {
    deal_repo: Arc<dyn DealRepository>,
    milestone_repo: Arc<dyn MilestoneRepository>,
    config: DealTimeoutConfig,
}

impl ProcessDealTimeouts {
    pub fn new(
        deal_repo: Arc<dyn DealRepository>,
        milestone_repo: Arc<dyn MilestoneRepository>,
        config: DealTimeoutConfig,
    ) -> Self {
        Self {
            deal_repo,
            milestone_repo,
            config,
        }
    }

    pub async fn execute(
        &self,
        batch_size: usize,
    ) -> Result<ProcessDealTimeoutsResult, ApplicationError> {
        let mut result = ProcessDealTimeoutsResult::default();
        let now = OffsetDateTime::now_utc();
        let batch_size = batch_size.max(1) as i64;

        for &status in active_statuses() {
            let Some(target) = self.config.transition_for(status) else {
                continue;
            };
            let Some(default_timeout) = self.config.timeout_for(status, None) else {
                continue;
            };

            // Fetch candidates that entered the state before the default timeout horizon.
            // Per-deal overrides are evaluated afterwards so longer overrides are not missed.
            let horizon = now - default_timeout;
            let candidates = self
                .deal_repo
                .find_deals_by_status(status, horizon, batch_size)
                .await?;

            for mut deal in candidates {
                let effective_timeout = self
                    .config
                    .timeout_for(deal.deal_status, deal.timeout_overrides.as_ref());
                let Some(effective_timeout) = effective_timeout else {
                    result.skipped.push(deal.id);
                    continue;
                };

                if now < deal.current_state_entered_at + effective_timeout {
                    result.skipped.push(deal.id);
                    continue;
                }

                if let Some(reason) = self.block_reason(&deal, target).await? {
                    warn!(
                        deal_id = %deal.id,
                        status = %status.as_str(),
                        reason = %reason,
                        "deal_timeout_blocked"
                    );
                    self.deal_repo
                        .record_history(
                            deal.id,
                            "DEAL_TIMEOUT_BLOCKED",
                            None,
                            Some(serde_json::json!({
                                "from_status": status.as_str(),
                                "to_status": target.as_str(),
                                "reason": reason,
                                "triggered_at": now,
                            })),
                        )
                        .await?;
                    result.blocked.push(deal.id);
                    continue;
                }

                match deal.transition(target) {
                    Ok(()) => {}
                    Err(err) => {
                        warn!(
                            deal_id = %deal.id,
                            error = %err,
                            "deal_timeout_transition_failed"
                        );
                        result.errors.push(deal.id);
                        continue;
                    }
                }

                if target == DealStatus::Executing {
                    deal.actual_start_date = Some(OffsetDateTime::now_utc().date());
                }

                self.deal_repo.update(&deal).await?;

                let timeout_seconds = effective_timeout.whole_seconds();
                self.deal_repo
                    .record_history(
                        deal.id,
                        "DEAL_TIMEOUT_TRANSITION",
                        None,
                        Some(serde_json::json!({
                            "from_status": status.as_str(),
                            "to_status": target.as_str(),
                            "timeout_seconds": timeout_seconds,
                            "triggered_at": now,
                        })),
                    )
                    .await?;

                info!(
                    deal_id = %deal.id,
                    from_status = %status.as_str(),
                    to_status = %target.as_str(),
                    timeout_seconds = %timeout_seconds,
                    "deal_timeout_transition"
                );
                result.transitioned.push(deal.id);
            }
        }

        Ok(result)
    }

    async fn block_reason(
        &self,
        deal: &Deal,
        target: DealStatus,
    ) -> Result<Option<String>, ApplicationError> {
        if target == DealStatus::Executing && deal.deal_status == DealStatus::Committed {
            let count = self.milestone_repo.count_by_deal(deal.id).await?;
            if count == 0 {
                return Ok(Some("milestones are required before executing".to_string()));
            }
        }
        Ok(None)
    }
}

fn active_statuses() -> &'static [DealStatus] {
    &[
        DealStatus::Draft,
        DealStatus::Suggested,
        DealStatus::PendingReview,
        DealStatus::Negotiating,
        DealStatus::AwaitingParty,
        DealStatus::TermsLocked,
        DealStatus::Committed,
        DealStatus::OnHold,
        DealStatus::Disputed,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{FakeDealRepo, FakeMilestoneRepo};
    use domain::entities::{Deal, DealRole, DealTitle};
    use domain::repositories::{DealAggregate, DealRepository};
    use std::sync::Arc;
    use time::Duration;

    fn sample_deal(status: DealStatus) -> Deal {
        let mut deal = Deal::new(
            Uuid::now_v7(),
            "DL-2026-0001".to_string(),
            DealTitle::new("Sample").unwrap(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            DealRole::Supplier,
        );
        deal.deal_status = status;
        deal.current_state_entered_at = OffsetDateTime::now_utc() - Duration::days(10);
        deal
    }

    #[tokio::test]
    async fn draft_deal_expires_after_timeout() {
        let deal_repo = Arc::new(FakeDealRepo::default());
        let milestone_repo = Arc::new(FakeMilestoneRepo::default());
        let mut deal = sample_deal(DealStatus::Draft);
        deal.current_state_entered_at = OffsetDateTime::now_utc() - Duration::seconds(100);
        deal_repo
            .create(&DealAggregate {
                deal: deal.clone(),
                participations: vec![],
            })
            .await
            .unwrap();

        let config = DealTimeoutConfig::new(60, 120, 120, 200, 120, 120, 80, 200, 120);
        let use_case = ProcessDealTimeouts::new(deal_repo.clone(), milestone_repo, config);
        let result = use_case.execute(10).await.unwrap();

        assert_eq!(result.transitioned, vec![deal.id]);
        let updated = deal_repo.find_by_id(deal.id).await.unwrap().unwrap();
        assert_eq!(updated.deal_status, DealStatus::Expired);

        let history = deal_repo.history.lock().unwrap();
        assert!(history
            .iter()
            .any(|(_, event, _, _)| event == "DEAL_TIMEOUT_TRANSITION"));
    }

    #[tokio::test]
    async fn deal_with_disabled_timeout_is_skipped() {
        let deal_repo = Arc::new(FakeDealRepo::default());
        let milestone_repo = Arc::new(FakeMilestoneRepo::default());
        let mut deal = sample_deal(DealStatus::Draft);
        deal.timeout_overrides = Some(serde_json::json!({ "DRAFT": null }));
        deal.current_state_entered_at = OffsetDateTime::now_utc() - Duration::seconds(100);
        deal_repo
            .create(&DealAggregate {
                deal: deal.clone(),
                participations: vec![],
            })
            .await
            .unwrap();

        let config = DealTimeoutConfig::new(60, 120, 120, 200, 120, 120, 80, 200, 120);
        let use_case = ProcessDealTimeouts::new(deal_repo.clone(), milestone_repo, config);
        let result = use_case.execute(10).await.unwrap();

        assert_eq!(result.skipped, vec![deal.id]);
        assert!(result.transitioned.is_empty());
    }

    #[tokio::test]
    async fn deal_with_override_uses_override() {
        let deal_repo = Arc::new(FakeDealRepo::default());
        let milestone_repo = Arc::new(FakeMilestoneRepo::default());
        let mut deal = sample_deal(DealStatus::Draft);
        deal.timeout_overrides = Some(serde_json::json!({ "DRAFT": 200 }));
        deal.current_state_entered_at = OffsetDateTime::now_utc() - Duration::seconds(250);
        deal_repo
            .create(&DealAggregate {
                deal: deal.clone(),
                participations: vec![],
            })
            .await
            .unwrap();

        let config = DealTimeoutConfig::new(60, 120, 120, 200, 120, 120, 80, 200, 120);
        let use_case = ProcessDealTimeouts::new(deal_repo.clone(), milestone_repo, config);
        let result = use_case.execute(10).await.unwrap();

        assert_eq!(result.transitioned, vec![deal.id]);
    }

    #[tokio::test]
    async fn committed_deal_without_milestones_is_blocked() {
        let deal_repo = Arc::new(FakeDealRepo::default());
        let milestone_repo = Arc::new(FakeMilestoneRepo::default());
        let mut deal = sample_deal(DealStatus::Committed);
        deal.current_state_entered_at = OffsetDateTime::now_utc() - Duration::seconds(100);
        deal_repo
            .create(&DealAggregate {
                deal: deal.clone(),
                participations: vec![],
            })
            .await
            .unwrap();

        let config = DealTimeoutConfig::new(60, 120, 120, 200, 120, 120, 80, 200, 120);
        let use_case = ProcessDealTimeouts::new(deal_repo.clone(), milestone_repo, config);
        let result = use_case.execute(10).await.unwrap();

        assert_eq!(result.blocked, vec![deal.id]);
        let history = deal_repo.history.lock().unwrap();
        assert!(history
            .iter()
            .any(|(_, event, _, _)| event == "DEAL_TIMEOUT_BLOCKED"));
    }

    #[tokio::test]
    async fn re_running_after_transition_is_no_op() {
        let deal_repo = Arc::new(FakeDealRepo::default());
        let milestone_repo = Arc::new(FakeMilestoneRepo::default());
        let mut deal = sample_deal(DealStatus::Draft);
        deal.current_state_entered_at = OffsetDateTime::now_utc() - Duration::seconds(100);
        deal_repo
            .create(&DealAggregate {
                deal: deal.clone(),
                participations: vec![],
            })
            .await
            .unwrap();

        let config = DealTimeoutConfig::new(60, 120, 120, 200, 120, 120, 80, 200, 120);
        let use_case = ProcessDealTimeouts::new(deal_repo.clone(), milestone_repo, config);

        let first = use_case.execute(10).await.unwrap();
        assert_eq!(first.transitioned, vec![deal.id]);

        let second = use_case.execute(10).await.unwrap();
        assert!(second.transitioned.is_empty());
        assert!(second.skipped.is_empty());
    }
}
