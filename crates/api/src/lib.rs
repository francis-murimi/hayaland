pub mod dto;
pub mod errors;
pub mod handlers;
pub mod middleware;
pub mod routes;
pub mod websocket;

use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use application::agreements::{
    AdminUpdateAgreement, GenerateAgreement, GetAgreement, SignAgreement,
};
use application::catalog::{
    AdminUpdateCatalogFlags, BindCatalogItemToDeal, ContactCatalogOwner, CreateEnhancement,
    CreateNeed, CreateResource, DeleteEnhancement, DeleteNeed, DeleteResource, GetEnhancement,
    GetNeed, GetResource, ListDealCatalogItems, ListEnhancements, ListNeeds, ListResources,
    UpdateEnhancement, UpdateNeed, UpdatePartyCatalogSettings, UpdateResource,
};
use application::deals::{
    AcceptTerm, CounterTerm, CreateDeal, ExecuteTransition, GetDeal, GetValueDistribution,
    ListDeals, ListTerms, ProposeTerm, RejectTerm, SetValueDistribution, SubmitDeal, UpdateDeal,
    ValidateDeal, WithdrawTerm,
};
use application::disputes::{
    EscalateDispute, GetDispute, ListAdminDisputes, ListDealDisputes, RaiseDispute, RejectDispute,
    ResolveDispute, RespondToDispute, SubmitEvidence,
};
use application::email::resend_verification::ResendVerificationEmail;
use application::email::verify_email::VerifyEmail;
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
use application::reviews::{
    GetDealReviewStatus, GetReview, HideReview, ListAdminReviews, ListDealReviews,
    ListPartyReviews, SubmitReview,
};
use application::roles::assign_user_roles::AssignUserRoles;
use application::roles::list_roles::ListRoles;
use application::roles::update_role_scopes::UpdateRoleScopes;
use application::trust_scores::{GetTrustScore, RecalculateTrustScore};
use application::users::authenticate_user::AuthenticateUser;
use application::users::create_user::CreateUser;
use application::users::deactivate_user::DeactivateUser;
use application::users::get_user::GetUser;
use application::users::list_users::ListUsers;
use application::users::token::TokenVerifier;
use application::users::update_user::UpdateUser;
use application::verifications::{
    ApproveVerification, GetVerificationStatus, ListAdminVerifications, ListPartyVerifications,
    RejectVerification, RevokeVerification, SubmitVerification,
};
use application::{
    chatrooms::{
        CreateChatRoom, GetChatRoom, JoinChatRoom, LeaveChatRoom, ListChatRooms,
        ManageChatRoomMembership, SoftDeleteChatRoom, UpdateChatRoom,
    },
    messages::{
        AdminBroadcast, EditMessage, GetMessage, GetUnreadCount, ListConversations, ListMessages,
        MarkRead, PinMessage, SendMessage, SoftDeleteMessage, ToggleReaction, UnpinMessage,
    },
    notifications::{
        AdminCreateTemplate, AdminDeleteTemplate, AdminGetTemplate, AdminListTemplates,
        AdminSendNotification, AdminUpdateTemplate, DeleteNotification, GetNotification,
        GetNotificationPreferences, GetUnreadCount as GetUnreadNotificationCount,
        ListNotifications, MarkAllNotificationsRead, MarkNotificationRead, SendNotification,
        UpdateNotificationPreferences,
    },
    ports::{EncryptionService, NotificationRealtimePublisher, RealtimePublisher},
};
use domain::repositories::{CatalogRepository, ChatRoomRepository, MessageRepository};
use sqlx::PgPool;
use std::net::TcpListener;
use std::sync::Arc;
use tracing_actix_web::TracingLogger;

/// All application services shared by handlers.
#[derive(Clone)]
pub struct AppState {
    pub create_user: CreateUser,
    pub get_user: GetUser,
    pub list_users: ListUsers,
    pub update_user: UpdateUser,
    pub assign_user_roles: AssignUserRoles,
    pub deactivate_user: DeactivateUser,
    pub authenticate_user: AuthenticateUser,
    pub verify_email: VerifyEmail,
    pub resend_verification_email: ResendVerificationEmail,
    pub request_password_reset: RequestPasswordReset,
    pub reset_password: ResetPassword,
    pub list_roles: ListRoles,
    pub update_role_scopes: UpdateRoleScopes,
    pub create_party: CreateParty,
    pub get_party: GetParty,
    pub list_my_parties: ListMyParties,
    pub search_parties: SearchParties,
    pub update_party: UpdateParty,
    pub delete_party: SoftDeleteParty,
    pub add_party_role: AddPartyRole,
    pub remove_party_role: RemovePartyRole,
    pub list_party_roles: ListPartyRoles,
    pub create_deal: CreateDeal,
    pub raise_dispute: RaiseDispute,
    pub list_deal_disputes: ListDealDisputes,
    pub get_dispute: GetDispute,
    pub submit_evidence: SubmitEvidence,
    pub respond_to_dispute: RespondToDispute,
    pub escalate_dispute: EscalateDispute,
    pub resolve_dispute: ResolveDispute,
    pub reject_dispute: RejectDispute,
    pub list_admin_disputes: ListAdminDisputes,
    pub get_deal: GetDeal,
    pub list_deals: ListDeals,
    pub update_deal: UpdateDeal,
    pub submit_deal: SubmitDeal,
    pub execute_transition: ExecuteTransition,
    pub propose_term: ProposeTerm,
    pub counter_term: CounterTerm,
    pub accept_term: AcceptTerm,
    pub reject_term: RejectTerm,
    pub withdraw_term: WithdrawTerm,
    pub list_terms: ListTerms,
    pub set_value_distribution: SetValueDistribution,
    pub get_value_distribution: GetValueDistribution,
    pub validate_deal: ValidateDeal,
    pub generate_agreement: GenerateAgreement,
    pub get_agreement: GetAgreement,
    pub sign_agreement: SignAgreement,
    pub admin_update_agreement: AdminUpdateAgreement,
    pub create_wallet: CreateWallet,
    pub deposit_points: DepositPoints,
    pub withdraw_points: WithdrawPoints,
    pub get_wallet: GetWallet,
    pub get_deal_wallet: GetDealWallet,
    pub hold_escrow: HoldEscrow,
    pub release_escrow: ReleaseEscrow,
    pub deduct_fee: DeductFee,
    pub record_adjustment: RecordAdjustment,
    pub list_wallet_transactions: ListWalletTransactions,
    pub list_deal_transactions: ListDealTransactions,
    pub approve_transaction: ApproveTransaction,
    pub list_pending_approvals: ListPendingApprovals,
    pub get_transaction: GetTransaction,
    pub create_milestone: CreateMilestone,
    pub list_milestones: ListMilestones,
    pub get_deal_progress: GetDealProgress,
    pub update_milestone: UpdateMilestone,
    pub delete_milestone: DeleteMilestone,
    pub start_milestone: StartMilestone,
    pub complete_milestone: CompleteMilestone,
    pub verify_milestone: VerifyMilestone,
    pub submit_review: SubmitReview,
    pub list_deal_reviews: ListDealReviews,
    pub list_party_reviews: ListPartyReviews,
    pub get_review: GetReview,
    pub get_deal_review_status: GetDealReviewStatus,
    pub hide_review: HideReview,
    pub list_admin_reviews: ListAdminReviews,
    pub get_trust_score: Option<GetTrustScore>,
    pub recalculate_trust_score: Option<RecalculateTrustScore>,
    pub submit_verification: SubmitVerification,
    pub list_party_verifications: ListPartyVerifications,
    pub get_verification_status: GetVerificationStatus,
    pub approve_verification: ApproveVerification,
    pub reject_verification: RejectVerification,
    pub revoke_verification: RevokeVerification,
    pub list_admin_verifications: ListAdminVerifications,
    pub send_message: SendMessage,
    pub edit_message: EditMessage,
    pub delete_message: SoftDeleteMessage,
    pub get_message: GetMessage,
    pub list_messages: ListMessages,
    pub list_conversations: ListConversations,
    pub mark_read: MarkRead,
    pub react: ToggleReaction,
    pub get_unread_count: GetUnreadCount,
    pub pin_message: PinMessage,
    pub unpin_message: UnpinMessage,
    pub admin_broadcast: AdminBroadcast,
    pub create_chat_room: CreateChatRoom,
    pub update_chat_room: UpdateChatRoom,
    pub delete_chat_room: SoftDeleteChatRoom,
    pub get_chat_room: GetChatRoom,
    pub list_chat_rooms: ListChatRooms,
    pub join_chat_room: JoinChatRoom,
    pub leave_chat_room: LeaveChatRoom,
    pub manage_chat_room_membership: ManageChatRoomMembership,
    pub list_notifications: ListNotifications,
    pub get_notification: GetNotification,
    pub mark_notification_read: MarkNotificationRead,
    pub mark_all_notifications_read: MarkAllNotificationsRead,
    pub delete_notification: DeleteNotification,
    pub get_unread_notification_count: GetUnreadNotificationCount,
    pub get_notification_preferences: GetNotificationPreferences,
    pub update_notification_preferences: UpdateNotificationPreferences,
    pub admin_send_notification: AdminSendNotification,
    pub admin_list_templates: AdminListTemplates,
    pub admin_create_template: AdminCreateTemplate,
    pub admin_get_template: AdminGetTemplate,
    pub admin_update_template: AdminUpdateTemplate,
    pub admin_delete_template: AdminDeleteTemplate,
    pub send_notification: Arc<SendNotification>,
    pub create_resource: CreateResource,
    pub update_resource: UpdateResource,
    pub delete_resource: DeleteResource,
    pub get_resource: GetResource,
    pub list_resources: ListResources,
    pub create_need: CreateNeed,
    pub update_need: UpdateNeed,
    pub delete_need: DeleteNeed,
    pub get_need: GetNeed,
    pub list_needs: ListNeeds,
    pub create_enhancement: CreateEnhancement,
    pub update_enhancement: UpdateEnhancement,
    pub delete_enhancement: DeleteEnhancement,
    pub get_enhancement: GetEnhancement,
    pub list_enhancements: ListEnhancements,
    pub admin_update_catalog_flags: AdminUpdateCatalogFlags,
    pub bind_catalog_item_to_deal: BindCatalogItemToDeal,
    pub list_deal_catalog_items: ListDealCatalogItems,
    pub contact_catalog_owner: ContactCatalogOwner,
    pub update_party_catalog_settings: UpdatePartyCatalogSettings,
    pub catalog_repo: Arc<dyn CatalogRepository>,
    pub db_pool: PgPool,
    pub encryption_service: Arc<dyn EncryptionService>,
    pub realtime_publisher: Arc<dyn RealtimePublisher>,
    pub notification_realtime_publisher: Arc<dyn NotificationRealtimePublisher>,
    pub message_repository: Arc<dyn MessageRepository>,
    pub chat_room_repository: Arc<dyn ChatRoomRepository>,
    pub websocket_registry: crate::websocket::SessionRegistry,
    pub token_validator: Arc<dyn TokenVerifier>,
}

/// Factory for the Actix HTTP server.
pub fn run(listener: TcpListener, state: AppState) -> Result<Server, std::io::Error> {
    let state = web::Data::new(state);

    let server = HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .configure(routes::configure)
            .wrap(TracingLogger::default())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
