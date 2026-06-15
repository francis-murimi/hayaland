use crate::chatrooms::dto::{
    CreateChatRoomCommand, JoinChatRoomCommand, LeaveChatRoomCommand, ManageMembershipCommand,
    MembershipAction,
};
use crate::chatrooms::{
    CreateChatRoom, GetChatRoom, JoinChatRoom, LeaveChatRoom, ManageChatRoomMembership,
    SoftDeleteChatRoom, UpdateChatRoom,
};
use crate::errors::ApplicationError;
use crate::messages::dto::ListMessagesQuery;
use crate::messages::ListMessages;
use crate::ports::NoOpRealtimePublisher;
use crate::test_helpers::{
    FakeChatRoomRepo, FakeDealRepo, FakeEncryptionService, FakeMessageRepo, FakePartyRepo,
};
use domain::entities::{
    ChatRoomMemberRole, ChatRoomType, DisplayName, Email, MessageType, Party, PartyMembershipRole,
    PartyType, RecipientType, UserPartyMembership,
};
use domain::repositories::{ChatRoomRepository, MessageRepository, PartyRepository};
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

fn write_scopes() -> Vec<String> {
    vec!["chatrooms:write".to_string()]
}

fn moderate_scopes() -> Vec<String> {
    vec!["chatrooms:moderate".to_string()]
}

#[allow(dead_code)]
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

fn create_use_case(
    room_repo: Arc<FakeChatRoomRepo>,
    message_repo: Arc<FakeMessageRepo>,
) -> CreateChatRoom {
    CreateChatRoom::new(room_repo, message_repo)
}

#[tokio::test]
async fn create_chat_room_requires_write_scope() {
    let room_repo = Arc::new(FakeChatRoomRepo::default());
    let message_repo = Arc::new(FakeMessageRepo::default());
    let use_case = create_use_case(room_repo.clone(), message_repo.clone());

    let err = use_case
        .execute(CreateChatRoomCommand {
            actor_user_id: alice(),
            scopes: vec!["messages:read".to_string()],
            name: "General".to_string(),
            description: None,
            room_type: ChatRoomType::Public,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, ApplicationError::Forbidden));

    let result = use_case
        .execute(CreateChatRoomCommand {
            actor_user_id: alice(),
            scopes: write_scopes(),
            name: "General".to_string(),
            description: None,
            room_type: ChatRoomType::Public,
        })
        .await
        .unwrap();
    assert_eq!(result.name, "General");
    assert_eq!(room_repo.rooms.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn public_room_can_be_joined_freely() {
    let room_repo = Arc::new(FakeChatRoomRepo::default());
    let message_repo = Arc::new(FakeMessageRepo::default());
    let create = create_use_case(room_repo.clone(), message_repo.clone());
    let room = create
        .execute(CreateChatRoomCommand {
            actor_user_id: alice(),
            scopes: write_scopes(),
            name: "Public Room".to_string(),
            description: None,
            room_type: ChatRoomType::Public,
        })
        .await
        .unwrap();

    let party_repo = Arc::new(FakePartyRepo::default());
    let join = JoinChatRoom::new(room_repo.clone(), party_repo.clone());
    let membership = join
        .execute(JoinChatRoomCommand {
            actor_user_id: bob(),
            actor_party_id: None,
            scopes: vec![],
            is_admin: false,
            room_id: room.id,
        })
        .await
        .unwrap();
    assert_eq!(membership.user_id, Some(bob()));
}

#[tokio::test]
async fn private_room_requires_moderate_scope() {
    let room_repo = Arc::new(FakeChatRoomRepo::default());
    let message_repo = Arc::new(FakeMessageRepo::default());
    let create = create_use_case(room_repo.clone(), message_repo.clone());
    let room = create
        .execute(CreateChatRoomCommand {
            actor_user_id: alice(),
            scopes: write_scopes(),
            name: "Private Room".to_string(),
            description: None,
            room_type: ChatRoomType::Private,
        })
        .await
        .unwrap();

    let party_repo = Arc::new(FakePartyRepo::default());
    let join = JoinChatRoom::new(room_repo.clone(), party_repo.clone());
    let err = join
        .execute(JoinChatRoomCommand {
            actor_user_id: bob(),
            actor_party_id: None,
            scopes: vec![],
            is_admin: false,
            room_id: room.id,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, ApplicationError::Forbidden));

    let joined = join
        .execute(JoinChatRoomCommand {
            actor_user_id: bob(),
            actor_party_id: None,
            scopes: moderate_scopes(),
            is_admin: false,
            room_id: room.id,
        })
        .await
        .unwrap();
    assert_eq!(joined.user_id, Some(bob()));
}

#[tokio::test]
async fn leave_chat_room_removes_membership() {
    let room_repo = Arc::new(FakeChatRoomRepo::default());
    let message_repo = Arc::new(FakeMessageRepo::default());
    let create = create_use_case(room_repo.clone(), message_repo.clone());
    let room = create
        .execute(CreateChatRoomCommand {
            actor_user_id: alice(),
            scopes: write_scopes(),
            name: "Leavable".to_string(),
            description: None,
            room_type: ChatRoomType::Public,
        })
        .await
        .unwrap();

    let party_repo = Arc::new(FakePartyRepo::default());
    let join = JoinChatRoom::new(room_repo.clone(), party_repo.clone());
    join.execute(JoinChatRoomCommand {
        actor_user_id: bob(),
        actor_party_id: None,
        scopes: vec![],
        is_admin: false,
        room_id: room.id,
    })
    .await
    .unwrap();

    let leave = LeaveChatRoom::new(room_repo.clone());
    leave
        .execute(LeaveChatRoomCommand {
            actor_user_id: bob(),
            actor_party_id: None,
            room_id: room.id,
        })
        .await
        .unwrap();

    assert!(room_repo
        .find_membership_for_user(room.id, bob())
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn manage_membership_add_and_remove() {
    let room_repo = Arc::new(FakeChatRoomRepo::default());
    let message_repo = Arc::new(FakeMessageRepo::default());
    let create = create_use_case(room_repo.clone(), message_repo.clone());
    let room = create
        .execute(CreateChatRoomCommand {
            actor_user_id: alice(),
            scopes: write_scopes(),
            name: "Managed".to_string(),
            description: None,
            room_type: ChatRoomType::Private,
        })
        .await
        .unwrap();

    let manage = ManageChatRoomMembership::new(room_repo.clone());
    let added = manage
        .execute(ManageMembershipCommand {
            actor_user_id: alice(),
            scopes: moderate_scopes(),
            is_admin: false,
            room_id: room.id,
            action: MembershipAction::Add,
            target_user_id: Some(bob()),
            target_party_id: None,
            role: Some(ChatRoomMemberRole::Member),
            membership_id: None,
        })
        .await
        .unwrap();
    assert!(added.is_some());

    let membership_id = added.unwrap().id;
    manage
        .execute(ManageMembershipCommand {
            actor_user_id: alice(),
            scopes: moderate_scopes(),
            is_admin: false,
            room_id: room.id,
            action: MembershipAction::Remove,
            target_user_id: None,
            target_party_id: None,
            role: None,
            membership_id: Some(membership_id),
        })
        .await
        .unwrap();

    assert!(room_repo
        .find_membership_for_user(room.id, bob())
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn update_and_get_chat_room() {
    let room_repo = Arc::new(FakeChatRoomRepo::default());
    let message_repo = Arc::new(FakeMessageRepo::default());
    let create = create_use_case(room_repo.clone(), message_repo.clone());
    let room = create
        .execute(CreateChatRoomCommand {
            actor_user_id: alice(),
            scopes: write_scopes(),
            name: "Updatable".to_string(),
            description: None,
            room_type: ChatRoomType::Public,
        })
        .await
        .unwrap();

    let update = UpdateChatRoom::new(room_repo.clone());
    let updated = update
        .execute(crate::chatrooms::dto::UpdateChatRoomCommand {
            actor_user_id: alice(),
            scopes: write_scopes(),
            is_admin: false,
            room_id: room.id,
            name: Some("Updated Name".to_string()),
            description: Some("new description".to_string()),
            room_type: None,
        })
        .await
        .unwrap();
    assert_eq!(updated.name, "Updated Name");

    let get = GetChatRoom::new(room_repo.clone());
    let found = get.execute(room.id, alice(), vec![], false).await.unwrap();
    assert_eq!(found.name, "Updated Name");
}

#[tokio::test]
async fn soft_delete_chat_room() {
    let room_repo = Arc::new(FakeChatRoomRepo::default());
    let message_repo = Arc::new(FakeMessageRepo::default());
    let create = create_use_case(room_repo.clone(), message_repo.clone());
    let room = create
        .execute(CreateChatRoomCommand {
            actor_user_id: alice(),
            scopes: write_scopes(),
            name: "Deletable".to_string(),
            description: None,
            room_type: ChatRoomType::Public,
        })
        .await
        .unwrap();

    let delete = SoftDeleteChatRoom::new(room_repo.clone(), Arc::new(NoOpRealtimePublisher));
    delete
        .execute(crate::chatrooms::dto::SoftDeleteChatRoomCommand {
            actor_user_id: alice(),
            scopes: write_scopes(),
            is_admin: false,
            room_id: room.id,
        })
        .await
        .unwrap();

    let found = room_repo.find_room_by_id(room.id).await.unwrap().unwrap();
    assert!(found.is_deleted);
}

#[tokio::test]
async fn send_and_list_room_messages() {
    let room_repo = Arc::new(FakeChatRoomRepo::default());
    let message_repo = Arc::new(FakeMessageRepo::default());
    let create = create_use_case(room_repo.clone(), message_repo.clone());
    let room = create
        .execute(CreateChatRoomCommand {
            actor_user_id: alice(),
            scopes: write_scopes(),
            name: "Room Chat".to_string(),
            description: None,
            room_type: ChatRoomType::Public,
        })
        .await
        .unwrap();

    let party_repo = Arc::new(FakePartyRepo::default());
    let join = JoinChatRoom::new(room_repo.clone(), party_repo.clone());
    join.execute(JoinChatRoomCommand {
        actor_user_id: bob(),
        actor_party_id: None,
        scopes: vec![],
        is_admin: false,
        room_id: room.id,
    })
    .await
    .unwrap();

    // Conversation was created by CreateChatRoom via message_repo.
    let conversation = message_repo
        .find_room_conversation(room.id)
        .await
        .unwrap()
        .unwrap();

    use crate::messages::SendMessage;
    let send = SendMessage::new(
        message_repo.clone(),
        party_repo.clone(),
        Arc::new(FakeDealRepo::default()),
        room_repo.clone(),
        Arc::new(FakeEncryptionService),
        Arc::new(NoOpRealtimePublisher),
    );
    send.execute(crate::messages::dto::SendMessageCommand {
        actor_user_id: bob(),
        actor_party_id: None,
        scopes: vec!["messages:write".to_string()],
        is_admin: false,
        recipient_type: RecipientType::Room,
        recipient_user_id: None,
        recipient_party_id: None,
        recipient_deal_id: None,
        recipient_room_id: Some(room.id),
        message_type: MessageType::Text,
        subject: None,
        content: "hello room".to_string(),
        attachment_urls: vec![],
        reply_to_message_id: None,
    })
    .await
    .unwrap();

    let list = ListMessages::new(
        message_repo.clone(),
        party_repo.clone(),
        Arc::new(FakeDealRepo::default()),
        room_repo.clone(),
        Arc::new(FakeEncryptionService),
    );
    let messages = list
        .execute(
            conversation.id,
            ListMessagesQuery {
                actor_user_id: bob(),
                actor_party_id: None,
                scopes: vec!["messages:read".to_string()],
                is_admin: false,
                before_id: None,
                limit: 50,
            },
        )
        .await
        .unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content_plaintext, "hello room");
}

use crate::chatrooms::access::{
    can_create_chat_room, can_manage_chat_room, has_scope, is_chat_room_visible,
};

#[tokio::test]
async fn chat_room_access_functions() {
    let room_repo = Arc::new(FakeChatRoomRepo::default());
    let message_repo = Arc::new(FakeMessageRepo::default());
    let create = create_use_case(room_repo.clone(), message_repo.clone());

    let public = create
        .execute(CreateChatRoomCommand {
            actor_user_id: alice(),
            scopes: write_scopes(),
            name: "Public".to_string(),
            description: None,
            room_type: ChatRoomType::Public,
        })
        .await
        .unwrap();

    let private = create
        .execute(CreateChatRoomCommand {
            actor_user_id: alice(),
            scopes: write_scopes(),
            name: "Private".to_string(),
            description: None,
            room_type: ChatRoomType::Private,
        })
        .await
        .unwrap();

    let deleted = create
        .execute(CreateChatRoomCommand {
            actor_user_id: alice(),
            scopes: write_scopes(),
            name: "Deleted".to_string(),
            description: None,
            room_type: ChatRoomType::Public,
        })
        .await
        .unwrap();

    SoftDeleteChatRoom::new(room_repo.clone(), Arc::new(NoOpRealtimePublisher))
        .execute(crate::chatrooms::dto::SoftDeleteChatRoomCommand {
            actor_user_id: alice(),
            scopes: write_scopes(),
            is_admin: false,
            room_id: deleted.id,
        })
        .await
        .unwrap();

    // Scope helpers.
    assert!(has_scope(&write_scopes(), "chatrooms:write"));
    assert!(has_scope(&vec!["admin:*".to_string()], "chatrooms:write"));
    assert!(!has_scope(&vec![], "chatrooms:write"));
    assert!(can_create_chat_room(&write_scopes()));
    assert!(can_create_chat_room(&moderate_scopes()));
    assert!(can_create_chat_room(&vec!["admin:messages".to_string()]));
    assert!(!can_create_chat_room(&vec![]));

    // Visibility.
    let public_room = room_repo.find_room_by_id(public.id).await.unwrap().unwrap();
    let private_room = room_repo
        .find_room_by_id(private.id)
        .await
        .unwrap()
        .unwrap();
    let deleted_room = room_repo
        .find_room_by_id(deleted.id)
        .await
        .unwrap()
        .unwrap();
    assert!(
        is_chat_room_visible(&public_room, bob(), &[], false, &*room_repo)
            .await
            .unwrap()
    );
    assert!(
        is_chat_room_visible(&public_room, bob(), &[], true, &*room_repo)
            .await
            .unwrap()
    );
    assert!(
        !is_chat_room_visible(&deleted_room, bob(), &[], false, &*room_repo)
            .await
            .unwrap()
    );
    assert!(
        !is_chat_room_visible(&private_room, bob(), &[], false, &*room_repo)
            .await
            .unwrap()
    );

    // Management.
    assert!(
        can_manage_chat_room(&public_room, alice(), false, &*room_repo)
            .await
            .unwrap()
    );
    assert!(can_manage_chat_room(&public_room, bob(), true, &*room_repo)
        .await
        .unwrap());
    assert!(
        !can_manage_chat_room(&public_room, bob(), false, &*room_repo)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn join_chat_room_party_and_validation_errors() {
    let room_repo = Arc::new(FakeChatRoomRepo::default());
    let message_repo = Arc::new(FakeMessageRepo::default());
    let create = create_use_case(room_repo.clone(), message_repo.clone());
    let room = create
        .execute(CreateChatRoomCommand {
            actor_user_id: alice(),
            scopes: write_scopes(),
            name: "Join Party".to_string(),
            description: None,
            room_type: ChatRoomType::Public,
        })
        .await
        .unwrap();

    let party_repo = Arc::new(FakePartyRepo::default());
    let party_id = make_party_with_member(&party_repo, bob(), "join-party").await;
    let join = JoinChatRoom::new(room_repo.clone(), party_repo.clone());

    // Cannot join on behalf of a party the actor is not a member of.
    let other_party_id = Uuid::now_v7();
    let other_party = Party::new(
        other_party_id,
        PartyType::Organization,
        DisplayName::new("other").unwrap(),
        Email::new("other@example.com").unwrap(),
    );
    party_repo.create(&other_party).await.unwrap();
    let err = join
        .execute(JoinChatRoomCommand {
            actor_user_id: bob(),
            actor_party_id: Some(other_party_id),
            scopes: vec![],
            is_admin: false,
            room_id: room.id,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, ApplicationError::Forbidden));

    // Join as party member works.
    join.execute(JoinChatRoomCommand {
        actor_user_id: bob(),
        actor_party_id: Some(party_id),
        scopes: vec![],
        is_admin: false,
        room_id: room.id,
    })
    .await
    .unwrap();

    // Duplicate party membership is rejected.
    let err = join
        .execute(JoinChatRoomCommand {
            actor_user_id: bob(),
            actor_party_id: Some(party_id),
            scopes: vec![],
            is_admin: false,
            room_id: room.id,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, ApplicationError::AlreadyChatRoomMember));

    // Joining a deleted room is not found.
    SoftDeleteChatRoom::new(room_repo.clone(), Arc::new(NoOpRealtimePublisher))
        .execute(crate::chatrooms::dto::SoftDeleteChatRoomCommand {
            actor_user_id: alice(),
            scopes: write_scopes(),
            is_admin: false,
            room_id: room.id,
        })
        .await
        .unwrap();
    let err = join
        .execute(JoinChatRoomCommand {
            actor_user_id: charlie(),
            actor_party_id: None,
            scopes: vec![],
            is_admin: false,
            room_id: room.id,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, ApplicationError::ChatRoomNotFound));
}

#[tokio::test]
async fn leave_chat_room_party_and_forbidden() {
    let room_repo = Arc::new(FakeChatRoomRepo::default());
    let message_repo = Arc::new(FakeMessageRepo::default());
    let create = create_use_case(room_repo.clone(), message_repo.clone());
    let room = create
        .execute(CreateChatRoomCommand {
            actor_user_id: alice(),
            scopes: write_scopes(),
            name: "Leave Party".to_string(),
            description: None,
            room_type: ChatRoomType::Public,
        })
        .await
        .unwrap();

    let party_repo = Arc::new(FakePartyRepo::default());
    let party_id = make_party_with_member(&party_repo, bob(), "leave-party").await;
    let join = JoinChatRoom::new(room_repo.clone(), party_repo.clone());
    join.execute(JoinChatRoomCommand {
        actor_user_id: bob(),
        actor_party_id: Some(party_id),
        scopes: vec![],
        is_admin: false,
        room_id: room.id,
    })
    .await
    .unwrap();

    let leave = LeaveChatRoom::new(room_repo.clone());
    leave
        .execute(LeaveChatRoomCommand {
            actor_user_id: bob(),
            actor_party_id: Some(party_id),
            room_id: room.id,
        })
        .await
        .unwrap();

    // Leaving again is not found.
    let err = leave
        .execute(LeaveChatRoomCommand {
            actor_user_id: bob(),
            actor_party_id: Some(party_id),
            room_id: room.id,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, ApplicationError::ChatRoomMembershipNotFound));

    // Leaving a party membership the actor is not associated with is not found.
    JoinChatRoom::new(room_repo.clone(), party_repo.clone())
        .execute(JoinChatRoomCommand {
            actor_user_id: charlie(),
            actor_party_id: None,
            scopes: vec![],
            is_admin: false,
            room_id: room.id,
        })
        .await
        .unwrap();
    let non_member_party = Uuid::now_v7();
    let err = leave
        .execute(LeaveChatRoomCommand {
            actor_user_id: charlie(),
            actor_party_id: Some(non_member_party),
            room_id: room.id,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, ApplicationError::ChatRoomMembershipNotFound));
}

#[tokio::test]
async fn manage_membership_set_role_and_party_target() {
    let room_repo = Arc::new(FakeChatRoomRepo::default());
    let message_repo = Arc::new(FakeMessageRepo::default());
    let create = create_use_case(room_repo.clone(), message_repo.clone());
    let room = create
        .execute(CreateChatRoomCommand {
            actor_user_id: alice(),
            scopes: write_scopes(),
            name: "Managed Party".to_string(),
            description: None,
            room_type: ChatRoomType::Private,
        })
        .await
        .unwrap();

    let party_repo = Arc::new(FakePartyRepo::default());
    let party_id = make_party_with_member(&party_repo, bob(), "managed-party").await;

    let manage = ManageChatRoomMembership::new(room_repo.clone());

    // Add a party target.
    let added = manage
        .execute(ManageMembershipCommand {
            actor_user_id: alice(),
            scopes: moderate_scopes(),
            is_admin: false,
            room_id: room.id,
            action: MembershipAction::Add,
            target_user_id: None,
            target_party_id: Some(party_id),
            role: Some(ChatRoomMemberRole::Member),
            membership_id: None,
        })
        .await
        .unwrap();
    assert!(added.is_some());

    // Duplicate party membership is rejected.
    let err = manage
        .execute(ManageMembershipCommand {
            actor_user_id: alice(),
            scopes: moderate_scopes(),
            is_admin: false,
            room_id: room.id,
            action: MembershipAction::Add,
            target_user_id: None,
            target_party_id: Some(party_id),
            role: Some(ChatRoomMemberRole::Member),
            membership_id: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, ApplicationError::AlreadyChatRoomMember));

    // Set role requires membership_id and role.
    let membership_id = added.unwrap().id;
    manage
        .execute(ManageMembershipCommand {
            actor_user_id: alice(),
            scopes: moderate_scopes(),
            is_admin: false,
            room_id: room.id,
            action: MembershipAction::SetRole,
            target_user_id: None,
            target_party_id: None,
            role: Some(ChatRoomMemberRole::Moderator),
            membership_id: Some(membership_id),
        })
        .await
        .unwrap();

    // Adding without a target is rejected.
    let err = manage
        .execute(ManageMembershipCommand {
            actor_user_id: alice(),
            scopes: moderate_scopes(),
            is_admin: false,
            room_id: room.id,
            action: MembershipAction::Add,
            target_user_id: None,
            target_party_id: None,
            role: None,
            membership_id: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, ApplicationError::Validation(_)));

    // Removing the creator is forbidden for non-admins.
    let creator_membership = room_repo
        .find_membership_for_user(room.id, alice())
        .await
        .unwrap()
        .unwrap();
    let err = manage
        .execute(ManageMembershipCommand {
            actor_user_id: alice(),
            scopes: moderate_scopes(),
            is_admin: false,
            room_id: room.id,
            action: MembershipAction::Remove,
            target_user_id: None,
            target_party_id: None,
            role: None,
            membership_id: Some(creator_membership.id),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, ApplicationError::CannotManageChatRoom));
}
