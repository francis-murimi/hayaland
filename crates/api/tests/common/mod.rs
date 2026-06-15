use api::websocket::SessionRegistry;
use api::AppState;
use application::agreements::{
    AdminUpdateAgreement, GenerateAgreement, GetAgreement, SignAgreement,
};
use application::chatrooms::{
    CreateChatRoom, GetChatRoom, JoinChatRoom, LeaveChatRoom, ListChatRooms,
    ManageChatRoomMembership, SoftDeleteChatRoom, UpdateChatRoom,
};
use application::deals::{
    AcceptTerm, CounterTerm, CreateDeal, ExecuteTransition, GetDeal, GetValueDistribution,
    ListDeals, ListTerms, ProposeTerm, RejectTerm, SetValueDistribution, SubmitDeal, UpdateDeal,
    ValidateDeal, WithdrawTerm,
};
use application::email::resend_verification::ResendVerificationEmail;
use application::email::verify_email::VerifyEmail;
use application::errors::ApplicationError;
use application::messages::{
    AdminBroadcast, EditMessage, GetMessage, GetUnreadCount, ListConversations, ListMessages,
    MarkRead, PinMessage, SendMessage, SoftDeleteMessage, ToggleReaction, UnpinMessage,
};
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
use application::ports::{EncryptionService, NoOpTrustScoreRecalculation, RealtimePublisher};
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
    AgreementRepository, ChatRoomRepository, DealRepository, EmailVerificationRepository,
    MessageRepository, MilestoneRepository, PartyRepository, PartyVerificationRepository,
    PasswordResetRepository, ReviewRepository, RoleRepository, UserRepository, WalletRepository,
};
use domain::services::ValidationConfig;
use infrastructure::{
    realtime::InMemoryRealtimePublisher,
    repositories::{
        PostgresAgreementRepository, PostgresChatRoomRepository, PostgresDealRepository,
        PostgresEmailVerificationRepository, PostgresMessageRepository,
        PostgresMilestoneRepository, PostgresPartyRepository, PostgresPartyVerificationRepository,
        PostgresPasswordResetRepository, PostgresReviewRepository, PostgresRoleRepository,
        PostgresUserRepository, PostgresWalletRepository,
    },
    security::{Argon2PasswordHasher, JwtTokenService, MessageEncryptionService},
};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

struct TestTokenService {
    secret: String,
}

#[async_trait]
impl TokenGenerator for TestTokenService {
    async fn generate(&self, ctx: &AuthContext) -> Result<String, ApplicationError> {
        JwtTokenService::new(self.secret.clone(), 86400)
            .generate(ctx)
            .await
    }
}

#[async_trait]
impl TokenVerifier for TestTokenService {
    async fn verify(&self, token: &str) -> Result<AuthContext, ApplicationError> {
        JwtTokenService::new(self.secret.clone(), 86400)
            .verify(token)
            .await
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

pub async fn build_state(pool: PgPool) -> AppState {
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
    let review_repo: Arc<dyn ReviewRepository> =
        Arc::new(PostgresReviewRepository::new(pool.clone()));
    let message_repo: Arc<dyn MessageRepository> =
        Arc::new(PostgresMessageRepository::new(pool.clone()));
    let chat_room_repo: Arc<dyn ChatRoomRepository> =
        Arc::new(PostgresChatRoomRepository::new(pool.clone()));
    let party_verification_repo: Arc<dyn PartyVerificationRepository> =
        Arc::new(PostgresPartyVerificationRepository::new(pool));
    let hasher: Arc<dyn PasswordHasher> = Arc::new(Argon2PasswordHasher);
    let token_service: Arc<dyn TokenVerifier> = Arc::new(TestTokenService {
        secret: "test-secret".to_string(),
    });
    let token_generator: Arc<dyn TokenGenerator> = Arc::new(TestTokenService {
        secret: "test-secret".to_string(),
    });
    const TEST_MESSAGE_KEY: &str = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=";
    let encryption_service: Arc<dyn EncryptionService> =
        Arc::new(MessageEncryptionService::from_base64(TEST_MESSAGE_KEY).unwrap());
    let realtime_publisher: Arc<dyn RealtimePublisher> = Arc::new(InMemoryRealtimePublisher::new());

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
            Arc::new(NoOpTrustScoreRecalculation),
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
        submit_verification: application::verifications::SubmitVerification::new(
            party_verification_repo.clone(),
            party_repo.clone(),
        ),
        list_party_verifications: application::verifications::ListPartyVerifications::new(
            party_verification_repo.clone(),
            party_repo.clone(),
        ),
        get_verification_status: application::verifications::GetVerificationStatus::new(
            party_verification_repo.clone(),
            party_repo.clone(),
        ),
        approve_verification: application::verifications::ApproveVerification::new(
            party_verification_repo.clone(),
            party_repo.clone(),
            Arc::new(NoOpTrustScoreRecalculation),
        ),
        reject_verification: application::verifications::RejectVerification::new(
            party_verification_repo.clone(),
            party_repo.clone(),
            Arc::new(NoOpTrustScoreRecalculation),
        ),
        revoke_verification: application::verifications::RevokeVerification::new(
            party_verification_repo.clone(),
            party_repo.clone(),
            Arc::new(NoOpTrustScoreRecalculation),
        ),
        list_admin_verifications: application::verifications::ListAdminVerifications::new(
            party_verification_repo.clone(),
        ),
        send_message: SendMessage::new(
            message_repo.clone(),
            party_repo.clone(),
            deal_repo.clone(),
            chat_room_repo.clone(),
            encryption_service.clone(),
            realtime_publisher.clone(),
        ),
        edit_message: EditMessage::new(
            message_repo.clone(),
            encryption_service.clone(),
            realtime_publisher.clone(),
        ),
        delete_message: SoftDeleteMessage::new(
            message_repo.clone(),
            encryption_service.clone(),
            realtime_publisher.clone(),
        ),
        get_message: GetMessage::new(
            message_repo.clone(),
            party_repo.clone(),
            deal_repo.clone(),
            chat_room_repo.clone(),
            encryption_service.clone(),
        ),
        list_messages: ListMessages::new(
            message_repo.clone(),
            party_repo.clone(),
            deal_repo.clone(),
            chat_room_repo.clone(),
            encryption_service.clone(),
        ),
        list_conversations: ListConversations::new(message_repo.clone()),
        mark_read: MarkRead::new(
            message_repo.clone(),
            party_repo.clone(),
            deal_repo.clone(),
            chat_room_repo.clone(),
            realtime_publisher.clone(),
        ),
        react: ToggleReaction::new(
            message_repo.clone(),
            party_repo.clone(),
            deal_repo.clone(),
            chat_room_repo.clone(),
            realtime_publisher.clone(),
        ),
        get_unread_count: GetUnreadCount::new(message_repo.clone()),
        pin_message: PinMessage::new(
            message_repo.clone(),
            party_repo.clone(),
            deal_repo.clone(),
            chat_room_repo.clone(),
            encryption_service.clone(),
        ),
        unpin_message: UnpinMessage::new(
            message_repo.clone(),
            party_repo.clone(),
            deal_repo.clone(),
            chat_room_repo.clone(),
            encryption_service.clone(),
        ),
        admin_broadcast: AdminBroadcast::new(
            message_repo.clone(),
            repo.clone(),
            party_repo.clone(),
            encryption_service.clone(),
            realtime_publisher.clone(),
        ),
        create_chat_room: CreateChatRoom::new(chat_room_repo.clone(), message_repo.clone()),
        update_chat_room: UpdateChatRoom::new(chat_room_repo.clone()),
        delete_chat_room: SoftDeleteChatRoom::new(
            chat_room_repo.clone(),
            realtime_publisher.clone(),
        ),
        get_chat_room: GetChatRoom::new(chat_room_repo.clone()),
        list_chat_rooms: ListChatRooms::new(chat_room_repo.clone(), party_repo.clone()),
        join_chat_room: JoinChatRoom::new(chat_room_repo.clone(), party_repo.clone()),
        leave_chat_room: LeaveChatRoom::new(chat_room_repo.clone()),
        manage_chat_room_membership: ManageChatRoomMembership::new(chat_room_repo.clone()),
        encryption_service,
        realtime_publisher,
        message_repository: message_repo,
        chat_room_repository: chat_room_repo,
        websocket_registry: SessionRegistry::new(),
        token_validator: token_service,
    }
}

pub async fn auth_token(user_id: Uuid, scopes: Vec<String>) -> String {
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

pub async fn create_user(pool: &PgPool, email: &str) -> Uuid {
    let user_id = Uuid::now_v7();
    sqlx::query!(
        r#"
        INSERT INTO users (id, email, username, password_hash, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        user_id,
        email,
        email.replace(['@', '.'], "_"),
        "hashed:password123",
        time::OffsetDateTime::now_utc(),
        time::OffsetDateTime::now_utc()
    )
    .execute(pool)
    .await
    .unwrap();
    user_id
}

#[allow(dead_code)]
pub async fn create_active_user(pool: &PgPool, email: &str) -> Uuid {
    let user_id = create_user(pool, email).await;
    sqlx::query!("UPDATE users SET is_active = true WHERE id = $1", user_id)
        .execute(pool)
        .await
        .unwrap();
    user_id
}
