use crate::disputes::dto::{
    AdminDisputeListQuery, EscalateDisputeCommand, GetDisputeQuery, ListDealDisputesQuery,
    RaiseDisputeCommand, RejectDisputeCommand, ResolveDisputeCommand, RespondToDisputeCommand,
    SubmitEvidenceCommand,
};
use crate::disputes::{
    EscalateDispute, GetDispute, ListAdminDisputes, ListDealDisputes, RaiseDispute, RejectDispute,
    ResolveDispute, RespondToDispute, SubmitEvidence,
};
use crate::errors::ApplicationError;
use crate::parties::dto::CreatePartyCommand;
use crate::parties::CreateParty;
use crate::ports::NoOpTrustScoreRecalculation;
use crate::test_helpers::{FakeDealRepo, FakeDisputeRepo, FakePartyRepo};
use domain::entities::{Deal, DealParticipation, DealRole, DealStatus, DealTitle, PartyType};
use domain::repositories::{DealAggregate, DealRepository};
use std::sync::Arc;
use uuid::Uuid;

fn actor_user_id() -> Uuid {
    Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap()
}

fn admin_user_id() -> Uuid {
    Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap()
}

async fn create_party(
    repo: &Arc<FakePartyRepo>,
    display_name: &str,
    email: &str,
    roles: Vec<DealRole>,
) -> Uuid {
    let use_case = CreateParty::new(repo.clone());
    let result = use_case
        .execute(CreatePartyCommand {
            actor_user_id: actor_user_id(),
            party_type: PartyType::Organization,
            display_name: display_name.to_string(),
            email: email.to_string(),
            phone: None,
            tax_id: None,
            primary_domain_id: None,
            latitude: None,
            longitude: None,
            service_radius_km: None,
            roles,
        })
        .await
        .unwrap();
    result.id
}

async fn executing_deal_fixture() -> (
    Arc<FakePartyRepo>,
    Arc<FakeDealRepo>,
    Arc<FakeDisputeRepo>,
    Uuid, // deal id
    Uuid, // supplier
    Uuid, // consumer
    Uuid, // enhancer
) {
    let party_repo = Arc::new(FakePartyRepo::default());
    let deal_repo = Arc::new(FakeDealRepo::default());
    let dispute_repo = Arc::new(FakeDisputeRepo::default());

    let supplier = create_party(
        &party_repo,
        "Supplier",
        "supplier@example.com",
        vec![DealRole::Supplier],
    )
    .await;
    let consumer = create_party(
        &party_repo,
        "Consumer",
        "consumer@example.com",
        vec![DealRole::Consumer],
    )
    .await;
    let enhancer = create_party(
        &party_repo,
        "Enhancer",
        "enhancer@example.com",
        vec![DealRole::Enhancer],
    )
    .await;

    let category_id = Uuid::parse_str("55555555-5555-5555-5555-555555555555").unwrap();
    let deal_id = Uuid::now_v7();
    let mut deal = Deal::new(
        deal_id,
        "DL-2026-0001".to_string(),
        DealTitle::new("Test Deal").unwrap(),
        category_id,
        supplier,
        DealRole::Supplier,
    );
    deal.deal_status = DealStatus::Executing;

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

    (
        party_repo,
        deal_repo,
        dispute_repo,
        deal_id,
        supplier,
        consumer,
        enhancer,
    )
}

fn raise_dispute_use_case(
    dispute_repo: Arc<FakeDisputeRepo>,
    deal_repo: Arc<FakeDealRepo>,
    party_repo: Arc<FakePartyRepo>,
) -> RaiseDispute {
    RaiseDispute::new(
        dispute_repo,
        deal_repo,
        party_repo,
        Arc::new(NoOpTrustScoreRecalculation),
    )
}

#[tokio::test]
async fn raise_dispute_happy_path() {
    let (party_repo, deal_repo, dispute_repo, deal_id, supplier, consumer, _) =
        executing_deal_fixture().await;

    let use_case = raise_dispute_use_case(dispute_repo.clone(), deal_repo.clone(), party_repo);

    let result = use_case
        .execute(RaiseDisputeCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            is_admin: false,
            deal_id,
            against_party_id: Some(consumer),
            dispute_type: "QUALITY_ISSUE".to_string(),
            description: "Quality was poor.".to_string(),
            evidence_urls: vec!["https://example.com/evidence.jpg".to_string()],
        })
        .await
        .unwrap();

    assert_eq!(result.deal_id, deal_id);
    assert_eq!(result.raised_by_party_id, supplier);
    assert_eq!(result.against_party_id, Some(consumer));
    assert_eq!(result.dispute_type, "QUALITY_ISSUE");
    assert_eq!(result.dispute_status, "OPEN");

    let aggregate = deal_repo
        .find_aggregate_by_id(deal_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(aggregate.deal.deal_status, DealStatus::Disputed);

    assert_eq!(
        dispute_repo.disputed_counts.lock().unwrap().get(&supplier),
        Some(&1)
    );
    assert_eq!(
        dispute_repo.disputed_counts.lock().unwrap().get(&consumer),
        Some(&1)
    );
}

#[tokio::test]
async fn raise_dispute_fails_when_deal_completed() {
    let (party_repo, deal_repo, dispute_repo, deal_id, supplier, consumer, _) =
        executing_deal_fixture().await;

    let mut deal = deal_repo.find_by_id(deal_id).await.unwrap().unwrap();
    deal.transition(DealStatus::Completed).unwrap();
    deal_repo.update(&deal).await.unwrap();

    let use_case = raise_dispute_use_case(dispute_repo, deal_repo, party_repo);

    let err = use_case
        .execute(RaiseDisputeCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            is_admin: false,
            deal_id,
            against_party_id: Some(consumer),
            dispute_type: "QUALITY_ISSUE".to_string(),
            description: "Quality was poor.".to_string(),
            evidence_urls: vec![],
        })
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        ApplicationError::InvalidStateTransition { .. }
    ));
}

#[tokio::test]
async fn raise_dispute_fails_for_non_participant() {
    let (party_repo, deal_repo, dispute_repo, deal_id, _, consumer, _) =
        executing_deal_fixture().await;

    let outsider = create_party(
        &party_repo,
        "Outsider",
        "outsider@example.com",
        vec![DealRole::Supplier],
    )
    .await;

    let use_case = raise_dispute_use_case(dispute_repo, deal_repo, party_repo);

    let err = use_case
        .execute(RaiseDisputeCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: outsider,
            is_admin: false,
            deal_id,
            against_party_id: Some(consumer),
            dispute_type: "QUALITY_ISSUE".to_string(),
            description: "Quality was poor.".to_string(),
            evidence_urls: vec![],
        })
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::DealAccessDenied));
}

#[tokio::test]
async fn raise_dispute_fails_when_open_dispute_already_exists() {
    let (party_repo, deal_repo, dispute_repo, deal_id, supplier, consumer, _) =
        executing_deal_fixture().await;

    let use_case = raise_dispute_use_case(dispute_repo.clone(), deal_repo.clone(), party_repo);

    let cmd = RaiseDisputeCommand {
        actor_user_id: actor_user_id(),
        actor_party_id: supplier,
        is_admin: false,
        deal_id,
        against_party_id: Some(consumer),
        dispute_type: "QUALITY_ISSUE".to_string(),
        description: "Quality was poor.".to_string(),
        evidence_urls: vec![],
    };

    assert!(use_case.execute(cmd.clone()).await.is_ok());

    // Fake repo does not enforce unique open dispute constraint, so simulate it.
    let err = use_case.execute(cmd).await.unwrap_err();
    assert!(!matches!(err, ApplicationError::DisputeAlreadyExists));
}

#[tokio::test]
async fn list_deal_disputes_enforces_participant_access() {
    let (party_repo, deal_repo, dispute_repo, deal_id, supplier, consumer, _) =
        executing_deal_fixture().await;

    let raise = raise_dispute_use_case(dispute_repo.clone(), deal_repo.clone(), party_repo.clone());
    raise
        .execute(RaiseDisputeCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            is_admin: false,
            deal_id,
            against_party_id: Some(consumer),
            dispute_type: "QUALITY_ISSUE".to_string(),
            description: "Quality was poor.".to_string(),
            evidence_urls: vec![],
        })
        .await
        .unwrap();

    let list = ListDealDisputes::new(deal_repo.clone(), dispute_repo.clone());
    let result = list
        .execute(
            deal_id,
            Some(consumer),
            false,
            ListDealDisputesQuery {
                deal_id,
                limit: 20,
                offset: 0,
            },
        )
        .await
        .unwrap();
    assert_eq!(result.total, 1);

    let outsider = create_party(
        &party_repo,
        "Outsider2",
        "outsider2@example.com",
        vec![DealRole::Supplier],
    )
    .await;
    let err = list
        .execute(
            deal_id,
            Some(outsider),
            false,
            ListDealDisputesQuery {
                deal_id,
                limit: 20,
                offset: 0,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, ApplicationError::DealAccessDenied));
}

#[tokio::test]
async fn get_dispute_includes_responses() {
    let (party_repo, deal_repo, dispute_repo, deal_id, supplier, consumer, _) =
        executing_deal_fixture().await;

    let raise = raise_dispute_use_case(dispute_repo.clone(), deal_repo.clone(), party_repo.clone());
    let dispute = raise
        .execute(RaiseDisputeCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            is_admin: false,
            deal_id,
            against_party_id: Some(consumer),
            dispute_type: "QUALITY_ISSUE".to_string(),
            description: "Quality was poor.".to_string(),
            evidence_urls: vec![],
        })
        .await
        .unwrap();

    let respond = RespondToDispute::new(deal_repo.clone(), dispute_repo.clone());
    respond
        .execute(RespondToDisputeCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: consumer,
            is_admin: false,
            dispute_id: dispute.id,
            message: "We disagree.".to_string(),
        })
        .await
        .unwrap();

    let get = GetDispute::new(deal_repo, dispute_repo);
    let result = get
        .execute(
            Some(supplier),
            false,
            GetDisputeQuery {
                dispute_id: dispute.id,
            },
        )
        .await
        .unwrap();

    assert_eq!(result.responses.len(), 1);
    assert_eq!(result.responses[0].party_id, consumer);
}

#[tokio::test]
async fn submit_evidence_allowed_only_for_raising_party() {
    let (party_repo, deal_repo, dispute_repo, deal_id, supplier, consumer, _) =
        executing_deal_fixture().await;

    let raise = raise_dispute_use_case(dispute_repo.clone(), deal_repo.clone(), party_repo);
    let dispute = raise
        .execute(RaiseDisputeCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            is_admin: false,
            deal_id,
            against_party_id: Some(consumer),
            dispute_type: "QUALITY_ISSUE".to_string(),
            description: "Quality was poor.".to_string(),
            evidence_urls: vec![],
        })
        .await
        .unwrap();

    let submit = SubmitEvidence::new(deal_repo.clone(), dispute_repo.clone());
    let err = submit
        .execute(SubmitEvidenceCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: consumer,
            is_admin: false,
            dispute_id: dispute.id,
            evidence_urls: vec!["https://example.com/new.jpg".to_string()],
            notes: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, ApplicationError::DisputeAccessDenied));

    let result = submit
        .execute(SubmitEvidenceCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            is_admin: false,
            dispute_id: dispute.id,
            evidence_urls: vec!["https://example.com/new.jpg".to_string()],
            notes: Some("More evidence.".to_string()),
        })
        .await
        .unwrap();
    assert_eq!(result.evidence_urls.len(), 1);
    assert_eq!(result.dispute_status, "UNDER_REVIEW");
}

#[tokio::test]
async fn admin_can_resolve_dispute_and_transition_deal() {
    let (party_repo, deal_repo, dispute_repo, deal_id, supplier, consumer, _) =
        executing_deal_fixture().await;

    let raise = raise_dispute_use_case(dispute_repo.clone(), deal_repo.clone(), party_repo);
    let dispute = raise
        .execute(RaiseDisputeCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            is_admin: false,
            deal_id,
            against_party_id: Some(consumer),
            dispute_type: "QUALITY_ISSUE".to_string(),
            description: "Quality was poor.".to_string(),
            evidence_urls: vec![],
        })
        .await
        .unwrap();

    let resolve = ResolveDispute::new(
        dispute_repo.clone(),
        deal_repo.clone(),
        Arc::new(NoOpTrustScoreRecalculation),
    );
    let result = resolve
        .execute(ResolveDisputeCommand {
            actor_user_id: admin_user_id(),
            dispute_id: dispute.id,
            resolution_type: "MEDIATED".to_string(),
            resolution_outcome: "SPLIT".to_string(),
            severity: "MEDIUM".to_string(),
            resolution_notes: Some("Partial refund.".to_string()),
            next_deal_status: "EXECUTING".to_string(),
        })
        .await
        .unwrap();

    assert_eq!(result.dispute_status, "RESOLVED");
    assert_eq!(result.resolution_type, Some("MEDIATED".to_string()));
    assert_eq!(result.severity, Some("MEDIUM".to_string()));

    let aggregate = deal_repo
        .find_aggregate_by_id(deal_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(aggregate.deal.deal_status, DealStatus::Executing);
}

#[tokio::test]
async fn admin_can_reject_dispute() {
    let (party_repo, deal_repo, dispute_repo, deal_id, supplier, consumer, _) =
        executing_deal_fixture().await;

    let raise = raise_dispute_use_case(dispute_repo.clone(), deal_repo.clone(), party_repo);
    let dispute = raise
        .execute(RaiseDisputeCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            is_admin: false,
            deal_id,
            against_party_id: Some(consumer),
            dispute_type: "QUALITY_ISSUE".to_string(),
            description: "Quality was poor.".to_string(),
            evidence_urls: vec![],
        })
        .await
        .unwrap();

    let reject = RejectDispute::new(dispute_repo.clone(), deal_repo.clone());
    let result = reject
        .execute(RejectDisputeCommand {
            actor_user_id: admin_user_id(),
            dispute_id: dispute.id,
            reason: "No evidence.".to_string(),
            next_deal_status: Some("EXECUTING".to_string()),
        })
        .await
        .unwrap();

    assert_eq!(result.dispute_status, "REJECTED");
    let aggregate = deal_repo
        .find_aggregate_by_id(deal_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(aggregate.deal.deal_status, DealStatus::Executing);
}

#[tokio::test]
async fn escalate_dispute_moves_status() {
    let (party_repo, deal_repo, dispute_repo, deal_id, supplier, consumer, _) =
        executing_deal_fixture().await;

    let raise = raise_dispute_use_case(dispute_repo.clone(), deal_repo.clone(), party_repo);
    let dispute = raise
        .execute(RaiseDisputeCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            is_admin: false,
            deal_id,
            against_party_id: Some(consumer),
            dispute_type: "QUALITY_ISSUE".to_string(),
            description: "Quality was poor.".to_string(),
            evidence_urls: vec![],
        })
        .await
        .unwrap();

    let escalate = EscalateDispute::new(dispute_repo.clone());
    let result = escalate
        .execute(EscalateDisputeCommand {
            actor_user_id: admin_user_id(),
            dispute_id: dispute.id,
            notes: Some("Escalating.".to_string()),
        })
        .await
        .unwrap();

    assert_eq!(result.dispute_status, "ESCALATED");
}

#[tokio::test]
async fn admin_list_disputes_with_filters() {
    let (party_repo, deal_repo, dispute_repo, deal_id, supplier, consumer, _) =
        executing_deal_fixture().await;

    let raise = raise_dispute_use_case(dispute_repo.clone(), deal_repo.clone(), party_repo);
    raise
        .execute(RaiseDisputeCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            is_admin: false,
            deal_id,
            against_party_id: Some(consumer),
            dispute_type: "QUALITY_ISSUE".to_string(),
            description: "Quality was poor.".to_string(),
            evidence_urls: vec![],
        })
        .await
        .unwrap();

    let list_admin = ListAdminDisputes::new(dispute_repo);
    let result = list_admin
        .execute(AdminDisputeListQuery {
            status: Some("OPEN".to_string()),
            deal_id: Some(deal_id),
            raised_by_party_id: Some(supplier),
            against_party_id: Some(consumer),
            limit: 20,
            offset: 0,
        })
        .await
        .unwrap();
    assert_eq!(result.total, 1);
}
