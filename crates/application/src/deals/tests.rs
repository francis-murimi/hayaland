use crate::agreements::{SignAgreement, SignAgreementCommand};
use crate::deals::dto::{
    CounterTermCommand, CreateDealCommand, ExecuteTransitionCommand, ListDealsQuery,
    ProposeTermCommand, SetValueDistributionCommand, SubmitDealCommand, TermActionCommand,
    UpdateDealCommand,
};
use crate::deals::{
    AcceptTerm, CounterTerm, CreateDeal, ExecuteTransition, GetDeal, GetValueDistribution,
    ListDeals, ListTerms, ProposeTerm, RejectTerm, SetValueDistribution, SubmitDeal, UpdateDeal,
    ValidateDeal, WithdrawTerm,
};
use crate::errors::ApplicationError;
use crate::parties::dto::CreatePartyCommand;
use crate::parties::CreateParty;
use crate::test_helpers::{FakeAgreementRepo, FakeDealRepo, FakePartyRepo};
use domain::entities::{
    DealRole, DealStatus, DistributionModel, ParticipationStatus, PartyType, TermStatus, TermType,
    ValueDistribution,
};
use domain::repositories::DealRepository;
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

async fn three_party_fixture() -> (
    Arc<FakePartyRepo>,
    Arc<FakeDealRepo>,
    Uuid,
    Uuid,
    Uuid,
    Uuid,
) {
    let party_repo = Arc::new(FakePartyRepo::default());
    let deal_repo = Arc::new(FakeDealRepo::default());

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
    (
        party_repo,
        deal_repo,
        supplier,
        consumer,
        enhancer,
        category_id,
    )
}

async fn set_valid_value_distribution(deal_repo: &Arc<FakeDealRepo>, deal_id: Uuid) {
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
}

async fn create_sample_deal() -> (
    Arc<FakePartyRepo>,
    Arc<FakeDealRepo>,
    Uuid,                          // deal id
    Uuid,                          // supplier
    Uuid,                          // consumer
    Uuid,                          // enhancer
    crate::deals::dto::DealResult, // full deal result
) {
    let (party_repo, deal_repo, supplier, consumer, enhancer, category_id) =
        three_party_fixture().await;

    let use_case = CreateDeal::new(deal_repo.clone(), party_repo.clone());
    let result = use_case
        .execute(CreateDealCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            title: "Three-Party Crop Deal".to_string(),
            description: None,
            domain_category_id: category_id,
            consumer_party_id: consumer,
            enhancer_party_id: enhancer,
            expected_start_date: None,
            expected_end_date: None,
            timeline: None,
            latitude: None,
            longitude: None,
        })
        .await
        .unwrap();

    (
        party_repo,
        deal_repo.clone(),
        result.id,
        supplier,
        consumer,
        enhancer,
        result,
    )
}

#[tokio::test]
async fn create_deal_stores_three_participations_and_sets_supplier_initiator() {
    let (_party_repo, _deal_repo, _deal_id, supplier, consumer, enhancer, result) =
        create_sample_deal().await;

    assert_eq!(result.deal_status, DealStatus::Draft);
    assert_eq!(result.initiator_party_id, supplier);
    assert_eq!(result.initiator_role, DealRole::Supplier);
    assert!(result.deal_reference.starts_with("DL-"));
    assert_eq!(result.participations.len(), 3);

    let roles: Vec<_> = result.participations.iter().map(|p| p.role).collect();
    assert!(roles.contains(&DealRole::Supplier));
    assert!(roles.contains(&DealRole::Consumer));
    assert!(roles.contains(&DealRole::Enhancer));

    let initiator = result
        .participations
        .iter()
        .find(|p| p.is_initiator)
        .unwrap();
    assert_eq!(initiator.party_id, supplier);
    assert_eq!(initiator.participation_status, "ACCEPTED");

    let consumer_part = result
        .participations
        .iter()
        .find(|p| p.party_id == consumer)
        .unwrap();
    assert_eq!(consumer_part.participation_status, "INVITED");

    let enhancer_part = result
        .participations
        .iter()
        .find(|p| p.party_id == enhancer)
        .unwrap();
    assert_eq!(enhancer_part.participation_status, "INVITED");
}

#[tokio::test]
async fn create_deal_rejects_actor_who_is_not_party_member() {
    let (party_repo, deal_repo, supplier, consumer, enhancer, category_id) =
        three_party_fixture().await;

    let use_case = CreateDeal::new(deal_repo, party_repo);
    let err = use_case
        .execute(CreateDealCommand {
            actor_user_id: other_user_id(),
            actor_party_id: supplier,
            title: "Bad Actor Deal".to_string(),
            description: None,
            domain_category_id: category_id,
            consumer_party_id: consumer,
            enhancer_party_id: enhancer,
            expected_start_date: None,
            expected_end_date: None,
            timeline: None,
            latitude: None,
            longitude: None,
        })
        .await
        .unwrap_err();

    assert!(matches!(err, crate::errors::ApplicationError::Forbidden));
}

#[tokio::test]
async fn get_deal_visible_to_participant_and_hidden_from_outsider() {
    let (party_repo, deal_repo, deal_id, supplier, _consumer, _enhancer, _result) =
        create_sample_deal().await;

    let get = GetDeal::new(deal_repo.clone(), party_repo.clone());

    let found = get
        .execute(deal_id, actor_user_id(), Some(supplier), false)
        .await
        .unwrap();
    assert_eq!(found.id, deal_id);

    let not_found = get
        .execute(deal_id, other_user_id(), None, false)
        .await
        .unwrap_err();
    assert!(matches!(
        not_found,
        crate::errors::ApplicationError::DealNotFound
    ));
}

#[tokio::test]
async fn update_deal_by_initiator_changes_title() {
    let (party_repo, deal_repo, deal_id, supplier, _consumer, _enhancer, _result) =
        create_sample_deal().await;

    let update = UpdateDeal::new(deal_repo.clone(), party_repo.clone());
    let result = update
        .execute(
            deal_id,
            UpdateDealCommand {
                actor_user_id: actor_user_id(),
                actor_party_id: supplier,
                title: Some("Updated Title".to_string()),
                description: None,
                domain_category_id: None,
                expected_start_date: None,
                expected_end_date: None,
                timeline: None,
                latitude: None,
                longitude: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(result.title, "Updated Title");
}

#[tokio::test]
async fn submit_deal_moves_to_suggested() {
    let (party_repo, deal_repo, deal_id, supplier, _consumer, _enhancer, _result) =
        create_sample_deal().await;

    set_valid_value_distribution(&deal_repo, deal_id).await;

    let submit = SubmitDeal::new(
        deal_repo,
        party_repo,
        domain::services::ValidationConfig::default(),
    );
    let result = submit
        .execute(
            deal_id,
            SubmitDealCommand {
                actor_user_id: actor_user_id(),
                actor_party_id: supplier,
            },
        )
        .await
        .unwrap();

    assert_eq!(result.deal_status, DealStatus::Suggested);
}

#[tokio::test]
async fn execute_transition_moves_through_negotiating_to_committed() {
    let (party_repo, deal_repo, deal_id, supplier, consumer, enhancer, _result) =
        create_sample_deal().await;

    // Move the deal directly to PENDING_REVIEW so we can exercise Negotiating.
    let mut aggregate = deal_repo
        .find_aggregate_by_id(deal_id)
        .await
        .unwrap()
        .unwrap();
    aggregate.deal.deal_status = DealStatus::PendingReview;
    deal_repo.update(&aggregate.deal).await.unwrap();

    // Manually accept all participations.
    let participations = deal_repo
        .find_participations_by_deal(deal_id)
        .await
        .unwrap();
    for mut p in participations {
        p.participation_status = ParticipationStatus::Accepted;
        p.responded_at = Some(time::OffsetDateTime::now_utc());
        deal_repo.update_participation(&p).await.unwrap();
    }

    set_valid_value_distribution(&deal_repo, deal_id).await;

    let agreement_repo = Arc::new(FakeAgreementRepo::default());
    let transition = ExecuteTransition::new(
        deal_repo.clone(),
        party_repo.clone(),
        agreement_repo.clone(),
        domain::services::ValidationConfig::default(),
    );

    let result = transition
        .execute(
            deal_id,
            ExecuteTransitionCommand {
                actor_user_id: actor_user_id(),
                actor_party_id: supplier,
                new_status: DealStatus::Negotiating,
                reason: None,
                acknowledge_warnings: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(result.deal_status, DealStatus::Negotiating);

    let result = transition
        .execute(
            deal_id,
            ExecuteTransitionCommand {
                actor_user_id: actor_user_id(),
                actor_party_id: supplier,
                new_status: DealStatus::TermsLocked,
                reason: None,
                acknowledge_warnings: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(result.deal_status, DealStatus::TermsLocked);

    // All three parties must sign before the deal can be committed.
    let sign = SignAgreement::new(deal_repo.clone(), party_repo.clone(), agreement_repo);
    for party_id in [supplier, consumer, enhancer] {
        sign.execute(SignAgreementCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: party_id,
            deal_id,
            signature_type: domain::entities::SignatureType::DigitalAttestation,
            ip_address: None,
        })
        .await
        .unwrap();
    }

    let result = transition
        .execute(
            deal_id,
            ExecuteTransitionCommand {
                actor_user_id: actor_user_id(),
                actor_party_id: supplier,
                new_status: DealStatus::Committed,
                reason: None,
                acknowledge_warnings: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(result.deal_status, DealStatus::Committed);
}

#[tokio::test]
async fn list_deals_returns_deals_for_member_party() {
    let (party_repo, deal_repo, deal_id, supplier, _consumer, _enhancer, _result) =
        create_sample_deal().await;

    let list = ListDeals::new(deal_repo, party_repo);
    let result = list
        .execute(
            actor_user_id(),
            Some(supplier),
            ListDealsQuery::default(),
            false,
        )
        .await
        .unwrap();

    assert_eq!(result.deals.len(), 1);
    assert_eq!(result.deals[0].id, deal_id);
    assert_eq!(result.total, 1);
}

#[tokio::test]
async fn propose_term_adds_term_to_deal() {
    let (_party_repo, deal_repo, deal_id, supplier, _consumer, _enhancer, _result) =
        create_sample_deal().await;

    let propose = ProposeTerm::new(deal_repo.clone(), _party_repo.clone());
    let term = propose
        .execute(ProposeTermCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            deal_id,
            term_type: TermType::Price,
            term_name: "Unit price".to_string(),
            description: "100 points per unit".to_string(),
            is_mandatory: true,
        })
        .await
        .unwrap();

    assert_eq!(term.deal_id, deal_id);
    assert_eq!(term.proposed_by_party_id, supplier);
    assert!(matches!(term.negotiation_status, TermStatus::Proposed));
    assert_eq!(term.version, 1);

    let list = ListTerms::new(deal_repo, _party_repo);
    let terms = list
        .execute(deal_id, actor_user_id(), Some(supplier), false)
        .await
        .unwrap();
    assert_eq!(terms.len(), 1);
}

#[tokio::test]
async fn accept_term_marks_term_accepted() {
    let (_party_repo, deal_repo, deal_id, supplier, consumer, _enhancer, _result) =
        create_sample_deal().await;

    let propose = ProposeTerm::new(deal_repo.clone(), _party_repo.clone());
    let term = propose
        .execute(crate::deals::dto::ProposeTermCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            deal_id,
            term_type: TermType::Price,
            term_name: "Unit price".to_string(),
            description: "100 points per unit".to_string(),
            is_mandatory: true,
        })
        .await
        .unwrap();

    let accept = AcceptTerm::new(deal_repo.clone(), _party_repo.clone());
    let accepted = accept
        .execute(TermActionCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: consumer,
            deal_id,
            term_id: term.id,
        })
        .await
        .unwrap();

    assert!(matches!(accepted.negotiation_status, TermStatus::Accepted));
}

#[tokio::test]
async fn counter_term_creates_new_version() {
    let (_party_repo, deal_repo, deal_id, supplier, consumer, _enhancer, _result) =
        create_sample_deal().await;

    let propose = ProposeTerm::new(deal_repo.clone(), _party_repo.clone());
    let term = propose
        .execute(crate::deals::dto::ProposeTermCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            deal_id,
            term_type: TermType::Price,
            term_name: "Unit price".to_string(),
            description: "100 points per unit".to_string(),
            is_mandatory: false,
        })
        .await
        .unwrap();

    let counter = CounterTerm::new(deal_repo.clone(), _party_repo.clone());
    let counter_term = counter
        .execute(CounterTermCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: consumer,
            deal_id,
            term_id: term.id,
            description: "90 points per unit".to_string(),
        })
        .await
        .unwrap();

    assert_eq!(counter_term.version, 2);
    assert_eq!(counter_term.parent_term_id, Some(term.id));
    assert_eq!(counter_term.proposed_by_party_id, consumer);
}

#[tokio::test]
async fn reject_and_withdraw_term_are_terminal() {
    let (_party_repo, deal_repo, deal_id, supplier, consumer, _enhancer, _result) =
        create_sample_deal().await;

    let propose = ProposeTerm::new(deal_repo.clone(), _party_repo.clone());
    let term = propose
        .execute(crate::deals::dto::ProposeTermCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            deal_id,
            term_type: TermType::Price,
            term_name: "Unit price".to_string(),
            description: "100 points per unit".to_string(),
            is_mandatory: false,
        })
        .await
        .unwrap();

    let reject = RejectTerm::new(deal_repo.clone(), _party_repo.clone());
    let rejected = reject
        .execute(TermActionCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: consumer,
            deal_id,
            term_id: term.id,
        })
        .await
        .unwrap();
    assert!(matches!(rejected.negotiation_status, TermStatus::Rejected));

    let term2 = propose
        .execute(ProposeTermCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            deal_id,
            term_type: TermType::PaymentTerms,
            term_name: "Payment".to_string(),
            description: "net 30".to_string(),
            is_mandatory: false,
        })
        .await
        .unwrap();

    let withdraw = WithdrawTerm::new(deal_repo, _party_repo);
    let withdrawn = withdraw
        .execute(TermActionCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            deal_id,
            term_id: term2.id,
        })
        .await
        .unwrap();
    assert!(matches!(
        withdrawn.negotiation_status,
        TermStatus::Withdrawn
    ));
}

#[tokio::test]
async fn set_and_get_value_distribution_updates_deal_totals() {
    let (_party_repo, deal_repo, deal_id, supplier, _consumer, _enhancer, _result) =
        create_sample_deal().await;

    let set = SetValueDistribution::new(deal_repo.clone(), _party_repo.clone());
    let vd = set
        .execute(SetValueDistributionCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            deal_id,
            total_value: Decimal::from(5000),
            distribution_model: domain::entities::DistributionModel::FixedPrice,
            supplier_share_percentage: Decimal::from(70),
            enhancer_share_percentage: Decimal::from(20),
            platform_fee_percentage: Decimal::from(10),
            consumer_cost_percentage: Decimal::from(100),
            payment_schedule: vec![],
        })
        .await
        .unwrap();

    assert_eq!(vd.total_value, Decimal::from(5000));
    assert_eq!(vd.supplier_share_amount, Decimal::from(3500));
    assert_eq!(vd.enhancer_share_amount, Decimal::from(1000));
    assert_eq!(vd.platform_fee_amount, Decimal::from(500));

    let deal = deal_repo.find_by_id(deal_id).await.unwrap().unwrap();
    assert_eq!(deal.total_deal_value, Some(Decimal::from(5000)));
    assert_eq!(deal.platform_fee_percentage, Decimal::from(10));

    let get = GetValueDistribution::new(deal_repo, _party_repo);
    let fetched = get
        .execute(deal_id, actor_user_id(), Some(supplier), false)
        .await
        .unwrap();
    assert_eq!(fetched.id, vd.id);
}

#[tokio::test]
async fn validate_deal_returns_good_score() {
    let (_party_repo, deal_repo, deal_id, _supplier, _consumer, _enhancer, _result) =
        create_sample_deal().await;
    set_valid_value_distribution(&deal_repo, deal_id).await;

    let validate = ValidateDeal::new(deal_repo.clone(), ValidationConfig::default());
    let result = validate.execute(deal_id).await.unwrap();

    assert!(!result.blocked);
    assert!(result.score >= Decimal::from(70));
    assert_eq!(result.status, "GOOD");

    let deal = deal_repo.find_by_id(deal_id).await.unwrap().unwrap();
    assert!(deal.win_win_win_validated);
    assert!(deal.validation_score.is_some());
}

#[tokio::test]
async fn submit_deal_requires_value_distribution() {
    let (_party_repo, deal_repo, deal_id, supplier, _consumer, _enhancer, _result) =
        create_sample_deal().await;

    let submit = SubmitDeal::new(deal_repo, _party_repo, ValidationConfig::default());
    let err = submit
        .execute(
            deal_id,
            SubmitDealCommand {
                actor_user_id: actor_user_id(),
                actor_party_id: supplier,
            },
        )
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        ApplicationError::WinWinWinValidationFailed { .. }
    ));
}

#[tokio::test]
async fn locking_terms_requires_value_distribution() {
    let (_party_repo, deal_repo, deal_id, supplier, _consumer, _enhancer, _result) =
        create_sample_deal().await;

    let mut aggregate = deal_repo
        .find_aggregate_by_id(deal_id)
        .await
        .unwrap()
        .unwrap();
    aggregate.deal.deal_status = DealStatus::Negotiating;
    deal_repo.update(&aggregate.deal).await.unwrap();

    let transition = ExecuteTransition::new(
        deal_repo,
        _party_repo,
        Arc::new(FakeAgreementRepo::default()),
        ValidationConfig::default(),
    );
    let err = transition
        .execute(
            deal_id,
            ExecuteTransitionCommand {
                actor_user_id: actor_user_id(),
                actor_party_id: supplier,
                new_status: DealStatus::TermsLocked,
                reason: None,
                acknowledge_warnings: false,
            },
        )
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        ApplicationError::WinWinWinValidationFailed { .. }
    ));
}
