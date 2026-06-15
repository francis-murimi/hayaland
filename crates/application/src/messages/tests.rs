use crate::errors::ApplicationError;
use crate::messages::dto::{
    AdminBroadcastCommand, BroadcastTarget, EditMessageCommand, MarkReadCommand, PinMessageCommand,
    SendMessageCommand, SoftDeleteMessageCommand, ToggleReactionCommand,
};
use crate::messages::{
    access::{is_conversation_visible_to_actor, is_message_visible_to_actor},
    AdminBroadcast, EditMessage, MarkRead, PinMessage, SendMessage, SoftDeleteMessage,
    ToggleReaction,
};
use crate::ports::NoOpRealtimePublisher;
use crate::test_helpers::{
    FakeChatRoomRepo, FakeDealRepo, FakeEncryptionService, FakeMessageRepo, FakePartyRepo,
    FakeRepo, RecordingPublisher,
};
use domain::entities::{
    ChatRoom, ChatRoomMemberRole, ChatRoomMembership, ChatRoomName, ChatRoomType, Conversation,
    DealParticipation, DealRole, DisplayName, Email, Message, ParticipationStatus, Party,
    PartyMembershipRole, PartyType, UserPartyMembership,
};
use domain::entities::{ConversationType, MessageType, ReactionType, RecipientType};
use domain::repositories::{ChatRoomRepository, PartyRepository};
use std::sync::Arc;
use uuid::Uuid;

fn alice() -> Uuid {
    Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap()
}

fn bob() -> Uuid {
    Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap()
}

fn charlie() -> Uuid {
    Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap()
}

fn admin_scopes() -> Vec<String> {
    vec!["admin:messages".to_string()]
}

fn user_scopes() -> Vec<String> {
    vec!["messages:read".to_string(), "messages:write".to_string()]
}

#[allow(dead_code)]
fn chatroom_write_scopes() -> Vec<String> {
    vec!["chatrooms:write".to_string()]
}

async fn make_party_with_member(
    party_repo: &Arc<FakePartyRepo>,
    user_id: Uuid,
    name: &str,
) -> Uuid {
    let party_id = Uuid::now_v7();
    let party = Party::new(
        party_id,
        PartyType::Organization,
        DisplayName::new(name).unwrap(),
        Email::new(&format!("{name}@example.com")).unwrap(),
    );
    party_repo.create(&party).await.unwrap();
    party_repo
        .add_membership(&UserPartyMembership::new(
            Uuid::now_v7(),
            user_id,
            party_id,
            PartyMembershipRole::Member,
        ))
        .await
        .unwrap();
    party_id
}

fn send_use_case(
    message_repo: Arc<FakeMessageRepo>,
    party_repo: Arc<FakePartyRepo>,
    room_repo: Arc<FakeChatRoomRepo>,
) -> SendMessage {
    send_use_case_with_deal(
        message_repo,
        party_repo,
        Arc::new(FakeDealRepo::default()),
        room_repo,
    )
}

fn send_use_case_with_deal(
    message_repo: Arc<FakeMessageRepo>,
    party_repo: Arc<FakePartyRepo>,
    deal_repo: Arc<FakeDealRepo>,
    room_repo: Arc<FakeChatRoomRepo>,
) -> SendMessage {
    SendMessage::new(
        message_repo,
        party_repo,
        deal_repo,
        room_repo,
        Arc::new(FakeEncryptionService),
        Arc::new(NoOpRealtimePublisher),
    )
}

#[tokio::test]
async fn send_direct_user_message_creates_conversation() {
    let message_repo = Arc::new(FakeMessageRepo::default());
    let use_case = send_use_case(
        message_repo.clone(),
        Arc::new(FakePartyRepo::default()),
        Arc::new(FakeChatRoomRepo::default()),
    );

    let result = use_case
        .execute(SendMessageCommand {
            actor_user_id: alice(),
            actor_party_id: None,
            scopes: user_scopes(),
            is_admin: false,
            recipient_type: RecipientType::User,
            recipient_user_id: Some(bob()),
            recipient_party_id: None,
            recipient_deal_id: None,
            recipient_room_id: None,
            message_type: MessageType::Text,
            subject: None,
            content: "hello bob".to_string(),
            attachment_urls: vec![],
            reply_to_message_id: None,
        })
        .await
        .unwrap();

    assert_eq!(result.recipient_user_id, Some(bob()));
    assert_eq!(result.content_plaintext, "hello bob");

    let conversations = message_repo.conversations.lock().unwrap();
    assert_eq!(conversations.len(), 1);
    let conv = conversations.values().next().unwrap();
    assert_eq!(conv.conversation_type, ConversationType::DirectUser);
}

#[tokio::test]
async fn send_party_members_message_requires_membership() {
    let message_repo = Arc::new(FakeMessageRepo::default());
    let party_repo = Arc::new(FakePartyRepo::default());
    let party_id = make_party_with_member(&party_repo, alice(), "party").await;

    let use_case = send_use_case(
        message_repo.clone(),
        party_repo.clone(),
        Arc::new(FakeChatRoomRepo::default()),
    );

    // Outsider cannot send.
    let err = use_case
        .execute(SendMessageCommand {
            actor_user_id: bob(),
            actor_party_id: Some(party_id),
            scopes: user_scopes(),
            is_admin: false,
            recipient_type: RecipientType::PartyMembers,
            recipient_user_id: None,
            recipient_party_id: None,
            recipient_deal_id: None,
            recipient_room_id: None,
            message_type: MessageType::Text,
            subject: None,
            content: "hello team".to_string(),
            attachment_urls: vec![],
            reply_to_message_id: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, ApplicationError::Forbidden));

    // Member can send.
    let result = use_case
        .execute(SendMessageCommand {
            actor_user_id: alice(),
            actor_party_id: Some(party_id),
            scopes: user_scopes(),
            is_admin: false,
            recipient_type: RecipientType::PartyMembers,
            recipient_user_id: None,
            recipient_party_id: None,
            recipient_deal_id: None,
            recipient_room_id: None,
            message_type: MessageType::Text,
            subject: None,
            content: "hello team".to_string(),
            attachment_urls: vec![],
            reply_to_message_id: None,
        })
        .await
        .unwrap();

    assert_eq!(result.sender_party_id, Some(party_id));
    let conv = message_repo
        .conversations
        .lock()
        .unwrap()
        .values()
        .next()
        .unwrap()
        .clone();
    assert_eq!(conv.conversation_type, ConversationType::PartyMembers);
}

#[tokio::test]
async fn send_party_message_visible_to_member() {
    let message_repo = Arc::new(FakeMessageRepo::default());
    let party_repo = Arc::new(FakePartyRepo::default());
    let sender_party_id = make_party_with_member(&party_repo, alice(), "sender-party").await;
    let recipient_party_id = make_party_with_member(&party_repo, bob(), "recipient-party").await;

    let send = send_use_case(
        message_repo.clone(),
        party_repo.clone(),
        Arc::new(FakeChatRoomRepo::default()),
    );

    let sent = send
        .execute(SendMessageCommand {
            actor_user_id: alice(),
            actor_party_id: Some(sender_party_id),
            scopes: user_scopes(),
            is_admin: false,
            recipient_type: RecipientType::Party,
            recipient_user_id: None,
            recipient_party_id: Some(recipient_party_id),
            recipient_deal_id: None,
            recipient_room_id: None,
            message_type: MessageType::Text,
            subject: None,
            content: "to the team".to_string(),
            attachment_urls: vec![],
            reply_to_message_id: None,
        })
        .await
        .unwrap();
    assert_eq!(sent.recipient_party_id, Some(recipient_party_id));

    let get = crate::messages::GetMessage::new(
        message_repo.clone(),
        party_repo.clone(),
        Arc::new(FakeDealRepo::default()),
        Arc::new(FakeChatRoomRepo::default()),
        Arc::new(FakeEncryptionService),
    );
    let found = get.execute(sent.id, bob(), None, false).await.unwrap();
    assert_eq!(found.content_plaintext, "to the team");
}

#[tokio::test]
async fn send_deal_message_visible_to_participant() {
    let message_repo = Arc::new(FakeMessageRepo::default());
    let party_repo = Arc::new(FakePartyRepo::default());
    let sender_party_id = make_party_with_member(&party_repo, alice(), "sender-party").await;
    let recipient_party_id = make_party_with_member(&party_repo, bob(), "recipient-party").await;

    let deal_id = Uuid::now_v7();
    let deal_repo = Arc::new(FakeDealRepo::default());
    deal_repo
        .participations
        .lock()
        .unwrap()
        .push(DealParticipation {
            id: Uuid::now_v7(),
            deal_id,
            party_id: sender_party_id,
            role: DealRole::Supplier,
            participation_status: ParticipationStatus::Accepted,
            is_initiator: true,
            value_share_percentage: None,
            value_share_amount: None,
            invited_at: Some(time::OffsetDateTime::now_utc()),
            responded_at: None,
            created_at: time::OffsetDateTime::now_utc(),
        });
    deal_repo
        .participations
        .lock()
        .unwrap()
        .push(DealParticipation {
            id: Uuid::now_v7(),
            deal_id,
            party_id: recipient_party_id,
            role: DealRole::Supplier,
            participation_status: ParticipationStatus::Accepted,
            is_initiator: false,
            value_share_percentage: None,
            value_share_amount: None,
            invited_at: Some(time::OffsetDateTime::now_utc()),
            responded_at: None,
            created_at: time::OffsetDateTime::now_utc(),
        });

    let send = send_use_case_with_deal(
        message_repo.clone(),
        party_repo.clone(),
        deal_repo.clone(),
        Arc::new(FakeChatRoomRepo::default()),
    );

    let sent = send
        .execute(SendMessageCommand {
            actor_user_id: alice(),
            actor_party_id: Some(sender_party_id),
            scopes: user_scopes(),
            is_admin: false,
            recipient_type: RecipientType::Deal,
            recipient_user_id: None,
            recipient_party_id: None,
            recipient_deal_id: Some(deal_id),
            recipient_room_id: None,
            message_type: MessageType::Text,
            subject: None,
            content: "deal update".to_string(),
            attachment_urls: vec![],
            reply_to_message_id: None,
        })
        .await
        .unwrap();

    let get = crate::messages::GetMessage::new(
        message_repo.clone(),
        party_repo.clone(),
        deal_repo.clone(),
        Arc::new(FakeChatRoomRepo::default()),
        Arc::new(FakeEncryptionService),
    );
    let found = get
        .execute(sent.id, bob(), Some(recipient_party_id), false)
        .await
        .unwrap();
    assert_eq!(found.content_plaintext, "deal update");
}

#[tokio::test]
async fn edit_message_by_sender_vs_non_sender() {
    let message_repo = Arc::new(FakeMessageRepo::default());
    let send = send_use_case(
        message_repo.clone(),
        Arc::new(FakePartyRepo::default()),
        Arc::new(FakeChatRoomRepo::default()),
    );

    let sent = send
        .execute(SendMessageCommand {
            actor_user_id: alice(),
            actor_party_id: None,
            scopes: user_scopes(),
            is_admin: false,
            recipient_type: RecipientType::User,
            recipient_user_id: Some(bob()),
            recipient_party_id: None,
            recipient_deal_id: None,
            recipient_room_id: None,
            message_type: MessageType::Text,
            subject: None,
            content: "original".to_string(),
            attachment_urls: vec![],
            reply_to_message_id: None,
        })
        .await
        .unwrap();

    let edit = EditMessage::new(
        message_repo.clone(),
        Arc::new(FakeEncryptionService),
        Arc::new(NoOpRealtimePublisher),
    );

    // Non-sender cannot edit.
    let err = edit
        .execute(EditMessageCommand {
            actor_user_id: bob(),
            actor_party_id: None,
            scopes: user_scopes(),
            is_admin: false,
            message_id: sent.id,
            content: "hacked".to_string(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, ApplicationError::CannotEditMessage));

    // Sender can edit.
    let updated = edit
        .execute(EditMessageCommand {
            actor_user_id: alice(),
            actor_party_id: None,
            scopes: user_scopes(),
            is_admin: false,
            message_id: sent.id,
            content: "updated".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(updated.content_plaintext, "updated");
    assert!(updated.edited_at.is_some());
}

#[tokio::test]
async fn soft_delete_message_by_sender_vs_non_sender() {
    let message_repo = Arc::new(FakeMessageRepo::default());
    let send = send_use_case(
        message_repo.clone(),
        Arc::new(FakePartyRepo::default()),
        Arc::new(FakeChatRoomRepo::default()),
    );

    let sent = send
        .execute(SendMessageCommand {
            actor_user_id: alice(),
            actor_party_id: None,
            scopes: user_scopes(),
            is_admin: false,
            recipient_type: RecipientType::User,
            recipient_user_id: Some(bob()),
            recipient_party_id: None,
            recipient_deal_id: None,
            recipient_room_id: None,
            message_type: MessageType::Text,
            subject: None,
            content: "delete me".to_string(),
            attachment_urls: vec![],
            reply_to_message_id: None,
        })
        .await
        .unwrap();

    let delete = SoftDeleteMessage::new(
        message_repo.clone(),
        Arc::new(FakeEncryptionService),
        Arc::new(NoOpRealtimePublisher),
    );

    let err = delete
        .execute(SoftDeleteMessageCommand {
            actor_user_id: bob(),
            actor_party_id: None,
            scopes: user_scopes(),
            is_admin: false,
            message_id: sent.id,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, ApplicationError::CannotDeleteMessage));

    let deleted = delete
        .execute(SoftDeleteMessageCommand {
            actor_user_id: alice(),
            actor_party_id: None,
            scopes: user_scopes(),
            is_admin: false,
            message_id: sent.id,
        })
        .await
        .unwrap();
    assert!(deleted.is_deleted);
}

#[tokio::test]
async fn reply_not_in_same_conversation_rejected() {
    let message_repo = Arc::new(FakeMessageRepo::default());
    let send = send_use_case(
        message_repo.clone(),
        Arc::new(FakePartyRepo::default()),
        Arc::new(FakeChatRoomRepo::default()),
    );

    let first = send
        .execute(SendMessageCommand {
            actor_user_id: alice(),
            actor_party_id: None,
            scopes: user_scopes(),
            is_admin: false,
            recipient_type: RecipientType::User,
            recipient_user_id: Some(bob()),
            recipient_party_id: None,
            recipient_deal_id: None,
            recipient_room_id: None,
            message_type: MessageType::Text,
            subject: None,
            content: "to bob".to_string(),
            attachment_urls: vec![],
            reply_to_message_id: None,
        })
        .await
        .unwrap();

    let err = send
        .execute(SendMessageCommand {
            actor_user_id: charlie(),
            actor_party_id: None,
            scopes: user_scopes(),
            is_admin: false,
            recipient_type: RecipientType::User,
            recipient_user_id: Some(bob()),
            recipient_party_id: None,
            recipient_deal_id: None,
            recipient_room_id: None,
            message_type: MessageType::Text,
            subject: None,
            content: "reply".to_string(),
            attachment_urls: vec![],
            reply_to_message_id: Some(first.id),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, ApplicationError::ReplyNotInSameContext));
}

#[tokio::test]
async fn mark_read_and_toggle_reaction() {
    let message_repo = Arc::new(FakeMessageRepo::default());
    let publisher = Arc::new(RecordingPublisher::default());
    let send = send_use_case(
        message_repo.clone(),
        Arc::new(FakePartyRepo::default()),
        Arc::new(FakeChatRoomRepo::default()),
    );
    let sent = send
        .execute(SendMessageCommand {
            actor_user_id: alice(),
            actor_party_id: None,
            scopes: user_scopes(),
            is_admin: false,
            recipient_type: RecipientType::User,
            recipient_user_id: Some(bob()),
            recipient_party_id: None,
            recipient_deal_id: None,
            recipient_room_id: None,
            message_type: MessageType::Text,
            subject: None,
            content: "read me".to_string(),
            attachment_urls: vec![],
            reply_to_message_id: None,
        })
        .await
        .unwrap();

    let mark = MarkRead::new(
        message_repo.clone(),
        Arc::new(FakePartyRepo::default()),
        Arc::new(FakeDealRepo::default()),
        Arc::new(FakeChatRoomRepo::default()),
        publisher.clone(),
    );
    let read = mark
        .execute(MarkReadCommand {
            actor_user_id: bob(),
            actor_party_id: None,
            scopes: user_scopes(),
            is_admin: false,
            message_id: sent.id,
        })
        .await
        .unwrap();
    assert_eq!(read.user_id, bob());

    let toggle = ToggleReaction::new(
        message_repo.clone(),
        Arc::new(FakePartyRepo::default()),
        Arc::new(FakeDealRepo::default()),
        Arc::new(FakeChatRoomRepo::default()),
        publisher.clone(),
    );
    let reaction = toggle
        .execute(ToggleReactionCommand {
            actor_user_id: bob(),
            actor_party_id: None,
            scopes: user_scopes(),
            is_admin: false,
            message_id: sent.id,
            reaction_type: ReactionType::Like,
        })
        .await
        .unwrap();
    assert!(reaction.is_some());

    // Toggling again removes it.
    let removed = toggle
        .execute(ToggleReactionCommand {
            actor_user_id: bob(),
            actor_party_id: None,
            scopes: user_scopes(),
            is_admin: false,
            message_id: sent.id,
            reaction_type: ReactionType::Like,
        })
        .await
        .unwrap();
    assert!(removed.is_none());

    let events = publisher.events.lock().unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, crate::ports::MessageEvent::MessageRead { .. })),
        "expected a read event"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, crate::ports::MessageEvent::MessageReaction { .. })),
        "expected a reaction event"
    );
}

#[tokio::test]
async fn admin_broadcast_requires_admin_scope() {
    let message_repo = Arc::new(FakeMessageRepo::default());
    let user_repo = Arc::new(FakeRepo {
        users: std::sync::Mutex::new(std::collections::HashMap::from([(
            bob(),
            domain::entities::User::new(
                bob(),
                Email::new("bob@example.com").unwrap(),
                domain::entities::Username::new("bob").unwrap(),
                domain::entities::PasswordHash::new("hash".to_string()).unwrap(),
            ),
        )])),
    });
    let party_repo = Arc::new(FakePartyRepo::default());
    let encryption = Arc::new(FakeEncryptionService);

    let broadcast = AdminBroadcast::new(
        message_repo.clone(),
        user_repo.clone(),
        party_repo.clone(),
        encryption.clone(),
        Arc::new(NoOpRealtimePublisher),
    );

    let err = broadcast
        .execute(AdminBroadcastCommand {
            actor_user_id: alice(),
            scopes: user_scopes(),
            target: BroadcastTarget::AllUsers,
            subject: Some("news".to_string()),
            content: "hello users".to_string(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, ApplicationError::Forbidden));

    let result = broadcast
        .execute(AdminBroadcastCommand {
            actor_user_id: alice(),
            scopes: admin_scopes(),
            target: BroadcastTarget::AllUsers,
            subject: Some("news".to_string()),
            content: "hello users".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].recipient_user_id, Some(bob()));
}

#[tokio::test]
async fn pin_only_for_group_contexts() {
    let message_repo = Arc::new(FakeMessageRepo::default());
    let party_repo = Arc::new(FakePartyRepo::default());
    let room_repo = Arc::new(FakeChatRoomRepo::default());
    let party_id = make_party_with_member(&party_repo, alice(), "party").await;

    let send = send_use_case(message_repo.clone(), party_repo.clone(), room_repo.clone());

    let direct = send
        .execute(SendMessageCommand {
            actor_user_id: alice(),
            actor_party_id: None,
            scopes: user_scopes(),
            is_admin: false,
            recipient_type: RecipientType::User,
            recipient_user_id: Some(bob()),
            recipient_party_id: None,
            recipient_deal_id: None,
            recipient_room_id: None,
            message_type: MessageType::Text,
            subject: None,
            content: "direct".to_string(),
            attachment_urls: vec![],
            reply_to_message_id: None,
        })
        .await
        .unwrap();

    let pin = PinMessage::new(
        message_repo.clone(),
        party_repo.clone(),
        Arc::new(FakeDealRepo::default()),
        room_repo.clone(),
        Arc::new(FakeEncryptionService),
    );
    let err = pin
        .execute(PinMessageCommand {
            actor_user_id: alice(),
            actor_party_id: None,
            scopes: user_scopes(),
            is_admin: false,
            message_id: direct.id,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, ApplicationError::Validation(_)));

    let group = send
        .execute(SendMessageCommand {
            actor_user_id: alice(),
            actor_party_id: Some(party_id),
            scopes: user_scopes(),
            is_admin: false,
            recipient_type: RecipientType::PartyMembers,
            recipient_user_id: None,
            recipient_party_id: None,
            recipient_deal_id: None,
            recipient_room_id: None,
            message_type: MessageType::Text,
            subject: None,
            content: "group".to_string(),
            attachment_urls: vec![],
            reply_to_message_id: None,
        })
        .await
        .unwrap();

    let pinned = pin
        .execute(PinMessageCommand {
            actor_user_id: alice(),
            actor_party_id: Some(party_id),
            scopes: user_scopes(),
            is_admin: false,
            message_id: group.id,
        })
        .await
        .unwrap();
    assert!(pinned.is_pinned);
}

fn make_message(
    sender: Uuid,
    sender_party_id: Option<Uuid>,
    recipient_type: RecipientType,
    recipient_user_id: Option<Uuid>,
    recipient_party_id: Option<Uuid>,
    recipient_deal_id: Option<Uuid>,
    recipient_room_id: Option<Uuid>,
) -> Message {
    Message::new(
        Uuid::now_v7(),
        Uuid::now_v7(),
        sender,
        sender_party_id,
        recipient_type,
        recipient_user_id,
        recipient_party_id,
        recipient_deal_id,
        recipient_room_id,
        MessageType::Text,
        None,
        "content".to_string(),
        vec![],
        None,
    )
    .unwrap()
}

async fn make_room_with_party_member(
    room_repo: &Arc<FakeChatRoomRepo>,
    party_repo: &Arc<FakePartyRepo>,
    user_id: Uuid,
    room_name: &str,
) -> (Uuid, Uuid) {
    let party_id = make_party_with_member(party_repo, user_id, &format!("{room_name}-party")).await;
    let room_id = Uuid::now_v7();
    let room = ChatRoom::new(
        room_id,
        ChatRoomName::new(room_name).unwrap(),
        None,
        ChatRoomType::Public,
        user_id,
    );
    room_repo.create_room(&room).await.unwrap();
    room_repo
        .add_membership(&ChatRoomMembership::for_party(
            Uuid::now_v7(),
            room_id,
            party_id,
            ChatRoomMemberRole::Member,
        ))
        .await
        .unwrap();
    (room_id, party_id)
}

fn sample_deal_participation(deal_id: Uuid, party_id: Uuid) -> DealParticipation {
    DealParticipation {
        id: Uuid::now_v7(),
        deal_id,
        party_id,
        role: DealRole::Supplier,
        participation_status: ParticipationStatus::Accepted,
        is_initiator: false,
        value_share_percentage: None,
        value_share_amount: None,
        invited_at: Some(time::OffsetDateTime::now_utc()),
        responded_at: None,
        created_at: time::OffsetDateTime::now_utc(),
    }
}

#[tokio::test]
async fn message_visibility_user_recipient() {
    let msg = make_message(
        alice(),
        None,
        RecipientType::User,
        Some(bob()),
        None,
        None,
        None,
    );
    let party_repo = Arc::new(FakePartyRepo::default());
    let deal_repo = Arc::new(FakeDealRepo::default());
    let room_repo = Arc::new(FakeChatRoomRepo::default());

    assert!(is_message_visible_to_actor(
        &msg,
        bob(),
        None,
        false,
        party_repo.as_ref(),
        deal_repo.as_ref(),
        room_repo.as_ref()
    )
    .await
    .unwrap());
    assert!(!is_message_visible_to_actor(
        &msg,
        charlie(),
        None,
        false,
        party_repo.as_ref(),
        deal_repo.as_ref(),
        room_repo.as_ref()
    )
    .await
    .unwrap());
    // Sender always sees their own message.
    assert!(is_message_visible_to_actor(
        &msg,
        alice(),
        None,
        false,
        party_repo.as_ref(),
        deal_repo.as_ref(),
        room_repo.as_ref()
    )
    .await
    .unwrap());
}

#[tokio::test]
async fn message_visibility_party_members() {
    let party_repo = Arc::new(FakePartyRepo::default());
    let party_id = make_party_with_member(&party_repo, bob(), "crew").await;
    let msg = make_message(
        alice(),
        Some(party_id),
        RecipientType::PartyMembers,
        None,
        None,
        None,
        None,
    );
    let deal_repo = Arc::new(FakeDealRepo::default());
    let room_repo = Arc::new(FakeChatRoomRepo::default());

    assert!(is_message_visible_to_actor(
        &msg,
        bob(),
        None,
        false,
        party_repo.as_ref(),
        deal_repo.as_ref(),
        room_repo.as_ref()
    )
    .await
    .unwrap());
    assert!(!is_message_visible_to_actor(
        &msg,
        charlie(),
        None,
        false,
        party_repo.as_ref(),
        deal_repo.as_ref(),
        room_repo.as_ref()
    )
    .await
    .unwrap());
}

#[tokio::test]
async fn message_visibility_deal_participant() {
    let party_repo = Arc::new(FakePartyRepo::default());
    let deal_repo = Arc::new(FakeDealRepo::default());
    let party_a = make_party_with_member(&party_repo, alice(), "deal-a").await;
    let party_b = make_party_with_member(&party_repo, bob(), "deal-b").await;

    let deal_id = Uuid::now_v7();
    deal_repo
        .participations
        .lock()
        .unwrap()
        .push(sample_deal_participation(deal_id, party_a));
    deal_repo
        .participations
        .lock()
        .unwrap()
        .push(sample_deal_participation(deal_id, party_b));

    let msg = make_message(
        alice(),
        None,
        RecipientType::Deal,
        None,
        None,
        Some(deal_id),
        None,
    );
    let room_repo = Arc::new(FakeChatRoomRepo::default());

    // Participant sees via actor_party_id match.
    assert!(is_message_visible_to_actor(
        &msg,
        bob(),
        Some(party_b),
        false,
        party_repo.as_ref(),
        deal_repo.as_ref(),
        room_repo.as_ref()
    )
    .await
    .unwrap());
    // Participant sees via party membership fallback.
    assert!(is_message_visible_to_actor(
        &msg,
        bob(),
        None,
        false,
        party_repo.as_ref(),
        deal_repo.as_ref(),
        room_repo.as_ref()
    )
    .await
    .unwrap());
    // Outsider does not see.
    assert!(!is_message_visible_to_actor(
        &msg,
        charlie(),
        None,
        false,
        party_repo.as_ref(),
        deal_repo.as_ref(),
        room_repo.as_ref()
    )
    .await
    .unwrap());
}

#[tokio::test]
async fn message_visibility_room_member() {
    let party_repo = Arc::new(FakePartyRepo::default());
    let room_repo = Arc::new(FakeChatRoomRepo::default());
    let (room_id, party_id) =
        make_room_with_party_member(&room_repo, &party_repo, bob(), "room").await;

    let msg = make_message(
        alice(),
        None,
        RecipientType::Room,
        None,
        None,
        None,
        Some(room_id),
    );
    let deal_repo = Arc::new(FakeDealRepo::default());

    assert!(is_message_visible_to_actor(
        &msg,
        bob(),
        Some(party_id),
        false,
        party_repo.as_ref(),
        deal_repo.as_ref(),
        room_repo.as_ref()
    )
    .await
    .unwrap());
    // Outsider without the party context cannot see.
    assert!(!is_message_visible_to_actor(
        &msg,
        charlie(),
        None,
        false,
        party_repo.as_ref(),
        deal_repo.as_ref(),
        room_repo.as_ref()
    )
    .await
    .unwrap());
}

#[tokio::test]
async fn message_visibility_admin_broadcast() {
    let msg = make_message(
        alice(),
        None,
        RecipientType::AdminBroadcast,
        None,
        None,
        None,
        None,
    );
    let party_repo = Arc::new(FakePartyRepo::default());
    let deal_repo = Arc::new(FakeDealRepo::default());
    let room_repo = Arc::new(FakeChatRoomRepo::default());

    assert!(is_message_visible_to_actor(
        &msg,
        bob(),
        None,
        false,
        party_repo.as_ref(),
        deal_repo.as_ref(),
        room_repo.as_ref()
    )
    .await
    .unwrap());
}

#[tokio::test]
async fn message_visibility_admin_or_sender_bypasses_recipient_checks() {
    let msg = make_message(
        alice(),
        None,
        RecipientType::User,
        Some(bob()),
        None,
        None,
        None,
    );
    let party_repo = Arc::new(FakePartyRepo::default());
    let deal_repo = Arc::new(FakeDealRepo::default());
    let room_repo = Arc::new(FakeChatRoomRepo::default());

    // Admin sees even though not the recipient.
    assert!(is_message_visible_to_actor(
        &msg,
        charlie(),
        None,
        true,
        party_repo.as_ref(),
        deal_repo.as_ref(),
        room_repo.as_ref()
    )
    .await
    .unwrap());
    // Sender sees even though recipient mismatch.
    assert!(is_message_visible_to_actor(
        &msg,
        alice(),
        None,
        false,
        party_repo.as_ref(),
        deal_repo.as_ref(),
        room_repo.as_ref()
    )
    .await
    .unwrap());
}

#[tokio::test]
async fn conversation_visibility_deal_and_room() {
    let party_repo = Arc::new(FakePartyRepo::default());
    let deal_repo = Arc::new(FakeDealRepo::default());
    let room_repo = Arc::new(FakeChatRoomRepo::default());

    let party_a = make_party_with_member(&party_repo, alice(), "conv-a").await;
    let party_b = make_party_with_member(&party_repo, bob(), "conv-b").await;
    let deal_id = Uuid::now_v7();
    deal_repo
        .participations
        .lock()
        .unwrap()
        .push(sample_deal_participation(deal_id, party_a));
    deal_repo
        .participations
        .lock()
        .unwrap()
        .push(sample_deal_participation(deal_id, party_b));

    let deal_conv = Conversation::new_deal(Uuid::now_v7(), deal_id, None);
    assert!(is_conversation_visible_to_actor(
        &deal_conv,
        bob(),
        Some(party_b),
        false,
        party_repo.as_ref(),
        deal_repo.as_ref(),
        room_repo.as_ref()
    )
    .await
    .unwrap());
    assert!(!is_conversation_visible_to_actor(
        &deal_conv,
        charlie(),
        None,
        false,
        party_repo.as_ref(),
        deal_repo.as_ref(),
        room_repo.as_ref()
    )
    .await
    .unwrap());

    let (room_id, room_party_id) =
        make_room_with_party_member(&room_repo, &party_repo, bob(), "conv-room").await;
    let room_conv = Conversation::new_room(Uuid::now_v7(), room_id, None);
    assert!(is_conversation_visible_to_actor(
        &room_conv,
        bob(),
        Some(room_party_id),
        false,
        party_repo.as_ref(),
        deal_repo.as_ref(),
        room_repo.as_ref()
    )
    .await
    .unwrap());

    let broadcast_conv = Conversation {
        id: Uuid::now_v7(),
        conversation_type: ConversationType::AdminBroadcast,
        user_a_id: None,
        user_b_id: None,
        party_a_id: None,
        party_b_id: None,
        party_id: None,
        deal_id: None,
        room_id: None,
        title: None,
        last_message_at: time::OffsetDateTime::now_utc(),
        created_at: time::OffsetDateTime::now_utc(),
    };
    assert!(is_conversation_visible_to_actor(
        &broadcast_conv,
        bob(),
        None,
        false,
        party_repo.as_ref(),
        deal_repo.as_ref(),
        room_repo.as_ref()
    )
    .await
    .unwrap());
}

#[tokio::test]
async fn message_and_conversation_visibility_remaining_branches() {
    let party_repo = Arc::new(FakePartyRepo::default());
    let deal_repo = Arc::new(FakeDealRepo::default());
    let room_repo = Arc::new(FakeChatRoomRepo::default());

    let party_id = make_party_with_member(&party_repo, bob(), "branch-party").await;
    let other_party_id = make_party_with_member(&party_repo, alice(), "branch-other").await;

    // Direct-user conversation visibility.
    let direct_user_conv = Conversation::new_direct_user(Uuid::now_v7(), alice(), bob());
    assert!(is_conversation_visible_to_actor(
        &direct_user_conv,
        bob(),
        None,
        false,
        party_repo.as_ref(),
        deal_repo.as_ref(),
        room_repo.as_ref()
    )
    .await
    .unwrap());
    assert!(!is_conversation_visible_to_actor(
        &direct_user_conv,
        charlie(),
        None,
        false,
        party_repo.as_ref(),
        deal_repo.as_ref(),
        room_repo.as_ref()
    )
    .await
    .unwrap());

    // Direct-party conversation visibility matches actor_party_id.
    let direct_party_conv =
        Conversation::new_direct_party(Uuid::now_v7(), party_id, other_party_id);
    assert!(is_conversation_visible_to_actor(
        &direct_party_conv,
        bob(),
        Some(party_id),
        false,
        party_repo.as_ref(),
        deal_repo.as_ref(),
        room_repo.as_ref()
    )
    .await
    .unwrap());
    assert!(!is_conversation_visible_to_actor(
        &direct_party_conv,
        bob(),
        None,
        false,
        party_repo.as_ref(),
        deal_repo.as_ref(),
        room_repo.as_ref()
    )
    .await
    .unwrap());

    // Party-members conversation visibility.
    let party_members_conv = Conversation::new_party_members(Uuid::now_v7(), party_id, None);
    assert!(is_conversation_visible_to_actor(
        &party_members_conv,
        bob(),
        None,
        false,
        party_repo.as_ref(),
        deal_repo.as_ref(),
        room_repo.as_ref()
    )
    .await
    .unwrap());
    assert!(!is_conversation_visible_to_actor(
        &party_members_conv,
        charlie(),
        None,
        false,
        party_repo.as_ref(),
        deal_repo.as_ref(),
        room_repo.as_ref()
    )
    .await
    .unwrap());

    // Party message visible when actor_party_id matches recipient_party_id.
    let party_msg = make_message(
        alice(),
        None,
        RecipientType::Party,
        None,
        Some(party_id),
        None,
        None,
    );
    assert!(is_message_visible_to_actor(
        &party_msg,
        bob(),
        Some(party_id),
        false,
        party_repo.as_ref(),
        deal_repo.as_ref(),
        room_repo.as_ref()
    )
    .await
    .unwrap());

    // Room message visible via direct user membership.
    let room_id = Uuid::now_v7();
    let room = ChatRoom::new(
        room_id,
        ChatRoomName::new("user-room").unwrap(),
        None,
        ChatRoomType::Public,
        alice(),
    );
    room_repo.create_room(&room).await.unwrap();
    room_repo
        .add_membership(&ChatRoomMembership::for_user(
            Uuid::now_v7(),
            room_id,
            charlie(),
            ChatRoomMemberRole::Member,
        ))
        .await
        .unwrap();
    let room_msg = make_message(
        alice(),
        None,
        RecipientType::Room,
        None,
        None,
        None,
        Some(room_id),
    );
    assert!(is_message_visible_to_actor(
        &room_msg,
        charlie(),
        None,
        false,
        party_repo.as_ref(),
        deal_repo.as_ref(),
        room_repo.as_ref()
    )
    .await
    .unwrap());
}
