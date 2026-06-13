use crate::deals::dto::{
    CreateDealCommand, ExecuteTransitionCommand, ListDealsQuery, SubmitDealCommand,
    UpdateDealCommand,
};
use crate::deals::{CreateDeal, ExecuteTransition, GetDeal, ListDeals, SubmitDeal, UpdateDeal};
use crate::parties::dto::CreatePartyCommand;
use crate::parties::CreateParty;
use crate::test_helpers::{FakeDealRepo, FakePartyRepo};
use domain::entities::{DealRole, DealStatus, ParticipationStatus, PartyType};
use domain::repositories::DealRepository;
use std::sync::Arc;
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

    let submit = SubmitDeal::new(deal_repo, party_repo);
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
    let (party_repo, deal_repo, deal_id, supplier, _consumer, _enhancer, _result) =
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

    let transition = ExecuteTransition::new(deal_repo.clone(), party_repo.clone());

    let result = transition
        .execute(
            deal_id,
            ExecuteTransitionCommand {
                actor_user_id: actor_user_id(),
                actor_party_id: supplier,
                new_status: DealStatus::Negotiating,
                reason: None,
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
            },
        )
        .await
        .unwrap();
    assert_eq!(result.deal_status, DealStatus::TermsLocked);

    let result = transition
        .execute(
            deal_id,
            ExecuteTransitionCommand {
                actor_user_id: actor_user_id(),
                actor_party_id: supplier,
                new_status: DealStatus::Committed,
                reason: None,
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
