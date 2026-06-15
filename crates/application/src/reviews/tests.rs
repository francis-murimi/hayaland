use crate::deals::dto::ExecuteTransitionCommand;
use crate::deals::execute_transition::ExecuteTransition;
use crate::errors::ApplicationError;
use crate::parties::dto::CreatePartyCommand;
use crate::parties::CreateParty;
use crate::reviews::dto::{
    AdminReviewListQuery, GetReviewQuery, ListDealReviewsQuery, ListPartyReviewsQuery,
    SubmitReviewCommand,
};
use crate::reviews::submit_review::NoOpTrustScoreRecalculation;
use crate::reviews::{
    GetDealReviewStatus, GetReview, HideReview, ListAdminReviews, ListDealReviews,
    ListPartyReviews, SubmitReview,
};
use crate::test_helpers::{
    FakeAgreementRepo, FakeDealRepo, FakeMilestoneRepo, FakePartyRepo, FakeReviewRepo,
};
use domain::entities::{Deal, DealParticipation, DealRole, DealStatus, DealTitle, PartyType};
use domain::repositories::{DealAggregate, DealRepository, MilestoneRepository, ReviewRepository};
use domain::services::ValidationConfig;
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
    Arc<FakeReviewRepo>,
    Uuid, // deal id
    Uuid, // supplier
    Uuid, // consumer
    Uuid, // enhancer
) {
    let party_repo = Arc::new(FakePartyRepo::default());
    let deal_repo = Arc::new(FakeDealRepo::default());
    let review_repo = Arc::new(FakeReviewRepo::default());

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
        review_repo,
        deal_id,
        supplier,
        consumer,
        enhancer,
    )
}

fn submit_review_use_case(
    review_repo: Arc<FakeReviewRepo>,
    deal_repo: Arc<FakeDealRepo>,
    party_repo: Arc<FakePartyRepo>,
) -> SubmitReview {
    SubmitReview::new(
        review_repo,
        deal_repo,
        party_repo,
        Arc::new(NoOpTrustScoreRecalculation),
    )
}

#[tokio::test]
async fn submit_review_happy_path() {
    let (party_repo, deal_repo, review_repo, deal_id, supplier, consumer, _) =
        three_party_fixture().await;

    let use_case = submit_review_use_case(review_repo, deal_repo, party_repo);

    let result = use_case
        .execute(SubmitReviewCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            is_admin: false,
            deal_id,
            reviewed_party_id: consumer,
            overall_rating: 4,
            communication_rating: Some(5),
            reliability_rating: None,
            quality_rating: None,
            timeliness_rating: None,
            review_text: Some("Great partner".to_string()),
            is_public: Some(true),
        })
        .await
        .unwrap();

    assert_eq!(result.overall_rating, 4);
    assert_eq!(result.communication_rating, Some(5));
    assert_eq!(result.reviewer_party_id, supplier);
    assert_eq!(result.reviewed_party_id, consumer);
    assert_eq!(result.reviewed_role, DealRole::Consumer);
    assert!(result.is_verified);
}

#[tokio::test]
async fn submit_review_rejects_duplicate() {
    let (party_repo, deal_repo, review_repo, deal_id, supplier, consumer, _) =
        three_party_fixture().await;

    let use_case =
        submit_review_use_case(review_repo.clone(), deal_repo.clone(), party_repo.clone());

    let cmd = SubmitReviewCommand {
        actor_user_id: actor_user_id(),
        actor_party_id: supplier,
        is_admin: false,
        deal_id,
        reviewed_party_id: consumer,
        overall_rating: 4,
        communication_rating: None,
        reliability_rating: None,
        quality_rating: None,
        timeliness_rating: None,
        review_text: None,
        is_public: Some(true),
    };

    assert!(use_case.execute(cmd.clone()).await.is_ok());
    let err = use_case.execute(cmd).await.unwrap_err();
    assert!(matches!(err, ApplicationError::DuplicateReview));
}

#[tokio::test]
async fn submit_review_rejects_self_review() {
    let (party_repo, deal_repo, review_repo, deal_id, supplier, _, _) = three_party_fixture().await;

    let use_case = submit_review_use_case(review_repo, deal_repo, party_repo);

    let err = use_case
        .execute(SubmitReviewCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            is_admin: false,
            deal_id,
            reviewed_party_id: supplier,
            overall_rating: 5,
            communication_rating: None,
            reliability_rating: None,
            quality_rating: None,
            timeliness_rating: None,
            review_text: None,
            is_public: Some(true),
        })
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Validation(_)));
}

#[tokio::test]
async fn submit_review_rejects_non_participant_reviewer() {
    let (party_repo, deal_repo, review_repo, deal_id, _, consumer, _) = three_party_fixture().await;

    let use_case = submit_review_use_case(review_repo, deal_repo, party_repo.clone());

    let outsider_party = create_party(
        &party_repo,
        "Outsider",
        "outsider@example.com",
        vec![DealRole::Supplier],
    )
    .await;

    let err = use_case
        .execute(SubmitReviewCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: outsider_party,
            is_admin: false,
            deal_id,
            reviewed_party_id: consumer,
            overall_rating: 5,
            communication_rating: None,
            reliability_rating: None,
            quality_rating: None,
            timeliness_rating: None,
            review_text: None,
            is_public: Some(true),
        })
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::DealAccessDenied));
}

#[tokio::test]
async fn submit_review_rejects_invalid_deal_status() {
    let (party_repo, deal_repo, review_repo, deal_id, supplier, consumer, _) =
        three_party_fixture().await;

    // Change deal status to committed.
    {
        let mut deal = deal_repo.find_by_id(deal_id).await.unwrap().unwrap();
        deal.deal_status = DealStatus::Committed;
        deal_repo.update(&deal).await.unwrap();
    }

    let use_case = submit_review_use_case(review_repo, deal_repo, party_repo);

    let err = use_case
        .execute(SubmitReviewCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            is_admin: false,
            deal_id,
            reviewed_party_id: consumer,
            overall_rating: 5,
            communication_rating: None,
            reliability_rating: None,
            quality_rating: None,
            timeliness_rating: None,
            review_text: None,
            is_public: Some(true),
        })
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Validation(_)));
}

#[tokio::test]
async fn submit_review_rejects_invalid_rating() {
    let (party_repo, deal_repo, review_repo, deal_id, supplier, consumer, _) =
        three_party_fixture().await;

    let use_case = submit_review_use_case(review_repo, deal_repo, party_repo);

    let err = use_case
        .execute(SubmitReviewCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            is_admin: false,
            deal_id,
            reviewed_party_id: consumer,
            overall_rating: 6,
            communication_rating: None,
            reliability_rating: None,
            quality_rating: None,
            timeliness_rating: None,
            review_text: None,
            is_public: Some(true),
        })
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Validation(_)));
}

#[tokio::test]
async fn list_deal_reviews_visible_to_participant() {
    let (party_repo, deal_repo, review_repo, deal_id, supplier, consumer, enhancer) =
        three_party_fixture().await;

    let submit = submit_review_use_case(review_repo.clone(), deal_repo.clone(), party_repo.clone());
    submit
        .execute(SubmitReviewCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            is_admin: false,
            deal_id,
            reviewed_party_id: consumer,
            overall_rating: 4,
            communication_rating: None,
            reliability_rating: None,
            quality_rating: None,
            timeliness_rating: None,
            review_text: None,
            is_public: Some(true),
        })
        .await
        .unwrap();

    let list = ListDealReviews::new(deal_repo.clone(), review_repo.clone());
    let result = list
        .execute(
            deal_id,
            actor_user_id(),
            Some(enhancer),
            false,
            ListDealReviewsQuery::default(),
        )
        .await
        .unwrap();
    assert_eq!(result.total, 1);

    // Non-participant cannot list.
    let err = list
        .execute(
            deal_id,
            other_user_id(),
            None,
            false,
            ListDealReviewsQuery::default(),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, ApplicationError::DealAccessDenied));
}

#[tokio::test]
async fn list_party_reviews_filters_public_private() {
    let (party_repo, deal_repo, review_repo, deal_id, supplier, consumer, enhancer) =
        three_party_fixture().await;

    let submit = submit_review_use_case(review_repo.clone(), deal_repo.clone(), party_repo.clone());

    // Public review for consumer.
    submit
        .execute(SubmitReviewCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            is_admin: false,
            deal_id,
            reviewed_party_id: consumer,
            overall_rating: 4,
            communication_rating: None,
            reliability_rating: None,
            quality_rating: None,
            timeliness_rating: None,
            review_text: Some("public".to_string()),
            is_public: Some(true),
        })
        .await
        .unwrap();

    // Private review for consumer.
    submit
        .execute(SubmitReviewCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: enhancer,
            is_admin: false,
            deal_id,
            reviewed_party_id: consumer,
            overall_rating: 5,
            communication_rating: None,
            reliability_rating: None,
            quality_rating: None,
            timeliness_rating: None,
            review_text: Some("private".to_string()),
            is_public: Some(false),
        })
        .await
        .unwrap();

    let list = ListPartyReviews::new(party_repo.clone(), review_repo.clone());

    // Outsider sees only public review.
    let public_only = list
        .execute(
            consumer,
            ListPartyReviewsQuery {
                actor_user_id: other_user_id(),
                actor_party_id: Some(supplier),
                is_admin: false,
                limit: 10,
                offset: 0,
            },
        )
        .await
        .unwrap();
    assert_eq!(public_only.total, 1);
    assert_eq!(public_only.reviews[0].overall_rating, 4);

    // Own party sees both.
    let own = list
        .execute(
            consumer,
            ListPartyReviewsQuery {
                actor_user_id: actor_user_id(),
                actor_party_id: Some(consumer),
                is_admin: false,
                limit: 10,
                offset: 0,
            },
        )
        .await
        .unwrap();
    assert_eq!(own.total, 2);

    // Admin sees both.
    let admin = list
        .execute(
            consumer,
            ListPartyReviewsQuery {
                actor_user_id: actor_user_id(),
                actor_party_id: None,
                is_admin: true,
                limit: 10,
                offset: 0,
            },
        )
        .await
        .unwrap();
    assert_eq!(admin.total, 2);
}

#[tokio::test]
async fn get_private_review_visibility() {
    let (party_repo, deal_repo, review_repo, deal_id, supplier, consumer, enhancer) =
        three_party_fixture().await;

    let submit = submit_review_use_case(review_repo.clone(), deal_repo.clone(), party_repo.clone());
    let result = submit
        .execute(SubmitReviewCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            is_admin: false,
            deal_id,
            reviewed_party_id: consumer,
            overall_rating: 4,
            communication_rating: None,
            reliability_rating: None,
            quality_rating: None,
            timeliness_rating: None,
            review_text: Some("secret".to_string()),
            is_public: Some(false),
        })
        .await
        .unwrap();

    let get = GetReview::new(deal_repo.clone(), review_repo.clone());

    // Reviewer can see private review.
    get.execute(
        result.id,
        GetReviewQuery {
            actor_user_id: actor_user_id(),
            actor_party_id: Some(supplier),
            is_admin: false,
        },
    )
    .await
    .unwrap();

    // Reviewed party can see private review.
    get.execute(
        result.id,
        GetReviewQuery {
            actor_user_id: actor_user_id(),
            actor_party_id: Some(consumer),
            is_admin: false,
        },
    )
    .await
    .unwrap();

    // Another participant cannot see private review.
    let err = get
        .execute(
            result.id,
            GetReviewQuery {
                actor_user_id: actor_user_id(),
                actor_party_id: Some(enhancer),
                is_admin: false,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, ApplicationError::DealAccessDenied));

    // Admin can see private review.
    get.execute(
        result.id,
        GetReviewQuery {
            actor_user_id: actor_user_id(),
            actor_party_id: None,
            is_admin: true,
        },
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn get_deal_review_status_reports_missing_pairs() {
    let (party_repo, deal_repo, review_repo, deal_id, supplier, consumer, enhancer) =
        three_party_fixture().await;

    let status = GetDealReviewStatus::new(deal_repo.clone(), review_repo.clone());

    let result = status
        .execute(deal_id, actor_user_id(), Some(supplier), false)
        .await
        .unwrap();
    assert_eq!(result.total_required, 6);
    assert_eq!(result.total_received, 0);
    assert!(!result.is_complete);
    assert_eq!(result.missing_pairs.len(), 6);

    let submit = submit_review_use_case(review_repo.clone(), deal_repo.clone(), party_repo.clone());
    for (reviewer, reviewed) in [
        (supplier, consumer),
        (supplier, enhancer),
        (consumer, supplier),
        (consumer, enhancer),
        (enhancer, supplier),
        (enhancer, consumer),
    ] {
        submit
            .execute(SubmitReviewCommand {
                actor_user_id: actor_user_id(),
                actor_party_id: reviewer,
                is_admin: false,
                deal_id,
                reviewed_party_id: reviewed,
                overall_rating: 4,
                communication_rating: None,
                reliability_rating: None,
                quality_rating: None,
                timeliness_rating: None,
                review_text: None,
                is_public: Some(true),
            })
            .await
            .unwrap();
    }

    let result = status
        .execute(deal_id, actor_user_id(), Some(supplier), false)
        .await
        .unwrap();
    assert_eq!(result.total_required, 6);
    assert_eq!(result.total_received, 6);
    assert!(result.is_complete);
    assert!(result.missing_pairs.is_empty());
}

#[tokio::test]
async fn execute_transition_requires_all_reviews_for_completion() {
    let (party_repo, deal_repo, review_repo, deal_id, supplier, consumer, enhancer) =
        three_party_fixture().await;

    let agreement_repo = Arc::new(FakeAgreementRepo::default());
    let milestone_repo = Arc::new(FakeMilestoneRepo::default());

    let transition = ExecuteTransition::new_with_reviews(
        deal_repo.clone(),
        party_repo.clone(),
        agreement_repo,
        milestone_repo.clone(),
        review_repo.clone(),
        ValidationConfig::default(),
    );

    // Missing milestones should be caught first.
    let err = transition
        .execute(
            deal_id,
            ExecuteTransitionCommand {
                actor_user_id: actor_user_id(),
                actor_party_id: supplier,
                is_admin: false,
                new_status: DealStatus::Completed,
                reason: None,
                acknowledge_warnings: false,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, ApplicationError::Validation(_)));

    // Add a verified milestone.
    use domain::entities::{Milestone, MilestoneStatus};
    use rust_decimal::Decimal;
    let now = time::OffsetDateTime::now_utc();
    let milestone = Milestone {
        id: Uuid::now_v7(),
        deal_id,
        milestone_name: "Milestone 1".to_string(),
        description: None,
        assigned_to_party_id: consumer,
        verified_by_party_id: supplier,
        due_date: None,
        completion_criteria: "Done".to_string(),
        milestone_status: MilestoneStatus::Verified,
        completion_percentage: Decimal::from(100),
        payment_trigger_amount: None,
        completed_at: Some(now),
        display_order: 1,
        created_at: now,
        updated_at: now,
    };
    milestone_repo.create(&milestone).await.unwrap();

    // Now reviews are missing.
    let err = transition
        .execute(
            deal_id,
            ExecuteTransitionCommand {
                actor_user_id: actor_user_id(),
                actor_party_id: supplier,
                is_admin: false,
                new_status: DealStatus::Completed,
                reason: None,
                acknowledge_warnings: false,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, ApplicationError::Validation(_)));

    // Submit all reviews.
    let submit = submit_review_use_case(review_repo.clone(), deal_repo.clone(), party_repo.clone());
    for (reviewer, reviewed) in [
        (supplier, consumer),
        (supplier, enhancer),
        (consumer, supplier),
        (consumer, enhancer),
        (enhancer, supplier),
        (enhancer, consumer),
    ] {
        submit
            .execute(SubmitReviewCommand {
                actor_user_id: actor_user_id(),
                actor_party_id: reviewer,
                is_admin: false,
                deal_id,
                reviewed_party_id: reviewed,
                overall_rating: 4,
                communication_rating: None,
                reliability_rating: None,
                quality_rating: None,
                timeliness_rating: None,
                review_text: None,
                is_public: Some(true),
            })
            .await
            .unwrap();
    }

    // Transition should succeed.
    let result = transition
        .execute(
            deal_id,
            ExecuteTransitionCommand {
                actor_user_id: actor_user_id(),
                actor_party_id: supplier,
                is_admin: false,
                new_status: DealStatus::Completed,
                reason: None,
                acknowledge_warnings: false,
            },
        )
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().deal_status, DealStatus::Completed);
}

#[tokio::test]
async fn admin_hide_review_clears_text_and_makes_private() {
    let (party_repo, deal_repo, review_repo, deal_id, supplier, consumer, _) =
        three_party_fixture().await;

    let submit = submit_review_use_case(review_repo.clone(), deal_repo.clone(), party_repo.clone());
    let result = submit
        .execute(SubmitReviewCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            is_admin: false,
            deal_id,
            reviewed_party_id: consumer,
            overall_rating: 4,
            communication_rating: None,
            reliability_rating: None,
            quality_rating: None,
            timeliness_rating: None,
            review_text: Some("needs moderation".to_string()),
            is_public: Some(true),
        })
        .await
        .unwrap();

    let hide = HideReview::new(review_repo.clone());
    hide.execute(result.id, Some("removed by admin".to_string()))
        .await
        .unwrap();

    let hidden = review_repo.find_by_id(result.id).await.unwrap().unwrap();
    assert!(!hidden.is_public);
    assert!(hidden.review_text.is_none());
    assert_eq!(
        hidden.platform_response.as_deref(),
        Some("removed by admin")
    );
}

#[tokio::test]
async fn list_admin_reviews_filters_by_criteria() {
    let (party_repo, deal_repo, review_repo, deal_id, supplier, consumer, enhancer) =
        three_party_fixture().await;

    let submit = submit_review_use_case(review_repo.clone(), deal_repo.clone(), party_repo.clone());
    submit
        .execute(SubmitReviewCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            is_admin: false,
            deal_id,
            reviewed_party_id: consumer,
            overall_rating: 4,
            communication_rating: None,
            reliability_rating: None,
            quality_rating: None,
            timeliness_rating: None,
            review_text: None,
            is_public: Some(true),
        })
        .await
        .unwrap();
    submit
        .execute(SubmitReviewCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: enhancer,
            is_admin: false,
            deal_id,
            reviewed_party_id: supplier,
            overall_rating: 5,
            communication_rating: None,
            reliability_rating: None,
            quality_rating: None,
            timeliness_rating: None,
            review_text: None,
            is_public: Some(false),
        })
        .await
        .unwrap();

    let list = ListAdminReviews::new(review_repo);

    let all = list
        .execute(AdminReviewListQuery {
            limit: 10,
            offset: 0,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(all.total, 2);

    let by_reviewer = list
        .execute(AdminReviewListQuery {
            reviewer_party_id: Some(supplier),
            limit: 10,
            offset: 0,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(by_reviewer.total, 1);
    assert_eq!(by_reviewer.reviews[0].overall_rating, 4);

    let public_only = list
        .execute(AdminReviewListQuery {
            is_public: Some(true),
            limit: 10,
            offset: 0,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(public_only.total, 1);
}
