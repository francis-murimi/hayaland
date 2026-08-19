use crate::deals::dto::{ExecuteTransitionCommand, ProposeTermCommand};
use crate::deals::{ExecuteTransition, ProposeTerm};
use crate::errors::ApplicationError;
use crate::milestones::dto::{CreateMilestoneCommand, MilestoneActionCommand};
use crate::milestones::{CompleteMilestone, CreateMilestone, StartMilestone, VerifyMilestone};
use crate::notifications::tests::fake_repos::{
    test_template, FakeDealRepo as NotificationFakeDealRepo, FakeEmailQueue,
    FakeNotificationPreferenceRepo, FakeNotificationPublisher, FakeNotificationRepo,
    FakeNotificationTemplateRepo, FakePartyRepo as NotificationFakePartyRepo, FakePushSender,
    FakeSmsSender, FakeUserRepo as NotificationFakeUserRepo,
};
use crate::notifications::LifecycleNotifier;
use crate::notifications::SendNotification;
use crate::parties::dto::CreatePartyCommand;
use crate::parties::CreateParty;
use crate::ports::NoOpTrustScoreRecalculation;
use crate::test_helpers::{
    FakeAgreementRepo, FakeDealRepo, FakeMilestoneRepo, FakePartyRepo, FakeReviewRepo,
    FakeTrustScoreRepo, FakeWalletRepo,
};
use domain::entities::{
    Agreement, Deal, DealParticipation, DealRole, DealStatus, DealTitle, DistributionModel,
    ParticipationStatus, PartyType, TermType, ValueDistribution,
};
use domain::repositories::{DealAggregate, DealRepository, MilestoneRepository};
use domain::services::ValidationConfig;
use rust_decimal::Decimal;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

fn actor_user_id() -> Uuid {
    Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap()
}

fn other_user_id() -> Uuid {
    Uuid::parse_str("99999999-9999-9999-9999-999999999999").unwrap()
}

async fn create_party(repo: &Arc<FakePartyRepo>, name: &str, roles: Vec<DealRole>) -> Uuid {
    CreateParty::new(repo.clone())
        .execute(CreatePartyCommand {
            actor_user_id: actor_user_id(),
            party_type: PartyType::Organization,
            display_name: name.to_string(),
            email: format!("{}@example.com", name.to_lowercase()),
            phone: None,
            tax_id: None,
            primary_domain_id: None,
            latitude: None,
            longitude: None,
            service_radius_km: None,
            roles,
        })
        .await
        .unwrap()
        .id
}

async fn three_party_fixture() -> (
    Arc<FakePartyRepo>,
    Arc<FakeDealRepo>,
    Uuid, // deal id
    Uuid, // supplier
    Uuid, // consumer
    Uuid, // enhancer
) {
    let party_repo = Arc::new(FakePartyRepo::default());
    let deal_repo = Arc::new(FakeDealRepo::default());

    let supplier = create_party(&party_repo, "Supplier", vec![DealRole::Supplier]).await;
    let consumer = create_party(&party_repo, "Consumer", vec![DealRole::Consumer]).await;
    let enhancer = create_party(&party_repo, "Enhancer", vec![DealRole::Enhancer]).await;

    let deal_id = Uuid::now_v7();
    let mut deal = Deal::new(
        deal_id,
        "DL-ET-0001".to_string(),
        DealTitle::new("Execute Transition Test Deal").unwrap(),
        Uuid::now_v7(),
        supplier,
        DealRole::Supplier,
    );
    deal.deal_status = DealStatus::PendingReview;

    deal_repo
        .create(&DealAggregate {
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

    let distribution = ValueDistribution {
        id: Uuid::now_v7(),
        deal_id,
        total_value: Decimal::from(10000),
        currency: "POINTS".to_string(),
        distribution_model: DistributionModel::FixedPrice,
        supplier_share_percentage: Decimal::from(60),
        supplier_share_amount: Decimal::from(6000),
        consumer_cost_percentage: Decimal::from(100),
        consumer_cost_amount: Decimal::from(10000),
        enhancer_share_percentage: Decimal::from(30),
        enhancer_share_amount: Decimal::from(3000),
        platform_fee_percentage: Decimal::from(10),
        platform_fee_amount: Decimal::from(1000),
        payment_schedule: vec![],
        win_win_win_score: None,
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
    };
    deal_repo
        .set_value_distribution(&distribution)
        .await
        .unwrap();

    (party_repo, deal_repo, deal_id, supplier, consumer, enhancer)
}

async fn accept_all_participations(deal_repo: &Arc<FakeDealRepo>, deal_id: Uuid) {
    let participations = deal_repo
        .find_participations_by_deal(deal_id)
        .await
        .unwrap();
    for mut p in participations {
        p.participation_status = ParticipationStatus::Accepted;
        p.responded_at = Some(OffsetDateTime::now_utc());
        deal_repo.update_participation(&p).await.unwrap();
    }
}

fn insert_unsigned_agreement(agreement_repo: &Arc<FakeAgreementRepo>, deal_id: Uuid) {
    let agreement = Agreement::new(
        Uuid::now_v7(),
        deal_id,
        "agreement text".to_string(),
        None,
        None,
        None,
        1,
    );
    agreement_repo
        .agreements
        .lock()
        .unwrap()
        .insert(deal_id, agreement);
}

fn transition_use_case(
    deal_repo: Arc<FakeDealRepo>,
    party_repo: Arc<FakePartyRepo>,
    agreement_repo: Arc<FakeAgreementRepo>,
) -> ExecuteTransition {
    ExecuteTransition::new(
        deal_repo,
        party_repo,
        agreement_repo,
        ValidationConfig::default(),
    )
}

fn transition_cmd(actor_party_id: Uuid, new_status: DealStatus) -> ExecuteTransitionCommand {
    ExecuteTransitionCommand {
        actor_user_id: actor_user_id(),
        actor_party_id,
        is_admin: false,
        new_status,
        reason: None,
        acknowledge_warnings: false,
    }
}

#[tokio::test]
async fn deal_not_found_returns_error() {
    let party_repo = Arc::new(FakePartyRepo::default());
    let deal_repo = Arc::new(FakeDealRepo::default());
    let agreement_repo = Arc::new(FakeAgreementRepo::default());

    let use_case = transition_use_case(deal_repo, party_repo, agreement_repo);
    let err = use_case
        .execute(
            Uuid::now_v7(),
            transition_cmd(Uuid::now_v7(), DealStatus::Negotiating),
        )
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::DealNotFound));
}

#[tokio::test]
async fn non_admin_non_member_is_forbidden() {
    let (party_repo, deal_repo, deal_id, supplier, _consumer, _enhancer) =
        three_party_fixture().await;
    let agreement_repo = Arc::new(FakeAgreementRepo::default());

    let use_case = transition_use_case(deal_repo, party_repo, agreement_repo);
    let mut cmd = transition_cmd(supplier, DealStatus::Negotiating);
    cmd.actor_user_id = other_user_id();

    let err = use_case.execute(deal_id, cmd).await.unwrap_err();
    assert!(matches!(err, ApplicationError::Forbidden));
}

#[tokio::test]
async fn admin_bypasses_membership_check() {
    let (party_repo, deal_repo, deal_id, supplier, _consumer, _enhancer) =
        three_party_fixture().await;
    let agreement_repo = Arc::new(FakeAgreementRepo::default());

    accept_all_participations(&deal_repo, deal_id).await;

    let use_case = transition_use_case(deal_repo, party_repo, agreement_repo);
    let mut cmd = transition_cmd(supplier, DealStatus::Negotiating);
    cmd.actor_user_id = other_user_id();
    cmd.is_admin = true;

    let result = use_case.execute(deal_id, cmd).await.unwrap();
    assert_eq!(result.deal_status, DealStatus::Negotiating);
}

#[tokio::test]
async fn non_participating_party_is_forbidden() {
    let (party_repo, deal_repo, deal_id, _supplier, _consumer, _enhancer) =
        three_party_fixture().await;
    let agreement_repo = Arc::new(FakeAgreementRepo::default());

    let outsider = create_party(&party_repo, "Outsider", vec![DealRole::Supplier]).await;

    let use_case = transition_use_case(deal_repo, party_repo, agreement_repo);
    let err = use_case
        .execute(deal_id, transition_cmd(outsider, DealStatus::Negotiating))
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Forbidden));
}

#[tokio::test]
async fn negotiating_from_non_pending_review_fails() {
    let (party_repo, deal_repo, deal_id, supplier, _consumer, _enhancer) =
        three_party_fixture().await;
    let agreement_repo = Arc::new(FakeAgreementRepo::default());

    let mut deal = deal_repo.find_by_id(deal_id).await.unwrap().unwrap();
    deal.deal_status = DealStatus::Negotiating;
    deal_repo.update(&deal).await.unwrap();

    let use_case = transition_use_case(deal_repo, party_repo, agreement_repo);
    let err = use_case
        .execute(deal_id, transition_cmd(supplier, DealStatus::Negotiating))
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        ApplicationError::InvalidStateTransition { .. }
    ));
}

#[tokio::test]
async fn negotiating_partial_acceptance_records_history() {
    let (party_repo, deal_repo, deal_id, supplier, _consumer, _enhancer) =
        three_party_fixture().await;
    let agreement_repo = Arc::new(FakeAgreementRepo::default());

    let use_case = transition_use_case(deal_repo.clone(), party_repo, agreement_repo);
    let result = use_case
        .execute(deal_id, transition_cmd(supplier, DealStatus::Negotiating))
        .await
        .unwrap();

    assert_eq!(result.deal_status, DealStatus::PendingReview);

    let history = deal_repo.history.lock().unwrap();
    assert!(history
        .iter()
        .any(|(_, event, _, _)| event == "PARTICIPATION_ACKNOWLEDGED"));
}

#[tokio::test]
async fn terms_locked_from_non_negotiating_fails() {
    let (party_repo, deal_repo, deal_id, supplier, _consumer, _enhancer) =
        three_party_fixture().await;
    let agreement_repo = Arc::new(FakeAgreementRepo::default());

    let use_case = transition_use_case(deal_repo, party_repo, agreement_repo);
    let err = use_case
        .execute(deal_id, transition_cmd(supplier, DealStatus::TermsLocked))
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        ApplicationError::InvalidStateTransition { .. }
    ));
}

#[tokio::test]
async fn terms_locked_requires_value_distribution() {
    let (party_repo, deal_repo, deal_id, supplier, _consumer, _enhancer) =
        three_party_fixture().await;
    let agreement_repo = Arc::new(FakeAgreementRepo::default());

    // Remove value distribution.
    deal_repo
        .value_distributions
        .lock()
        .unwrap()
        .remove(&deal_id);

    let mut deal = deal_repo.find_by_id(deal_id).await.unwrap().unwrap();
    deal.deal_status = DealStatus::Negotiating;
    deal_repo.update(&deal).await.unwrap();

    let use_case = transition_use_case(deal_repo, party_repo, agreement_repo);
    let err = use_case
        .execute(deal_id, transition_cmd(supplier, DealStatus::TermsLocked))
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        ApplicationError::WinWinWinValidationFailed { .. }
    ));
}

#[tokio::test]
async fn terms_locked_rejects_unaccepted_mandatory_term() {
    let (party_repo, deal_repo, deal_id, supplier, _consumer, _enhancer) =
        three_party_fixture().await;
    let agreement_repo = Arc::new(FakeAgreementRepo::default());

    let mut deal = deal_repo.find_by_id(deal_id).await.unwrap().unwrap();
    deal.deal_status = DealStatus::Negotiating;
    deal_repo.update(&deal).await.unwrap();

    ProposeTerm::new(deal_repo.clone(), party_repo.clone())
        .execute(ProposeTermCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            is_admin: false,
            deal_id,
            term_type: TermType::Price,
            term_name: "Price".to_string(),
            description: "100".to_string(),
            is_mandatory: true,
        })
        .await
        .unwrap();

    let use_case = transition_use_case(deal_repo, party_repo, agreement_repo);
    let err = use_case
        .execute(deal_id, transition_cmd(supplier, DealStatus::TermsLocked))
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        ApplicationError::WinWinWinValidationFailed { .. }
    ));
}

#[tokio::test]
async fn committed_from_non_terms_locked_fails() {
    let (party_repo, deal_repo, deal_id, supplier, _consumer, _enhancer) =
        three_party_fixture().await;
    let agreement_repo = Arc::new(FakeAgreementRepo::default());

    let mut deal = deal_repo.find_by_id(deal_id).await.unwrap().unwrap();
    deal.deal_status = DealStatus::Negotiating;
    deal_repo.update(&deal).await.unwrap();

    let use_case = transition_use_case(deal_repo, party_repo, agreement_repo);
    let err = use_case
        .execute(deal_id, transition_cmd(supplier, DealStatus::Committed))
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        ApplicationError::InvalidStateTransition { .. }
    ));
}

#[tokio::test]
async fn committed_requires_value_distribution() {
    let (party_repo, deal_repo, deal_id, supplier, _consumer, _enhancer) =
        three_party_fixture().await;
    let agreement_repo = Arc::new(FakeAgreementRepo::default());

    let mut deal = deal_repo.find_by_id(deal_id).await.unwrap().unwrap();
    deal.deal_status = DealStatus::TermsLocked;
    deal_repo.update(&deal).await.unwrap();
    deal_repo
        .value_distributions
        .lock()
        .unwrap()
        .remove(&deal_id);

    let use_case = transition_use_case(deal_repo, party_repo, agreement_repo);
    let err = use_case
        .execute(deal_id, transition_cmd(supplier, DealStatus::Committed))
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        ApplicationError::WinWinWinValidationFailed { .. }
    ));
}

#[tokio::test]
async fn committed_requires_signed_agreement() {
    let (party_repo, deal_repo, deal_id, supplier, _consumer, _enhancer) =
        three_party_fixture().await;
    let agreement_repo = Arc::new(FakeAgreementRepo::default());

    let mut deal = deal_repo.find_by_id(deal_id).await.unwrap().unwrap();
    deal.deal_status = DealStatus::TermsLocked;
    deal_repo.update(&deal).await.unwrap();

    let use_case = transition_use_case(deal_repo, party_repo, agreement_repo);
    let err = use_case
        .execute(deal_id, transition_cmd(supplier, DealStatus::Committed))
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Validation(_)));
}

#[tokio::test]
async fn committed_rejects_unsigned_agreement() {
    let (party_repo, deal_repo, deal_id, supplier, _consumer, _enhancer) =
        three_party_fixture().await;
    let agreement_repo = Arc::new(FakeAgreementRepo::default());

    insert_unsigned_agreement(&agreement_repo, deal_id);

    let mut deal = deal_repo.find_by_id(deal_id).await.unwrap().unwrap();
    deal.deal_status = DealStatus::TermsLocked;
    deal_repo.update(&deal).await.unwrap();

    let wallet_repo = Arc::new(FakeWalletRepo::default());

    let use_case = transition_use_case(deal_repo, party_repo, agreement_repo)
        .with_wallet_repository(wallet_repo);
    let err = use_case
        .execute(deal_id, transition_cmd(supplier, DealStatus::Committed))
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Validation(_)));
}

#[tokio::test]
async fn executing_requires_milestones() {
    let (party_repo, deal_repo, deal_id, supplier, _consumer, _enhancer) =
        three_party_fixture().await;
    let agreement_repo = Arc::new(FakeAgreementRepo::default());

    accept_all_participations(&deal_repo, deal_id).await;
    let mut deal = deal_repo.find_by_id(deal_id).await.unwrap().unwrap();
    deal.deal_status = DealStatus::Committed;
    deal_repo.update(&deal).await.unwrap();

    let milestone_repo = Arc::new(FakeMilestoneRepo::default());
    let use_case = ExecuteTransition::new_with_milestones(
        deal_repo,
        party_repo,
        agreement_repo,
        milestone_repo,
        ValidationConfig::default(),
    );

    let err = use_case
        .execute(deal_id, transition_cmd(supplier, DealStatus::Executing))
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Validation(_)));
}

#[tokio::test]
async fn executing_without_milestone_repo_errors() {
    let (party_repo, deal_repo, deal_id, supplier, _consumer, _enhancer) =
        three_party_fixture().await;
    let agreement_repo = Arc::new(FakeAgreementRepo::default());

    accept_all_participations(&deal_repo, deal_id).await;
    let mut deal = deal_repo.find_by_id(deal_id).await.unwrap().unwrap();
    deal.deal_status = DealStatus::Committed;
    deal_repo.update(&deal).await.unwrap();

    let use_case = transition_use_case(deal_repo, party_repo, agreement_repo);
    let err = use_case
        .execute(deal_id, transition_cmd(supplier, DealStatus::Executing))
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Validation(_)));
}

#[tokio::test]
async fn completed_requires_all_milestones_verified() {
    let (party_repo, deal_repo, deal_id, supplier, consumer, _enhancer) =
        three_party_fixture().await;
    let agreement_repo = Arc::new(FakeAgreementRepo::default());

    accept_all_participations(&deal_repo, deal_id).await;
    let mut deal = deal_repo.find_by_id(deal_id).await.unwrap().unwrap();
    deal.deal_status = DealStatus::Executing;
    deal_repo.update(&deal).await.unwrap();

    let milestone_repo = Arc::new(FakeMilestoneRepo::default());
    CreateMilestone::new(
        party_repo.clone(),
        deal_repo.clone(),
        milestone_repo.clone(),
    )
    .execute(CreateMilestoneCommand {
        actor_user_id: actor_user_id(),
        actor_party_id: supplier,
        is_admin: false,
        deal_id,
        milestone_name: "Milestone One".to_string(),
        description: None,
        assigned_to_party_id: supplier,
        verified_by_party_id: consumer,
        due_date: None,
        completion_criteria: "done".to_string(),
        payment_trigger_amount: None,
        display_order: 1,
    })
    .await
    .unwrap();

    let use_case = ExecuteTransition::new_with_milestones(
        deal_repo,
        party_repo,
        agreement_repo,
        milestone_repo,
        ValidationConfig::default(),
    );

    let err = use_case
        .execute(deal_id, transition_cmd(supplier, DealStatus::Completed))
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Validation(_)));
}

#[tokio::test]
async fn completed_requires_reviews() {
    let (party_repo, deal_repo, deal_id, supplier, consumer, _enhancer) =
        three_party_fixture().await;
    let agreement_repo = Arc::new(FakeAgreementRepo::default());

    accept_all_participations(&deal_repo, deal_id).await;
    let mut deal = deal_repo.find_by_id(deal_id).await.unwrap().unwrap();
    deal.deal_status = DealStatus::Executing;
    deal_repo.update(&deal).await.unwrap();

    let milestone_repo = Arc::new(FakeMilestoneRepo::default());
    CreateMilestone::new(
        party_repo.clone(),
        deal_repo.clone(),
        milestone_repo.clone(),
    )
    .execute(CreateMilestoneCommand {
        actor_user_id: actor_user_id(),
        actor_party_id: supplier,
        is_admin: false,
        deal_id,
        milestone_name: "Milestone One".to_string(),
        description: None,
        assigned_to_party_id: supplier,
        verified_by_party_id: consumer,
        due_date: None,
        completion_criteria: "done".to_string(),
        payment_trigger_amount: None,
        display_order: 1,
    })
    .await
    .unwrap();

    StartMilestone::new(
        party_repo.clone(),
        deal_repo.clone(),
        milestone_repo.clone(),
    )
    .execute(MilestoneActionCommand {
        actor_user_id: actor_user_id(),
        actor_party_id: supplier,
        is_admin: false,
        milestone_id: {
            let list = milestone_repo
                .find_by_deal(deal_id, i64::MAX, 0)
                .await
                .unwrap();
            list[0].id
        },
        comment: None,
    })
    .await
    .unwrap();

    CompleteMilestone::new(
        party_repo.clone(),
        deal_repo.clone(),
        milestone_repo.clone(),
    )
    .execute(MilestoneActionCommand {
        actor_user_id: actor_user_id(),
        actor_party_id: supplier,
        is_admin: false,
        milestone_id: {
            let list = milestone_repo
                .find_by_deal(deal_id, i64::MAX, 0)
                .await
                .unwrap();
            list[0].id
        },
        comment: None,
    })
    .await
    .unwrap();

    VerifyMilestone::new(
        party_repo.clone(),
        deal_repo.clone(),
        milestone_repo.clone(),
        Arc::new(FakeWalletRepo::default()),
    )
    .execute(MilestoneActionCommand {
        actor_user_id: actor_user_id(),
        actor_party_id: consumer,
        is_admin: false,
        milestone_id: {
            let list = milestone_repo
                .find_by_deal(deal_id, i64::MAX, 0)
                .await
                .unwrap();
            list[0].id
        },
        comment: None,
    })
    .await
    .unwrap();

    let review_repo = Arc::new(FakeReviewRepo::default());
    let use_case = ExecuteTransition::new_with_reviews(
        deal_repo,
        party_repo,
        agreement_repo,
        milestone_repo,
        review_repo,
        ValidationConfig::default(),
    );

    let err = use_case
        .execute(deal_id, transition_cmd(supplier, DealStatus::Completed))
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Validation(_)));
}

#[tokio::test]
async fn completed_without_review_repo_errors() {
    let (party_repo, deal_repo, deal_id, supplier, consumer, _enhancer) =
        three_party_fixture().await;
    let agreement_repo = Arc::new(FakeAgreementRepo::default());

    accept_all_participations(&deal_repo, deal_id).await;
    let mut deal = deal_repo.find_by_id(deal_id).await.unwrap().unwrap();
    deal.deal_status = DealStatus::Executing;
    deal_repo.update(&deal).await.unwrap();

    let milestone_repo = Arc::new(FakeMilestoneRepo::default());
    let wallet_repo = Arc::new(FakeWalletRepo::default());
    CreateMilestone::new(
        party_repo.clone(),
        deal_repo.clone(),
        milestone_repo.clone(),
    )
    .execute(CreateMilestoneCommand {
        actor_user_id: actor_user_id(),
        actor_party_id: supplier,
        is_admin: false,
        deal_id,
        milestone_name: "Milestone One".to_string(),
        description: None,
        assigned_to_party_id: supplier,
        verified_by_party_id: consumer,
        due_date: None,
        completion_criteria: "done".to_string(),
        payment_trigger_amount: None,
        display_order: 1,
    })
    .await
    .unwrap();

    StartMilestone::new(
        party_repo.clone(),
        deal_repo.clone(),
        milestone_repo.clone(),
    )
    .execute(MilestoneActionCommand {
        actor_user_id: actor_user_id(),
        actor_party_id: supplier,
        is_admin: false,
        milestone_id: {
            let list = milestone_repo
                .find_by_deal(deal_id, i64::MAX, 0)
                .await
                .unwrap();
            list[0].id
        },
        comment: None,
    })
    .await
    .unwrap();

    CompleteMilestone::new(
        party_repo.clone(),
        deal_repo.clone(),
        milestone_repo.clone(),
    )
    .execute(MilestoneActionCommand {
        actor_user_id: actor_user_id(),
        actor_party_id: supplier,
        is_admin: false,
        milestone_id: {
            let list = milestone_repo
                .find_by_deal(deal_id, i64::MAX, 0)
                .await
                .unwrap();
            list[0].id
        },
        comment: None,
    })
    .await
    .unwrap();

    VerifyMilestone::new(
        party_repo.clone(),
        deal_repo.clone(),
        milestone_repo.clone(),
        wallet_repo,
    )
    .execute(MilestoneActionCommand {
        actor_user_id: actor_user_id(),
        actor_party_id: consumer,
        is_admin: false,
        milestone_id: {
            let list = milestone_repo
                .find_by_deal(deal_id, i64::MAX, 0)
                .await
                .unwrap();
            list[0].id
        },
        comment: None,
    })
    .await
    .unwrap();

    let use_case = ExecuteTransition::new_with_milestones(
        deal_repo,
        party_repo,
        agreement_repo,
        milestone_repo,
        ValidationConfig::default(),
    );

    let err = use_case
        .execute(deal_id, transition_cmd(supplier, DealStatus::Completed))
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Infrastructure(_)));
}

#[tokio::test]
async fn unsupported_transition_is_rejected() {
    let (party_repo, deal_repo, deal_id, supplier, _consumer, _enhancer) =
        three_party_fixture().await;
    let agreement_repo = Arc::new(FakeAgreementRepo::default());

    let use_case = transition_use_case(deal_repo, party_repo, agreement_repo);
    let err = use_case
        .execute(deal_id, transition_cmd(supplier, DealStatus::Suggested))
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Validation(_)));
}

#[tokio::test]
async fn cancellation_from_terminal_state_fails() {
    let (party_repo, deal_repo, deal_id, supplier, _consumer, _enhancer) =
        three_party_fixture().await;
    let agreement_repo = Arc::new(FakeAgreementRepo::default());

    let mut deal = deal_repo.find_by_id(deal_id).await.unwrap().unwrap();
    deal.deal_status = DealStatus::Completed;
    deal_repo.update(&deal).await.unwrap();

    let use_case = transition_use_case(deal_repo, party_repo, agreement_repo);
    let err = use_case
        .execute(deal_id, transition_cmd(supplier, DealStatus::Cancelled))
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        ApplicationError::InvalidStateTransition { .. }
    ));
}

#[tokio::test]
async fn cancellation_updates_trust_scores_and_requests_recalculation() {
    let (party_repo, deal_repo, deal_id, supplier, consumer, enhancer) =
        three_party_fixture().await;
    let agreement_repo = Arc::new(FakeAgreementRepo::default());

    let mut deal = deal_repo.find_by_id(deal_id).await.unwrap().unwrap();
    deal.deal_status = DealStatus::Negotiating;
    deal_repo.update(&deal).await.unwrap();

    let trust_repo = Arc::new(FakeTrustScoreRepo::default());
    let recalc: Arc<dyn crate::ports::TrustScoreRecalculationPort> =
        Arc::new(NoOpTrustScoreRecalculation);

    let use_case = transition_use_case(deal_repo.clone(), party_repo, agreement_repo)
        .with_trust_score_repository(trust_repo.clone())
        .with_trust_score_recalculation_port(recalc);

    let result = use_case
        .execute(deal_id, transition_cmd(supplier, DealStatus::Cancelled))
        .await
        .unwrap();

    assert_eq!(result.deal_status, DealStatus::Cancelled);

    let rows = trust_repo.rows.lock().unwrap();
    assert_eq!(rows.get(&supplier).unwrap().deals_cancelled_count, 1);
    assert_eq!(rows.get(&consumer).unwrap().deals_cancelled_count, 1);
    assert_eq!(rows.get(&enhancer).unwrap().deals_cancelled_count, 1);
}

#[tokio::test]
async fn transition_emits_notification_via_notifier() {
    let (party_repo, deal_repo, deal_id, supplier, _consumer, _enhancer) =
        three_party_fixture().await;
    let agreement_repo = Arc::new(FakeAgreementRepo::default());

    accept_all_participations(&deal_repo, deal_id).await;

    let prefs = Arc::new(FakeNotificationPreferenceRepo::new());
    let template_repo = Arc::new(FakeNotificationTemplateRepo::new());
    template_repo.with(test_template(
        Uuid::now_v7(),
        "deal_terms_locked_in_app",
        domain::entities::NotificationType::DealTermsLocked,
        domain::entities::NotificationChannel::InApp,
        "en",
        "",
        "Terms locked",
    ));

    let send = Arc::new(SendNotification::new(
        Arc::new(FakeNotificationRepo::new()),
        prefs,
        template_repo,
        Arc::new(NotificationFakeUserRepo::new()),
        Arc::new(NotificationFakePartyRepo::new()),
        Arc::new(NotificationFakeDealRepo::new()),
        Arc::new(FakeEmailQueue::new()),
        Arc::new(FakeNotificationPublisher::new()),
        Arc::new(FakePushSender),
        Arc::new(FakeSmsSender),
        "en".to_string(),
    ));
    let notifier = Arc::new(LifecycleNotifier::new(send));

    let use_case =
        transition_use_case(deal_repo.clone(), party_repo, agreement_repo).with_notifier(notifier);

    let mut deal = deal_repo.find_by_id(deal_id).await.unwrap().unwrap();
    deal.deal_status = DealStatus::Negotiating;
    deal_repo.update(&deal).await.unwrap();

    let result = use_case
        .execute(deal_id, transition_cmd(supplier, DealStatus::TermsLocked))
        .await
        .unwrap();

    assert_eq!(result.deal_status, DealStatus::TermsLocked);
}
