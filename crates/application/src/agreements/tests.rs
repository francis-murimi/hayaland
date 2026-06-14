use crate::agreements::dto::{
    AdminUpdateAgreementCommand, GenerateAgreementCommand, SignAgreementCommand,
};
use crate::agreements::{AdminUpdateAgreement, GenerateAgreement, GetAgreement, SignAgreement};
use crate::deals::dto::{CreateDealCommand, ProposeTermCommand, SetValueDistributionCommand};
use crate::deals::{AcceptTerm, CreateDeal, ProposeTerm, SetValueDistribution};
use crate::errors::ApplicationError;
use crate::parties::dto::CreatePartyCommand;
use crate::parties::CreateParty;
use crate::test_helpers::{FakeAgreementRepo, FakeDealRepo, FakePartyRepo};
use domain::entities::{
    AgreementStatus, DealRole, DealStatus, DistributionModel, PartyType, SignatureType, TermType,
};
use domain::repositories::{AgreementRepository, DealRepository};
use rust_decimal::Decimal;
use std::sync::Arc;
use uuid::Uuid;

fn actor_user_id() -> Uuid {
    Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap()
}

fn outsider_user_id() -> Uuid {
    Uuid::parse_str("99999999-9999-9999-9999-999999999999").unwrap()
}

async fn create_party(
    repo: &Arc<FakePartyRepo>,
    display_name: &str,
    email: &str,
    roles: Vec<DealRole>,
) -> Uuid {
    CreateParty::new(repo.clone())
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
        .unwrap()
        .id
}

async fn locked_three_party_deal() -> (
    Arc<FakePartyRepo>,
    Arc<FakeDealRepo>,
    Arc<FakeAgreementRepo>,
    Uuid, // deal id
    Uuid, // supplier
    Uuid, // consumer
    Uuid, // enhancer
) {
    let party_repo = Arc::new(FakePartyRepo::default());
    let deal_repo = Arc::new(FakeDealRepo::default());
    let agreement_repo = Arc::new(FakeAgreementRepo::default());

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
    let deal = CreateDeal::new(deal_repo.clone(), party_repo.clone())
        .execute(CreateDealCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            title: "Three-Party Deal".to_string(),
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

    SetValueDistribution::new(deal_repo.clone(), party_repo.clone())
        .execute(SetValueDistributionCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            deal_id: deal.id,
            total_value: Decimal::from(10000),
            distribution_model: DistributionModel::FixedPrice,
            supplier_share_percentage: Decimal::from(60),
            enhancer_share_percentage: Decimal::from(30),
            platform_fee_percentage: Decimal::from(10),
            consumer_cost_percentage: Decimal::from(100),
            payment_schedule: vec![],
        })
        .await
        .unwrap();

    // Lock the deal status directly; transition tests are covered in the deals module.
    let mut aggregate = deal_repo
        .find_aggregate_by_id(deal.id)
        .await
        .unwrap()
        .unwrap();
    aggregate.deal.deal_status = DealStatus::TermsLocked;
    deal_repo.update(&aggregate.deal).await.unwrap();

    (
        party_repo,
        deal_repo,
        agreement_repo,
        deal.id,
        supplier,
        consumer,
        enhancer,
    )
}

async fn accept_mandatory_term(
    deal_repo: &Arc<FakeDealRepo>,
    party_repo: &Arc<FakePartyRepo>,
    deal_id: Uuid,
    proposed_by: Uuid,
    accepted_by: Uuid,
) {
    // Terms must be proposed while the deal is still negotiable.
    let mut aggregate = deal_repo
        .find_aggregate_by_id(deal_id)
        .await
        .unwrap()
        .unwrap();
    let previous_status = aggregate.deal.deal_status;
    aggregate.deal.deal_status = DealStatus::Negotiating;
    deal_repo.update(&aggregate.deal).await.unwrap();

    let term = ProposeTerm::new(deal_repo.clone(), party_repo.clone())
        .execute(ProposeTermCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: proposed_by,
            deal_id,
            term_type: TermType::Price,
            term_name: "Unit price".to_string(),
            description: "100 points".to_string(),
            is_mandatory: true,
        })
        .await
        .unwrap();

    AcceptTerm::new(deal_repo.clone(), party_repo.clone())
        .execute(crate::deals::dto::TermActionCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: accepted_by,
            deal_id,
            term_id: term.id,
        })
        .await
        .unwrap();

    // Restore the deal status for agreement generation/signing.
    let mut aggregate = deal_repo
        .find_aggregate_by_id(deal_id)
        .await
        .unwrap()
        .unwrap();
    aggregate.deal.deal_status = previous_status;
    deal_repo.update(&aggregate.deal).await.unwrap();
}

#[tokio::test]
async fn generate_agreement_creates_pending_signature_agreement() {
    let (party_repo, deal_repo, agreement_repo, deal_id, supplier, consumer, _enhancer) =
        locked_three_party_deal().await;

    accept_mandatory_term(&deal_repo, &party_repo, deal_id, supplier, consumer).await;

    let result = GenerateAgreement::new(deal_repo, party_repo, agreement_repo)
        .execute(GenerateAgreementCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            deal_id,
        })
        .await
        .unwrap();

    assert_eq!(result.deal_id, deal_id);
    assert_eq!(result.status, AgreementStatus::PendingSignatures);
    assert_eq!(result.version, 1);
    assert!(!result.agreement_text.is_empty());
    assert!(result.agreement_text.contains("Supplier"));
    assert!(result.agreement_text.contains("Consumer"));
    assert!(result.agreement_text.contains("Enhancer"));
}

#[tokio::test]
async fn generate_agreement_requires_terms_locked_status() {
    let (party_repo, deal_repo, agreement_repo, deal_id, supplier, _consumer, _enhancer) =
        locked_three_party_deal().await;

    // Move deal out of TERMS_LOCKED.
    let mut aggregate = deal_repo
        .find_aggregate_by_id(deal_id)
        .await
        .unwrap()
        .unwrap();
    aggregate.deal.deal_status = DealStatus::Negotiating;
    deal_repo.update(&aggregate.deal).await.unwrap();

    let err = GenerateAgreement::new(deal_repo, party_repo, agreement_repo)
        .execute(GenerateAgreementCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            deal_id,
        })
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Validation(_)));
}

#[tokio::test]
async fn generate_agreement_requires_mandatory_terms_accepted() {
    let (party_repo, deal_repo, agreement_repo, deal_id, supplier, _consumer, _enhancer) =
        locked_three_party_deal().await;

    // Propose but do not accept a mandatory term. Terms cannot be proposed once locked.
    let mut aggregate = deal_repo
        .find_aggregate_by_id(deal_id)
        .await
        .unwrap()
        .unwrap();
    aggregate.deal.deal_status = DealStatus::Negotiating;
    deal_repo.update(&aggregate.deal).await.unwrap();

    ProposeTerm::new(deal_repo.clone(), party_repo.clone())
        .execute(ProposeTermCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            deal_id,
            term_type: TermType::Price,
            term_name: "Unit price".to_string(),
            description: "100 points".to_string(),
            is_mandatory: true,
        })
        .await
        .unwrap();

    // Restore locked status so the generate use case reaches the mandatory-term check.
    let mut aggregate = deal_repo
        .find_aggregate_by_id(deal_id)
        .await
        .unwrap()
        .unwrap();
    aggregate.deal.deal_status = DealStatus::TermsLocked;
    deal_repo.update(&aggregate.deal).await.unwrap();

    let err = GenerateAgreement::new(deal_repo, party_repo, agreement_repo)
        .execute(GenerateAgreementCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            deal_id,
        })
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Validation(_)));
}

#[tokio::test]
async fn sign_agreement_records_signature() {
    let (party_repo, deal_repo, agreement_repo, deal_id, supplier, consumer, _enhancer) =
        locked_three_party_deal().await;
    accept_mandatory_term(&deal_repo, &party_repo, deal_id, supplier, consumer).await;

    let sign_repo = agreement_repo.clone();
    let generate = GenerateAgreement::new(deal_repo.clone(), party_repo.clone(), agreement_repo);
    generate
        .execute(GenerateAgreementCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            deal_id,
        })
        .await
        .unwrap();

    let sign = SignAgreement::new(deal_repo.clone(), party_repo.clone(), sign_repo);
    let result = sign
        .execute(SignAgreementCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            deal_id,
            signature_type: SignatureType::DigitalAttestation,
            ip_address: Some("127.0.0.1".to_string()),
        })
        .await
        .unwrap();

    assert_eq!(result.signatures.len(), 1);
    assert_eq!(result.signatures[0].party_id, supplier);
    assert!(result.signatures[0].signature_data.starts_with("sha256:"));
}

#[tokio::test]
async fn sign_agreement_fails_for_non_participant_party() {
    let (party_repo, deal_repo, agreement_repo, deal_id, supplier, consumer, _enhancer) =
        locked_three_party_deal().await;
    accept_mandatory_term(&deal_repo, &party_repo, deal_id, supplier, consumer).await;

    GenerateAgreement::new(
        deal_repo.clone(),
        party_repo.clone(),
        agreement_repo.clone(),
    )
    .execute(GenerateAgreementCommand {
        actor_user_id: actor_user_id(),
        actor_party_id: supplier,
        deal_id,
    })
    .await
    .unwrap();

    // Use a party id that is not part of the deal.
    let outsider_party = create_party(
        &party_repo,
        "Outsider",
        "outsider@example.com",
        vec![DealRole::Supplier],
    )
    .await;

    let err = SignAgreement::new(deal_repo, party_repo, agreement_repo)
        .execute(SignAgreementCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: outsider_party,
            deal_id,
            signature_type: SignatureType::DigitalAttestation,
            ip_address: None,
        })
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Forbidden));
}

#[tokio::test]
async fn sign_agreement_marks_signed_after_three_signatures() {
    let (party_repo, deal_repo, agreement_repo, deal_id, supplier, consumer, enhancer) =
        locked_three_party_deal().await;
    accept_mandatory_term(&deal_repo, &party_repo, deal_id, supplier, consumer).await;

    GenerateAgreement::new(
        deal_repo.clone(),
        party_repo.clone(),
        agreement_repo.clone(),
    )
    .execute(GenerateAgreementCommand {
        actor_user_id: actor_user_id(),
        actor_party_id: supplier,
        deal_id,
    })
    .await
    .unwrap();

    let check_repo = agreement_repo.clone();
    let sign = SignAgreement::new(deal_repo, party_repo, agreement_repo);

    for party_id in [supplier, consumer, enhancer] {
        sign.execute(SignAgreementCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: party_id,
            deal_id,
            signature_type: SignatureType::DigitalAttestation,
            ip_address: None,
        })
        .await
        .unwrap();
    }

    let final_agreement = check_repo.find_by_deal_id(deal_id).await.unwrap().unwrap();
    assert_eq!(final_agreement.agreement_status, AgreementStatus::Signed);
}

#[tokio::test]
async fn get_agreement_visible_to_participant_hidden_from_outsider() {
    let (party_repo, deal_repo, agreement_repo, deal_id, supplier, consumer, _enhancer) =
        locked_three_party_deal().await;
    accept_mandatory_term(&deal_repo, &party_repo, deal_id, supplier, consumer).await;

    GenerateAgreement::new(
        deal_repo.clone(),
        party_repo.clone(),
        agreement_repo.clone(),
    )
    .execute(GenerateAgreementCommand {
        actor_user_id: actor_user_id(),
        actor_party_id: supplier,
        deal_id,
    })
    .await
    .unwrap();

    let get = GetAgreement::new(
        deal_repo.clone(),
        party_repo.clone(),
        agreement_repo.clone(),
    );

    let found = get
        .execute(deal_id, actor_user_id(), Some(supplier), false)
        .await
        .unwrap();
    assert_eq!(found.deal_id, deal_id);

    let hidden = get
        .execute(deal_id, outsider_user_id(), None, false)
        .await
        .unwrap_err();
    assert!(matches!(hidden, ApplicationError::DealNotFound));

    let admin_found = get
        .execute(deal_id, outsider_user_id(), None, true)
        .await
        .unwrap();
    assert_eq!(admin_found.deal_id, deal_id);
}

#[tokio::test]
async fn admin_update_agreement_updates_allowed_fields() {
    let (party_repo, deal_repo, agreement_repo, deal_id, supplier, consumer, _enhancer) =
        locked_three_party_deal().await;
    accept_mandatory_term(&deal_repo, &party_repo, deal_id, supplier, consumer).await;

    GenerateAgreement::new(
        deal_repo.clone(),
        party_repo.clone(),
        agreement_repo.clone(),
    )
    .execute(GenerateAgreementCommand {
        actor_user_id: actor_user_id(),
        actor_party_id: supplier,
        deal_id,
    })
    .await
    .unwrap();

    let admin_user = Uuid::now_v7();
    let result = AdminUpdateAgreement::new(deal_repo, agreement_repo)
        .execute(AdminUpdateAgreementCommand {
            admin_user_id: admin_user,
            deal_id,
            governing_law: Some("California".to_string()),
            dispute_resolution: Some("Arbitration".to_string()),
            effective_date: None,
            termination_date: None,
            auto_renew: Some(true),
            status: None,
            reason: Some("Added governing law".to_string()),
        })
        .await
        .unwrap();

    assert_eq!(result.governing_law, Some("California".to_string()));
    assert_eq!(result.dispute_resolution, Some("Arbitration".to_string()));
    assert!(result.auto_renew);
}

#[tokio::test]
async fn admin_update_agreement_can_terminate() {
    let (party_repo, deal_repo, agreement_repo, deal_id, supplier, consumer, _enhancer) =
        locked_three_party_deal().await;
    accept_mandatory_term(&deal_repo, &party_repo, deal_id, supplier, consumer).await;

    GenerateAgreement::new(
        deal_repo.clone(),
        party_repo.clone(),
        agreement_repo.clone(),
    )
    .execute(GenerateAgreementCommand {
        actor_user_id: actor_user_id(),
        actor_party_id: supplier,
        deal_id,
    })
    .await
    .unwrap();

    let result = AdminUpdateAgreement::new(deal_repo, agreement_repo)
        .execute(AdminUpdateAgreementCommand {
            admin_user_id: Uuid::now_v7(),
            deal_id,
            governing_law: None,
            dispute_resolution: None,
            effective_date: None,
            termination_date: None,
            auto_renew: None,
            status: Some(AgreementStatus::Terminated),
            reason: None,
        })
        .await
        .unwrap();

    assert_eq!(result.status, AgreementStatus::Terminated);
}

#[tokio::test]
async fn admin_update_agreement_rejects_illegal_status_change() {
    let (party_repo, deal_repo, agreement_repo, deal_id, supplier, consumer, _enhancer) =
        locked_three_party_deal().await;
    accept_mandatory_term(&deal_repo, &party_repo, deal_id, supplier, consumer).await;

    GenerateAgreement::new(
        deal_repo.clone(),
        party_repo.clone(),
        agreement_repo.clone(),
    )
    .execute(GenerateAgreementCommand {
        actor_user_id: actor_user_id(),
        actor_party_id: supplier,
        deal_id,
    })
    .await
    .unwrap();

    let err = AdminUpdateAgreement::new(deal_repo, agreement_repo)
        .execute(AdminUpdateAgreementCommand {
            admin_user_id: Uuid::now_v7(),
            deal_id,
            governing_law: None,
            dispute_resolution: None,
            effective_date: None,
            termination_date: None,
            auto_renew: None,
            status: Some(AgreementStatus::Signed),
            reason: None,
        })
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Validation(_)));
}

#[tokio::test]
async fn sign_agreement_fails_when_actor_not_party_member() {
    let (party_repo, deal_repo, agreement_repo, deal_id, supplier, consumer, _enhancer) =
        locked_three_party_deal().await;
    accept_mandatory_term(&deal_repo, &party_repo, deal_id, supplier, consumer).await;

    GenerateAgreement::new(
        deal_repo.clone(),
        party_repo.clone(),
        agreement_repo.clone(),
    )
    .execute(GenerateAgreementCommand {
        actor_user_id: actor_user_id(),
        actor_party_id: supplier,
        deal_id,
    })
    .await
    .unwrap();

    let err = SignAgreement::new(deal_repo, party_repo, agreement_repo)
        .execute(SignAgreementCommand {
            actor_user_id: outsider_user_id(),
            actor_party_id: supplier,
            deal_id,
            signature_type: SignatureType::DigitalAttestation,
            ip_address: None,
        })
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Forbidden));
}

#[tokio::test]
async fn sign_agreement_fails_when_no_agreement_exists() {
    let (party_repo, deal_repo, agreement_repo, deal_id, supplier, _consumer, _enhancer) =
        locked_three_party_deal().await;

    let err = SignAgreement::new(deal_repo, party_repo, agreement_repo)
        .execute(SignAgreementCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            deal_id,
            signature_type: SignatureType::DigitalAttestation,
            ip_address: None,
        })
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::DealNotFound));
}

#[tokio::test]
async fn sign_agreement_fails_when_agreement_not_signable() {
    let (party_repo, deal_repo, agreement_repo, deal_id, supplier, consumer, _enhancer) =
        locked_three_party_deal().await;
    accept_mandatory_term(&deal_repo, &party_repo, deal_id, supplier, consumer).await;

    GenerateAgreement::new(
        deal_repo.clone(),
        party_repo.clone(),
        agreement_repo.clone(),
    )
    .execute(GenerateAgreementCommand {
        actor_user_id: actor_user_id(),
        actor_party_id: supplier,
        deal_id,
    })
    .await
    .unwrap();

    let mut agreement = agreement_repo
        .find_by_deal_id(deal_id)
        .await
        .unwrap()
        .unwrap();
    agreement.mark_terminated();
    agreement_repo.update(&agreement).await.unwrap();

    let err = SignAgreement::new(deal_repo, party_repo, agreement_repo)
        .execute(SignAgreementCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            deal_id,
            signature_type: SignatureType::DigitalAttestation,
            ip_address: None,
        })
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Validation(_)));
}

#[tokio::test]
async fn sign_agreement_fails_when_party_already_signed() {
    let (party_repo, deal_repo, agreement_repo, deal_id, supplier, consumer, _enhancer) =
        locked_three_party_deal().await;
    accept_mandatory_term(&deal_repo, &party_repo, deal_id, supplier, consumer).await;

    GenerateAgreement::new(
        deal_repo.clone(),
        party_repo.clone(),
        agreement_repo.clone(),
    )
    .execute(GenerateAgreementCommand {
        actor_user_id: actor_user_id(),
        actor_party_id: supplier,
        deal_id,
    })
    .await
    .unwrap();

    let sign = SignAgreement::new(
        deal_repo.clone(),
        party_repo.clone(),
        agreement_repo.clone(),
    );
    sign.execute(SignAgreementCommand {
        actor_user_id: actor_user_id(),
        actor_party_id: supplier,
        deal_id,
        signature_type: SignatureType::DigitalAttestation,
        ip_address: None,
    })
    .await
    .unwrap();

    let err = sign
        .execute(SignAgreementCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: supplier,
            deal_id,
            signature_type: SignatureType::DigitalAttestation,
            ip_address: None,
        })
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Validation(_)));
}
