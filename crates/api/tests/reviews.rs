use actix_web::http::header;
use actix_web::{http::StatusCode, test, web::Data, App};
use api::routes;
use api::AppState;
use application::agreements::{
    AdminUpdateAgreement, GenerateAgreement, GetAgreement, SignAgreement,
};
use application::deals::{
    AcceptTerm, CounterTerm, CreateDeal, ExecuteTransition, GetDeal, GetValueDistribution,
    ListDeals, ListTerms, ProposeTerm, RejectTerm, SetValueDistribution, SubmitDeal, UpdateDeal,
    ValidateDeal, WithdrawTerm,
};
use application::email::resend_verification::ResendVerificationEmail;
use application::email::verify_email::VerifyEmail;
use application::errors::ApplicationError;
use application::milestones::{
    CompleteMilestone, CreateMilestone, DeleteMilestone, GetDealProgress, ListMilestones,
    StartMilestone, UpdateMilestone, VerifyMilestone,
};
use application::parties::{
    AddPartyRole, CreateParty, GetParty, ListMyParties, ListPartyRoles, RemovePartyRole,
    SearchParties, SoftDeleteParty, UpdateParty,
};
use application::password_reset::request_password_reset::RequestPasswordReset;
use application::password_reset::reset_password::ResetPassword;
use application::payments::{
    ApproveTransaction, CreateWallet, DeductFee, DepositPoints, GetDealWallet, GetTransaction,
    GetWallet, HoldEscrow, ListDealTransactions, ListPendingApprovals, ListWalletTransactions,
    RecordAdjustment, ReleaseEscrow, WithdrawPoints,
};
use application::roles::assign_user_roles::AssignUserRoles;
use application::roles::list_roles::ListRoles;
use application::roles::update_role_scopes::UpdateRoleScopes;
use application::users::authenticate_user::AuthenticateUser;
use application::users::create_user::{CreateUser, PasswordHasher};
use application::users::deactivate_user::DeactivateUser;
use application::users::get_user::GetUser;
use application::users::list_users::ListUsers;
use application::users::token::{AuthContext, TokenGenerator, TokenVerifier};
use application::users::update_user::UpdateUser;
use async_trait::async_trait;

use domain::repositories::{
    AgreementRepository, DealRepository, EmailVerificationRepository, MilestoneRepository,
    PartyRepository, PasswordResetRepository, ReviewRepository, RoleRepository, UserRepository,
    WalletRepository,
};
use domain::services::ValidationConfig;
use infrastructure::{
    repositories::{
        PostgresAgreementRepository, PostgresDealRepository, PostgresEmailVerificationRepository,
        PostgresMilestoneRepository, PostgresPartyRepository, PostgresPasswordResetRepository,
        PostgresReviewRepository, PostgresRoleRepository, PostgresUserRepository,
        PostgresWalletRepository,
    },
    security::{Argon2PasswordHasher, JwtTokenService},
};
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

struct TestTokenService {
    secret: String,
}

#[async_trait]
impl TokenGenerator for TestTokenService {
    async fn generate(&self, ctx: &AuthContext) -> Result<String, ApplicationError> {
        let inner = JwtTokenService::new(self.secret.clone(), 86400);
        inner.generate(ctx).await
    }
}

#[async_trait]
impl TokenVerifier for TestTokenService {
    async fn verify(&self, token: &str) -> Result<AuthContext, ApplicationError> {
        let inner = JwtTokenService::new(self.secret.clone(), 86400);
        inner.verify(token).await
    }
}

async fn build_state(pool: PgPool) -> AppState {
    let repo: Arc<dyn UserRepository> = Arc::new(PostgresUserRepository::new(pool.clone()));
    let verification_repo: Arc<dyn EmailVerificationRepository> =
        Arc::new(PostgresEmailVerificationRepository::new(pool.clone()));
    let password_reset_repo: Arc<dyn PasswordResetRepository> =
        Arc::new(PostgresPasswordResetRepository::new(pool.clone()));
    let role_repo: Arc<dyn RoleRepository> = Arc::new(PostgresRoleRepository::new(pool.clone()));
    let party_repo: Arc<dyn PartyRepository> = Arc::new(PostgresPartyRepository::new(pool.clone()));
    let deal_repo: Arc<dyn DealRepository> = Arc::new(PostgresDealRepository::new(pool.clone()));
    let agreement_repo: Arc<dyn AgreementRepository> =
        Arc::new(PostgresAgreementRepository::new(pool.clone()));
    let wallet_repo: Arc<dyn WalletRepository> =
        Arc::new(PostgresWalletRepository::new(pool.clone()));
    let milestone_repo: Arc<dyn MilestoneRepository> =
        Arc::new(PostgresMilestoneRepository::new(pool.clone()));
    let review_repo: Arc<dyn ReviewRepository> = Arc::new(PostgresReviewRepository::new(pool));
    let hasher: Arc<dyn PasswordHasher> = Arc::new(Argon2PasswordHasher);
    let token_service: Arc<dyn TokenVerifier> = Arc::new(TestTokenService {
        secret: "test-secret".to_string(),
    });
    let token_generator: Arc<dyn TokenGenerator> = Arc::new(TestTokenService {
        secret: "test-secret".to_string(),
    });

    AppState {
        create_user: CreateUser::new(
            repo.clone(),
            verification_repo.clone(),
            Arc::new(FakeEmailQueue),
            hasher.clone(),
            "https://app.hayaland.local".to_string(),
            86400,
        ),
        get_user: GetUser::new(repo.clone()),
        list_users: ListUsers::new(repo.clone()),
        update_user: UpdateUser::new(repo.clone()),
        assign_user_roles: AssignUserRoles::new(repo.clone()),
        deactivate_user: DeactivateUser::new(repo.clone()),
        authenticate_user: AuthenticateUser::new(
            repo.clone(),
            role_repo.clone(),
            hasher.clone(),
            token_generator.clone(),
        ),
        verify_email: VerifyEmail::new(repo.clone(), verification_repo.clone()),
        resend_verification_email: ResendVerificationEmail::new(
            repo.clone(),
            verification_repo,
            Arc::new(FakeEmailQueue),
            "https://app.hayaland.local".to_string(),
            86400,
        ),
        request_password_reset: RequestPasswordReset::new(
            repo.clone(),
            password_reset_repo.clone(),
            Arc::new(FakeEmailQueue),
            "https://app.hayaland.local".to_string(),
            3600,
        ),
        reset_password: ResetPassword::new(repo.clone(), password_reset_repo, hasher),
        list_roles: ListRoles::new(role_repo.clone()),
        update_role_scopes: UpdateRoleScopes::new(role_repo),
        create_party: CreateParty::new_with_wallet(party_repo.clone(), wallet_repo.clone()),
        get_party: GetParty::new(party_repo.clone()),
        list_my_parties: ListMyParties::new(party_repo.clone()),
        search_parties: SearchParties::new(party_repo.clone()),
        update_party: UpdateParty::new(party_repo.clone()),
        delete_party: SoftDeleteParty::new(party_repo.clone()),
        add_party_role: AddPartyRole::new(party_repo.clone()),
        remove_party_role: RemovePartyRole::new(party_repo.clone()),
        list_party_roles: ListPartyRoles::new(party_repo.clone()),
        create_deal: CreateDeal::new(deal_repo.clone(), party_repo.clone()),
        get_deal: GetDeal::new(deal_repo.clone(), party_repo.clone()),
        list_deals: ListDeals::new(deal_repo.clone(), party_repo.clone()),
        update_deal: UpdateDeal::new(deal_repo.clone(), party_repo.clone()),
        submit_deal: SubmitDeal::new(
            deal_repo.clone(),
            party_repo.clone(),
            ValidationConfig::default(),
        ),
        execute_transition: ExecuteTransition::new_with_reviews(
            deal_repo.clone(),
            party_repo.clone(),
            agreement_repo.clone(),
            milestone_repo.clone(),
            review_repo.clone(),
            ValidationConfig::default(),
        ),
        propose_term: ProposeTerm::new(deal_repo.clone(), party_repo.clone()),
        counter_term: CounterTerm::new(deal_repo.clone(), party_repo.clone()),
        accept_term: AcceptTerm::new(deal_repo.clone(), party_repo.clone()),
        reject_term: RejectTerm::new(deal_repo.clone(), party_repo.clone()),
        withdraw_term: WithdrawTerm::new(deal_repo.clone(), party_repo.clone()),
        list_terms: ListTerms::new(deal_repo.clone(), party_repo.clone()),
        set_value_distribution: SetValueDistribution::new(deal_repo.clone(), party_repo.clone()),
        get_value_distribution: GetValueDistribution::new(deal_repo.clone(), party_repo.clone()),
        validate_deal: ValidateDeal::new(
            deal_repo.clone(),
            party_repo.clone(),
            ValidationConfig::default(),
        ),
        generate_agreement: GenerateAgreement::new(
            deal_repo.clone(),
            party_repo.clone(),
            agreement_repo.clone(),
        ),
        get_agreement: GetAgreement::new(
            deal_repo.clone(),
            party_repo.clone(),
            agreement_repo.clone(),
        ),
        sign_agreement: SignAgreement::new(
            deal_repo.clone(),
            party_repo.clone(),
            agreement_repo.clone(),
        ),
        admin_update_agreement: AdminUpdateAgreement::new(deal_repo.clone(), agreement_repo),
        create_wallet: CreateWallet::new(wallet_repo.clone()),
        deposit_points: DepositPoints::new(
            party_repo.clone(),
            deal_repo.clone(),
            wallet_repo.clone(),
        ),
        withdraw_points: WithdrawPoints::new(
            party_repo.clone(),
            deal_repo.clone(),
            wallet_repo.clone(),
        ),
        get_wallet: GetWallet::new(party_repo.clone(), wallet_repo.clone()),
        get_deal_wallet: GetDealWallet::new(
            party_repo.clone(),
            deal_repo.clone(),
            wallet_repo.clone(),
        ),
        hold_escrow: HoldEscrow::new(party_repo.clone(), deal_repo.clone(), wallet_repo.clone()),
        release_escrow: ReleaseEscrow::new(
            party_repo.clone(),
            deal_repo.clone(),
            wallet_repo.clone(),
        ),
        deduct_fee: DeductFee::new(party_repo.clone(), deal_repo.clone(), wallet_repo.clone()),
        record_adjustment: RecordAdjustment::new(
            party_repo.clone(),
            deal_repo.clone(),
            wallet_repo.clone(),
        ),
        list_wallet_transactions: ListWalletTransactions::new(
            party_repo.clone(),
            wallet_repo.clone(),
        ),
        list_deal_transactions: ListDealTransactions::new(
            party_repo.clone(),
            deal_repo.clone(),
            wallet_repo.clone(),
        ),
        approve_transaction: ApproveTransaction::new(party_repo.clone(), wallet_repo.clone()),
        list_pending_approvals: ListPendingApprovals::new(party_repo.clone(), wallet_repo.clone()),
        get_transaction: GetTransaction::new(party_repo.clone(), wallet_repo.clone()),
        create_milestone: CreateMilestone::new(
            party_repo.clone(),
            deal_repo.clone(),
            milestone_repo.clone(),
        ),
        list_milestones: ListMilestones::new(
            party_repo.clone(),
            deal_repo.clone(),
            milestone_repo.clone(),
        ),
        get_deal_progress: GetDealProgress::new(
            party_repo.clone(),
            deal_repo.clone(),
            milestone_repo.clone(),
        ),
        update_milestone: UpdateMilestone::new(
            party_repo.clone(),
            deal_repo.clone(),
            milestone_repo.clone(),
        ),
        delete_milestone: DeleteMilestone::new(
            party_repo.clone(),
            deal_repo.clone(),
            milestone_repo.clone(),
        ),
        start_milestone: StartMilestone::new(
            party_repo.clone(),
            deal_repo.clone(),
            milestone_repo.clone(),
        ),
        complete_milestone: CompleteMilestone::new(
            party_repo.clone(),
            deal_repo.clone(),
            milestone_repo.clone(),
        ),
        verify_milestone: VerifyMilestone::new(
            party_repo.clone(),
            deal_repo.clone(),
            milestone_repo.clone(),
            wallet_repo.clone(),
        ),
        submit_review: application::reviews::SubmitReview::new(
            review_repo.clone(),
            deal_repo.clone(),
            party_repo.clone(),
            Arc::new(application::reviews::submit_review::NoOpTrustScoreRecalculation),
        ),
        list_deal_reviews: application::reviews::ListDealReviews::new(
            deal_repo.clone(),
            review_repo.clone(),
        ),
        list_party_reviews: application::reviews::ListPartyReviews::new(
            party_repo.clone(),
            review_repo.clone(),
        ),
        get_review: application::reviews::GetReview::new(deal_repo.clone(), review_repo.clone()),
        get_deal_review_status: application::reviews::GetDealReviewStatus::new(
            deal_repo.clone(),
            review_repo.clone(),
        ),
        hide_review: application::reviews::HideReview::new(review_repo.clone()),
        list_admin_reviews: application::reviews::ListAdminReviews::new(review_repo.clone()),
        token_validator: token_service,
    }
}

struct FakeEmailQueue;

#[async_trait]
impl application::email::queue::EmailQueue for FakeEmailQueue {
    async fn enqueue(
        &self,
        _item: application::email::queue::EmailQueueItem,
    ) -> Result<(), ApplicationError> {
        Ok(())
    }
}

async fn auth_token(
    _token_service: &Arc<dyn TokenVerifier>,
    user_id: Uuid,
    scopes: Vec<String>,
) -> String {
    let generator = JwtTokenService::new("test-secret".to_string(), 86400);
    generator
        .generate(&AuthContext {
            user_id,
            roles: vec!["user".to_string()],
            scopes,
        })
        .await
        .unwrap()
}

async fn setup_executing_deal(pool: &PgPool) -> (Uuid, Uuid, Uuid, Uuid, Uuid) {
    let deal_id = Uuid::now_v7();
    let supplier_user = Uuid::now_v7();
    let consumer_user = Uuid::now_v7();
    let supplier_party = Uuid::now_v7();
    let consumer_party = Uuid::now_v7();
    let category_id = Uuid::now_v7();

    sqlx::query!("INSERT INTO categories (id, category_name, category_code, category_type) VALUES ($1, $2, $3, $4)",
        category_id, "Test", "TEST", "DOMAIN"
    )
    .execute(pool)
    .await
    .unwrap();

    for (user_id, email) in [
        (supplier_user, "supplier@example.com"),
        (consumer_user, "consumer@example.com"),
    ] {
        sqlx::query!(
            r#"
            INSERT INTO users (id, email, username, password_hash, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            user_id,
            email,
            email.replace('@', "_"),
            "hashed:password123",
            time::OffsetDateTime::now_utc(),
            time::OffsetDateTime::now_utc()
        )
        .execute(pool)
        .await
        .unwrap();
    }

    for (party_id, email) in [
        (supplier_party, "supplier-party@example.com"),
        (consumer_party, "consumer-party@example.com"),
    ] {
        sqlx::query!(
            r#"
            INSERT INTO parties (
                id, party_type, display_name, email, verification_status, is_active, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            party_id,
            "ORGANIZATION",
            "Test Party",
            email,
            "UNVERIFIED",
            true,
            time::OffsetDateTime::now_utc(),
            time::OffsetDateTime::now_utc()
        )
        .execute(pool)
        .await
        .unwrap();
    }

    sqlx::query!(
        r#"
        INSERT INTO user_party_memberships (id, user_id, party_id, member_role, is_active, created_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        Uuid::now_v7(),
        supplier_user,
        supplier_party,
        "OWNER",
        true,
        time::OffsetDateTime::now_utc()
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query!(
        r#"
        INSERT INTO user_party_memberships (id, user_id, party_id, member_role, is_active, created_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        Uuid::now_v7(),
        consumer_user,
        consumer_party,
        "OWNER",
        true,
        time::OffsetDateTime::now_utc()
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query!(
        r#"
        INSERT INTO deals (
            id, deal_reference, deal_title, domain_category_id, initiator_party_id, initiator_role,
            deal_status, platform_fee_percentage, platform_fee_amount, is_public, current_state_entered_at,
            created_at, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        "#,
        deal_id,
        format!("DL-{}-0001", Uuid::now_v7()),
        "Test Deal",
        category_id,
        supplier_party,
        "SUPPLIER",
        "EXECUTING",
        Decimal::ZERO,
        Decimal::ZERO,
        false,
        time::OffsetDateTime::now_utc(),
        time::OffsetDateTime::now_utc(),
        time::OffsetDateTime::now_utc()
    )
    .execute(pool)
    .await
    .unwrap();

    for (party_id, role, is_initiator) in [
        (supplier_party, "SUPPLIER", true),
        (consumer_party, "CONSUMER", false),
    ] {
        sqlx::query!(
            r#"
            INSERT INTO deal_participations (
                id, deal_id, party_id, role, participation_status, is_initiator, invited_at, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            Uuid::now_v7(),
            deal_id,
            party_id,
            role,
            "ACCEPTED",
            is_initiator,
            time::OffsetDateTime::now_utc(),
            time::OffsetDateTime::now_utc()
        )
        .execute(pool)
        .await
        .unwrap();
    }

    (
        deal_id,
        supplier_user,
        consumer_user,
        supplier_party,
        consumer_party,
    )
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_review_returns_201(pool: PgPool) {
    let state = build_state(pool.clone()).await;
    let (deal_id, supplier_user, _, supplier_party, consumer_party) =
        setup_executing_deal(&pool).await;
    let token = auth_token(
        &state.token_validator,
        supplier_user,
        vec!["reviews:read".to_string(), "reviews:write".to_string()],
    )
    .await;

    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/reviews"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", supplier_party.to_string()))
        .set_json(serde_json::json!({
            "reviewedPartyId": consumer_party,
            "overallRating": 4,
            "reviewText": "Great partner"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["overallRating"], 4);
    assert_eq!(body["reviewerPartyId"], supplier_party.to_string());
    assert_eq!(body["reviewedPartyId"], consumer_party.to_string());
}

#[sqlx::test(migrations = "../../migrations")]
async fn duplicate_review_returns_conflict(pool: PgPool) {
    let state = build_state(pool.clone()).await;
    let (deal_id, supplier_user, _, supplier_party, consumer_party) =
        setup_executing_deal(&pool).await;
    let token = auth_token(
        &state.token_validator,
        supplier_user,
        vec!["reviews:read".to_string(), "reviews:write".to_string()],
    )
    .await;

    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let body = serde_json::json!({
        "reviewedPartyId": consumer_party,
        "overallRating": 4
    });

    for i in 0..2 {
        let req = test::TestRequest::post()
            .uri(&format!("/api/v1/deals/{deal_id}/reviews"))
            .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
            .insert_header(("X-Party-ID", supplier_party.to_string()))
            .set_json(&body)
            .to_request();
        let resp = test::call_service(&app, req).await;
        if i == 0 {
            assert_eq!(resp.status(), StatusCode::CREATED);
        } else {
            assert_eq!(resp.status(), StatusCode::CONFLICT);
        }
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn non_participant_cannot_list_deal_reviews(pool: PgPool) {
    let state = build_state(pool.clone()).await;
    let (deal_id, _, _consumer_user, _, _) = setup_executing_deal(&pool).await;

    // Create an unrelated user/party.
    let outsider_user = Uuid::now_v7();
    let outsider_party = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, email, username, password_hash, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6)",
        outsider_user,
        "outsider@example.com",
        "outsider",
        "hashed:password123",
        time::OffsetDateTime::now_utc(),
        time::OffsetDateTime::now_utc()
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO parties (id, party_type, display_name, email, verification_status, is_active, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        outsider_party,
        "ORGANIZATION",
        "Outsider",
        "outsider-party@example.com",
        "UNVERIFIED",
        true,
        time::OffsetDateTime::now_utc(),
        time::OffsetDateTime::now_utc()
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO user_party_memberships (id, user_id, party_id, member_role, is_active, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
        Uuid::now_v7(),
        outsider_user,
        outsider_party,
        "OWNER",
        true,
        time::OffsetDateTime::now_utc()
    )
    .execute(&pool)
    .await
    .unwrap();

    let token = auth_token(
        &state.token_validator,
        outsider_user,
        vec!["reviews:read".to_string()],
    )
    .await;

    let app = test::init_service(
        App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/deals/{deal_id}/reviews"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .insert_header(("X-Party-ID", outsider_party.to_string()))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../migrations")]
async fn admin_can_hide_review(pool: PgPool) {
    let state = build_state(pool.clone()).await;
    let (deal_id, supplier_user, _, supplier_party, consumer_party) =
        setup_executing_deal(&pool).await;
    let user_token = auth_token(
        &state.token_validator,
        supplier_user,
        vec!["reviews:read".to_string(), "reviews:write".to_string()],
    )
    .await;

    let app = test::init_service(
        App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/deals/{deal_id}/reviews"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {user_token}")))
        .insert_header(("X-Party-ID", supplier_party.to_string()))
        .set_json(serde_json::json!({
            "reviewedPartyId": consumer_party,
            "overallRating": 4,
            "reviewText": "needs moderation"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let review_id = body["id"].as_str().unwrap();

    let admin_user = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, email, username, password_hash, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6)",
        admin_user,
        "admin@example.com",
        "admin",
        "hashed:password123",
        time::OffsetDateTime::now_utc(),
        time::OffsetDateTime::now_utc()
    )
    .execute(&pool)
    .await
    .unwrap();
    let admin_token = auth_token(
        &state.token_validator,
        admin_user,
        vec!["admin:reviews".to_string()],
    )
    .await;

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/admin/reviews/{review_id}/hide"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {admin_token}")))
        .set_json(serde_json::json!({
            "platformResponse": "removed by admin"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/reviews/{review_id}"))
        .insert_header((header::AUTHORIZATION, format!("Bearer {user_token}")))
        .insert_header(("X-Party-ID", supplier_party.to_string()))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["isPublic"], false);
    assert!(body["reviewText"].is_null());
    assert_eq!(body["platformResponse"], "removed by admin");
}
