use crate::errors::ApplicationError;
use crate::parties::dto::CreatePartyCommand;
use crate::parties::CreateParty;
use crate::ports::NoOpTrustScoreRecalculation;
use crate::test_helpers::{FakePartyRepo, FakePartyVerificationRepo};
use crate::verifications::dto::{
    AdminVerificationListQuery, ApproveVerificationCommand, GetVerificationStatusQuery,
    ListPartyVerificationsQuery, RejectVerificationCommand, RevokeVerificationCommand,
    SubmitVerificationCommand,
};
use crate::verifications::{
    ApproveVerification, GetVerificationStatus, ListAdminVerifications, ListPartyVerifications,
    RejectVerification, RevokeVerification, SubmitVerification,
};
use domain::entities::{PartyType, VerificationStatus};
use domain::repositories::PartyRepository;
use std::sync::Arc;
use uuid::Uuid;

fn actor_user_id() -> Uuid {
    Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap()
}

fn other_user_id() -> Uuid {
    Uuid::parse_str("99999999-9999-9999-9999-999999999999").unwrap()
}

async fn create_party(party_repo: &Arc<FakePartyRepo>, display_name: &str, email: &str) -> Uuid {
    let use_case = CreateParty::new(party_repo.clone());
    use_case
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
            roles: vec![],
        })
        .await
        .unwrap()
        .id
}

async fn setup() -> (Arc<FakePartyRepo>, Arc<FakePartyVerificationRepo>, Uuid) {
    let party_repo = Arc::new(FakePartyRepo::default());
    let verification_repo = Arc::new(FakePartyVerificationRepo::default());
    let party_id = create_party(&party_repo, "Test Party", "test@example.com").await;

    party_repo
        .add_membership(&domain::entities::UserPartyMembership::new(
            Uuid::now_v7(),
            actor_user_id(),
            party_id,
            domain::entities::PartyMembershipRole::Owner,
        ))
        .await
        .unwrap();

    (party_repo, verification_repo, party_id)
}

#[tokio::test]
async fn submit_verification_happy_path() {
    let (party_repo, verification_repo, party_id) = setup().await;
    let use_case = SubmitVerification::new(verification_repo, party_repo);

    let result = use_case
        .execute(SubmitVerificationCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: party_id,
            target_party_id: party_id,
            is_admin: false,
            verification_type: "GOVERNMENT_ID".to_string(),
            evidence_urls: vec!["url1".to_string()],
            notes: Some("please review".to_string()),
        })
        .await
        .unwrap();

    assert_eq!(result.party_id, party_id);
    assert_eq!(result.verification_type, "GOVERNMENT_ID");
    assert_eq!(result.status, "PENDING");
    assert_eq!(result.points, 30);
    assert_eq!(result.evidence_urls, vec!["url1".to_string()]);
}

#[tokio::test]
async fn submit_verification_rejects_missing_evidence_for_admin_reviewed_types() {
    let (party_repo, verification_repo, party_id) = setup().await;
    let use_case = SubmitVerification::new(verification_repo, party_repo);

    let err = use_case
        .execute(SubmitVerificationCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: party_id,
            target_party_id: party_id,
            is_admin: false,
            verification_type: "GOVERNMENT_ID".to_string(),
            evidence_urls: vec![],
            notes: None,
        })
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Validation(_)));
}

#[tokio::test]
async fn submit_verification_allows_no_evidence_for_automated_types() {
    let (party_repo, verification_repo, party_id) = setup().await;
    let use_case = SubmitVerification::new(verification_repo, party_repo);

    let result = use_case
        .execute(SubmitVerificationCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: party_id,
            target_party_id: party_id,
            is_admin: false,
            verification_type: "EMAIL".to_string(),
            evidence_urls: vec![],
            notes: None,
        })
        .await
        .unwrap();

    assert_eq!(result.verification_type, "EMAIL");
}

#[tokio::test]
async fn submit_verification_rejects_duplicate_active_type() {
    let (party_repo, verification_repo, party_id) = setup().await;
    let use_case = SubmitVerification::new(verification_repo.clone(), party_repo.clone());

    let cmd = SubmitVerificationCommand {
        actor_user_id: actor_user_id(),
        actor_party_id: party_id,
        target_party_id: party_id,
        is_admin: false,
        verification_type: "GOVERNMENT_ID".to_string(),
        evidence_urls: vec!["url1".to_string()],
        notes: None,
    };

    assert!(use_case.execute(cmd.clone()).await.is_ok());
    let err = use_case.execute(cmd).await.unwrap_err();
    assert!(matches!(err, ApplicationError::DuplicateVerification));
}

#[tokio::test]
async fn submit_verification_rejects_non_member() {
    let (party_repo, verification_repo, party_id) = setup().await;
    let use_case = SubmitVerification::new(verification_repo, party_repo);

    let err = use_case
        .execute(SubmitVerificationCommand {
            actor_user_id: other_user_id(),
            actor_party_id: party_id,
            target_party_id: party_id,
            is_admin: false,
            verification_type: "GOVERNMENT_ID".to_string(),
            evidence_urls: vec!["url1".to_string()],
            notes: None,
        })
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Forbidden));
}

#[tokio::test]
async fn submit_verification_rejects_invalid_type() {
    let (party_repo, verification_repo, party_id) = setup().await;
    let use_case = SubmitVerification::new(verification_repo, party_repo);

    let err = use_case
        .execute(SubmitVerificationCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: party_id,
            target_party_id: party_id,
            is_admin: false,
            verification_type: "UNKNOWN".to_string(),
            evidence_urls: vec!["url1".to_string()],
            notes: None,
        })
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Validation(_)));
}

#[tokio::test]
async fn approve_verification_updates_party_status_and_triggers_recalc() {
    let (party_repo, verification_repo, party_id) = setup().await;
    let submit = SubmitVerification::new(verification_repo.clone(), party_repo.clone());
    let approve = ApproveVerification::new(
        verification_repo.clone(),
        party_repo.clone(),
        Arc::new(NoOpTrustScoreRecalculation),
    );

    let submitted = submit
        .execute(SubmitVerificationCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: party_id,
            target_party_id: party_id,
            is_admin: false,
            verification_type: "GOVERNMENT_ID".to_string(),
            evidence_urls: vec!["url1".to_string()],
            notes: None,
        })
        .await
        .unwrap();

    let result = approve
        .execute(ApproveVerificationCommand {
            actor_user_id: actor_user_id(),
            verification_id: submitted.id,
            review_notes: Some("approved".to_string()),
        })
        .await
        .unwrap();

    assert_eq!(result.status, "APPROVED");

    let party = party_repo.find_by_id(party_id).await.unwrap().unwrap();
    assert_eq!(party.verification_status, VerificationStatus::Verified);
}

#[tokio::test]
async fn reject_verification_keeps_party_unverified() {
    let (party_repo, verification_repo, party_id) = setup().await;
    let submit = SubmitVerification::new(verification_repo.clone(), party_repo.clone());
    let reject = RejectVerification::new(
        verification_repo.clone(),
        party_repo.clone(),
        Arc::new(NoOpTrustScoreRecalculation),
    );

    let submitted = submit
        .execute(SubmitVerificationCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: party_id,
            target_party_id: party_id,
            is_admin: false,
            verification_type: "GOVERNMENT_ID".to_string(),
            evidence_urls: vec!["url1".to_string()],
            notes: None,
        })
        .await
        .unwrap();

    let result = reject
        .execute(RejectVerificationCommand {
            actor_user_id: actor_user_id(),
            verification_id: submitted.id,
            reason: "expired document".to_string(),
            review_notes: None,
        })
        .await
        .unwrap();

    assert_eq!(result.status, "REJECTED");
    assert_eq!(result.rejection_reason.as_deref(), Some("expired document"));

    let party = party_repo.find_by_id(party_id).await.unwrap().unwrap();
    assert_eq!(party.verification_status, VerificationStatus::Unverified);
}

#[tokio::test]
async fn revoke_verification_reduces_level() {
    let (party_repo, verification_repo, party_id) = setup().await;
    let submit = SubmitVerification::new(verification_repo.clone(), party_repo.clone());
    let approve = ApproveVerification::new(
        verification_repo.clone(),
        party_repo.clone(),
        Arc::new(NoOpTrustScoreRecalculation),
    );
    let revoke = RevokeVerification::new(
        verification_repo.clone(),
        party_repo.clone(),
        Arc::new(NoOpTrustScoreRecalculation),
    );

    let submitted = submit
        .execute(SubmitVerificationCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: party_id,
            target_party_id: party_id,
            is_admin: false,
            verification_type: "GOVERNMENT_ID".to_string(),
            evidence_urls: vec!["url1".to_string()],
            notes: None,
        })
        .await
        .unwrap();

    approve
        .execute(ApproveVerificationCommand {
            actor_user_id: actor_user_id(),
            verification_id: submitted.id,
            review_notes: None,
        })
        .await
        .unwrap();

    let result = revoke
        .execute(RevokeVerificationCommand {
            actor_user_id: actor_user_id(),
            verification_id: submitted.id,
            reason: "fraud".to_string(),
            review_notes: None,
        })
        .await
        .unwrap();

    assert_eq!(result.status, "REVOKED");

    let party = party_repo.find_by_id(party_id).await.unwrap().unwrap();
    assert_eq!(party.verification_status, VerificationStatus::Unverified);
}

#[tokio::test]
async fn list_party_verifications_includes_evidence_for_member() {
    let (party_repo, verification_repo, party_id) = setup().await;
    let submit = SubmitVerification::new(verification_repo.clone(), party_repo.clone());
    let list = ListPartyVerifications::new(verification_repo, party_repo);

    submit
        .execute(SubmitVerificationCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: party_id,
            target_party_id: party_id,
            is_admin: false,
            verification_type: "GOVERNMENT_ID".to_string(),
            evidence_urls: vec!["secret".to_string()],
            notes: None,
        })
        .await
        .unwrap();

    let result = list
        .execute(
            party_id,
            ListPartyVerificationsQuery {
                actor_user_id: actor_user_id(),
                actor_party_id: party_id,
                is_admin: false,
            },
        )
        .await
        .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].evidence_urls, vec!["secret".to_string()]);
}

#[tokio::test]
async fn list_party_verifications_hides_evidence_from_outsider() {
    let (party_repo, verification_repo, party_id) = setup().await;
    let submit = SubmitVerification::new(verification_repo.clone(), party_repo.clone());
    let list = ListPartyVerifications::new(verification_repo, party_repo);

    submit
        .execute(SubmitVerificationCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: party_id,
            target_party_id: party_id,
            is_admin: false,
            verification_type: "GOVERNMENT_ID".to_string(),
            evidence_urls: vec!["secret".to_string()],
            notes: None,
        })
        .await
        .unwrap();

    let err = list
        .execute(
            party_id,
            ListPartyVerificationsQuery {
                actor_user_id: other_user_id(),
                actor_party_id: party_id,
                is_admin: false,
            },
        )
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Forbidden));
}

#[tokio::test]
async fn get_verification_status_reports_level() {
    let (party_repo, verification_repo, party_id) = setup().await;
    let submit = SubmitVerification::new(verification_repo.clone(), party_repo.clone());
    let approve = ApproveVerification::new(
        verification_repo.clone(),
        party_repo.clone(),
        Arc::new(NoOpTrustScoreRecalculation),
    );
    let status = GetVerificationStatus::new(verification_repo, party_repo);

    let submitted = submit
        .execute(SubmitVerificationCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: party_id,
            target_party_id: party_id,
            is_admin: false,
            verification_type: "EMAIL".to_string(),
            evidence_urls: vec![],
            notes: None,
        })
        .await
        .unwrap();

    approve
        .execute(ApproveVerificationCommand {
            actor_user_id: actor_user_id(),
            verification_id: submitted.id,
            review_notes: None,
        })
        .await
        .unwrap();

    let result = status
        .execute(
            party_id,
            GetVerificationStatusQuery {
                actor_user_id: actor_user_id(),
                actor_party_id: party_id,
                is_admin: false,
            },
        )
        .await
        .unwrap();

    assert_eq!(result.effective_points, 10);
    assert_eq!(result.verification_level, 1);
    assert_eq!(result.approved_count, 1);
    assert_eq!(result.next_level_points, 25);
}

#[tokio::test]
async fn list_admin_verifications_filters_by_status() {
    let (party_repo, verification_repo, party_id) = setup().await;
    let submit = SubmitVerification::new(verification_repo.clone(), party_repo);
    let list = ListAdminVerifications::new(verification_repo);

    submit
        .execute(SubmitVerificationCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: party_id,
            target_party_id: party_id,
            is_admin: false,
            verification_type: "GOVERNMENT_ID".to_string(),
            evidence_urls: vec!["url".to_string()],
            notes: None,
        })
        .await
        .unwrap();

    let all = list
        .execute(AdminVerificationListQuery {
            limit: 10,
            offset: 0,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(all.total, 1);

    let pending = list
        .execute(AdminVerificationListQuery {
            status: Some("PENDING".to_string()),
            limit: 10,
            offset: 0,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(pending.total, 1);

    let approved = list
        .execute(AdminVerificationListQuery {
            status: Some("APPROVED".to_string()),
            limit: 10,
            offset: 0,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(approved.total, 0);
}

#[tokio::test]
async fn approve_non_pending_fails() {
    let (party_repo, verification_repo, party_id) = setup().await;
    let submit = SubmitVerification::new(verification_repo.clone(), party_repo.clone());
    let approve = ApproveVerification::new(
        verification_repo.clone(),
        party_repo.clone(),
        Arc::new(NoOpTrustScoreRecalculation),
    );
    let reject = RejectVerification::new(
        verification_repo.clone(),
        party_repo.clone(),
        Arc::new(NoOpTrustScoreRecalculation),
    );

    let submitted = submit
        .execute(SubmitVerificationCommand {
            actor_user_id: actor_user_id(),
            actor_party_id: party_id,
            target_party_id: party_id,
            is_admin: false,
            verification_type: "GOVERNMENT_ID".to_string(),
            evidence_urls: vec!["url".to_string()],
            notes: None,
        })
        .await
        .unwrap();

    reject
        .execute(RejectVerificationCommand {
            actor_user_id: actor_user_id(),
            verification_id: submitted.id,
            reason: "bad".to_string(),
            review_notes: None,
        })
        .await
        .unwrap();

    let err = approve
        .execute(ApproveVerificationCommand {
            actor_user_id: actor_user_id(),
            verification_id: submitted.id,
            review_notes: None,
        })
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Validation(_)));
}
