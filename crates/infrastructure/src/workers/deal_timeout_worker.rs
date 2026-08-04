use application::deals::{ProcessDealTimeouts, ProcessDealTimeoutsResult};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval, MissedTickBehavior};
use tracing::{info, warn};

/// Run the deal timeout worker loop.
pub async fn run_deal_timeout_worker(
    process_timeouts: Arc<ProcessDealTimeouts>,
    interval_duration: Duration,
    batch_size: usize,
) {
    let mut ticker = interval(interval_duration);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        ticker.tick().await;
        run_tick(&process_timeouts, batch_size).await;
    }
}

async fn run_tick(process_timeouts: &ProcessDealTimeouts, batch_size: usize) {
    match process_timeouts.execute(batch_size).await {
        Ok(ProcessDealTimeoutsResult {
            transitioned,
            blocked,
            skipped,
            errors,
        }) => {
            let candidates = transitioned.len() + blocked.len() + skipped.len() + errors.len();
            info!(
                candidates = candidates,
                transitioned = transitioned.len(),
                blocked = blocked.len(),
                skipped = skipped.len(),
                errors = errors.len(),
                "deal_timeout_worker_tick_complete"
            );
        }
        Err(err) => {
            warn!(error = %err, "deal_timeout_worker_tick_failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use application::test_helpers::{FakeDealRepo, FakeMilestoneRepo};
    use domain::entities::{Deal, DealParticipation, DealRole, DealStatus, DealTitle};
    use domain::repositories::DealRepository;
    use time::OffsetDateTime;
    use uuid::Uuid;

    fn deal_timeout_config() -> application::deals::timeout_config::DealTimeoutConfig {
        application::deals::timeout_config::DealTimeoutConfig {
            draft_seconds: 1,
            suggested_seconds: 1,
            pending_review_seconds: 1,
            negotiating_seconds: 1,
            awaiting_party_seconds: 1,
            terms_locked_seconds: 1,
            committed_seconds: 1,
            on_hold_seconds: 1,
            disputed_seconds: 1,
        }
    }

    async fn seed_expired_deal(repo: &Arc<FakeDealRepo>) -> Uuid {
        let deal_id = Uuid::now_v7();
        let supplier = Uuid::now_v7();
        let consumer = Uuid::now_v7();
        let enhancer = Uuid::now_v7();
        let category_id = Uuid::now_v7();
        let mut deal = Deal::new(
            deal_id,
            "DL-TIMEOUT".to_string(),
            DealTitle::new("Timeout Deal").unwrap(),
            category_id,
            supplier,
            DealRole::Supplier,
        );
        deal.deal_status = DealStatus::Draft;
        deal.current_state_entered_at = OffsetDateTime::now_utc() - time::Duration::seconds(10);

        repo.create(&domain::repositories::DealAggregate {
            deal,
            participations: vec![
                DealParticipation::new(Uuid::now_v7(), deal_id, supplier, DealRole::Supplier, true),
                DealParticipation::new(
                    Uuid::now_v7(),
                    deal_id,
                    consumer,
                    DealRole::Consumer,
                    false,
                ),
                DealParticipation::new(
                    Uuid::now_v7(),
                    deal_id,
                    enhancer,
                    DealRole::Enhancer,
                    false,
                ),
            ],
        })
        .await
        .unwrap();
        deal_id
    }

    #[tokio::test]
    async fn tick_transitions_expired_deal() {
        let deal_repo = Arc::new(FakeDealRepo::default());
        let milestone_repo = Arc::new(FakeMilestoneRepo::default());
        let use_case =
            ProcessDealTimeouts::new(deal_repo.clone(), milestone_repo, deal_timeout_config());

        let deal_id = seed_expired_deal(&deal_repo).await;

        run_tick(&use_case, 10).await;

        let deal = deal_repo.find_by_id(deal_id).await.unwrap().unwrap();
        assert_eq!(deal.deal_status, DealStatus::Expired);
    }

    #[tokio::test]
    async fn tick_handles_empty_batch() {
        let deal_repo = Arc::new(FakeDealRepo::default());
        let milestone_repo = Arc::new(FakeMilestoneRepo::default());
        let use_case = ProcessDealTimeouts::new(deal_repo, milestone_repo, deal_timeout_config());

        run_tick(&use_case, 10).await;
    }
}
