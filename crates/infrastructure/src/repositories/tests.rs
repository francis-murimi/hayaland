use crate::repositories::{
    PostgresChatRoomRepository, PostgresMessageRepository, PostgresPartyRepository,
    PostgresUserRepository,
};
use domain::entities::{
    ChatRoom, ChatRoomMemberRole, ChatRoomMembership, ChatRoomName, ChatRoomType, Conversation,
    DisplayName, Email, Message, MessageReaction, MessageRead, MessageType, Party,
    PartyMembershipRole, PartyType, PasswordHash, ReactionType, RecipientType, User,
    UserPartyMembership, Username,
};
use domain::errors::DomainError;
use domain::repositories::{
    ChatRoomListQuery, ChatRoomRepository, MessageListQuery, MessageRepository, PartyRepository,
    UserRepository,
};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

fn sample_user(email: &str, username: &str) -> User {
    User::new(
        Uuid::now_v7(),
        Email::new(email).unwrap(),
        Username::new(username).unwrap(),
        PasswordHash::new(format!("hash-{username}")).unwrap(),
    )
}

fn sample_party(email: &str) -> Party {
    Party::new(
        Uuid::now_v7(),
        PartyType::Organization,
        DisplayName::new("Green Acres Farm").unwrap(),
        Email::new(email).unwrap(),
    )
}

async fn create_user(pool: &PgPool, email: &str, username: &str) -> User {
    let repo = PostgresUserRepository::new(pool.clone());
    let user = sample_user(email, username);
    repo.create(&user).await.unwrap();
    user
}

async fn create_party(pool: &PgPool, email: &str) -> Party {
    let repo = PostgresPartyRepository::new(pool.clone());
    let party = sample_party(email);
    repo.create(&party).await.unwrap();
    party
}

async fn add_party_member(pool: &PgPool, user_id: Uuid, party_id: Uuid) {
    let repo = PostgresPartyRepository::new(pool.clone());
    let membership = UserPartyMembership::new(
        Uuid::now_v7(),
        user_id,
        party_id,
        PartyMembershipRole::Member,
    );
    repo.add_membership(&membership).await.unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
async fn message_create_and_find_conversation(pool: PgPool) {
    let a = create_user(&pool, "conv_a@example.com", "conv_a").await.id;
    let b = create_user(&pool, "conv_b@example.com", "conv_b").await.id;
    let repo = PostgresMessageRepository::new(pool);
    let conv = Conversation::new_direct_user(Uuid::now_v7(), a, b);

    repo.create_conversation(&conv).await.unwrap();

    let found = repo.find_conversation_by_id(conv.id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, conv.id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn message_find_direct_user_conversation(pool: PgPool) {
    let a = create_user(&pool, "dir_a@example.com", "dir_a").await.id;
    let b = create_user(&pool, "dir_b@example.com", "dir_b").await.id;
    let repo = PostgresMessageRepository::new(pool);
    let conv = Conversation::new_direct_user(Uuid::now_v7(), a, b);

    repo.create_conversation(&conv).await.unwrap();

    let found = repo.find_direct_user_conversation(a, b).await.unwrap();
    assert!(found.is_some());
    let found = repo.find_direct_user_conversation(b, a).await.unwrap();
    assert!(found.is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn message_create_and_find(pool: PgPool) {
    let sender = create_user(&pool, "snd@example.com", "sender").await.id;
    let recipient = create_user(&pool, "rcp@example.com", "recipient").await.id;
    let repo = PostgresMessageRepository::new(pool);
    let conv = Conversation::new_direct_user(Uuid::now_v7(), sender, recipient);
    repo.create_conversation(&conv).await.unwrap();

    let message = Message::new(
        Uuid::now_v7(),
        conv.id,
        sender,
        None,
        RecipientType::User,
        Some(recipient),
        None,
        None,
        None,
        MessageType::Text,
        None,
        "hello".to_string(),
        vec![],
        None,
    )
    .unwrap();
    repo.create_message(&message).await.unwrap();

    let found = repo.find_message_by_id(message.id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().content, "hello");
}

#[sqlx::test(migrations = "../../migrations")]
async fn message_list_and_pagination(pool: PgPool) {
    let sender = create_user(&pool, "pag_s@example.com", "pag_sender")
        .await
        .id;
    let recipient = create_user(&pool, "pag_r@example.com", "pag_recipient")
        .await
        .id;
    let repo = PostgresMessageRepository::new(pool);
    let conv = Conversation::new_direct_user(Uuid::now_v7(), sender, recipient);
    repo.create_conversation(&conv).await.unwrap();

    for i in 0..5 {
        let mut message = Message::new(
            Uuid::now_v7(),
            conv.id,
            sender,
            None,
            RecipientType::User,
            Some(recipient),
            None,
            None,
            None,
            MessageType::Text,
            None,
            format!("msg-{i}"),
            vec![],
            None,
        )
        .unwrap();
        // Stagger created_at so ordering is deterministic.
        message.created_at = OffsetDateTime::now_utc() + time::Duration::seconds(i);
        repo.create_message(&message).await.unwrap();
    }

    let all = repo
        .list_messages(
            conv.id,
            &MessageListQuery {
                before_id: None,
                limit: 10,
            },
        )
        .await
        .unwrap();
    assert_eq!(all.len(), 5);
    assert_eq!(all[0].message.content, "msg-4");

    let page = repo
        .list_messages(
            conv.id,
            &MessageListQuery {
                before_id: Some(all[0].message.id),
                limit: 10,
            },
        )
        .await
        .unwrap();
    assert_eq!(page.len(), 4);
}

#[sqlx::test(migrations = "../../migrations")]
async fn message_edit(pool: PgPool) {
    let sender = create_user(&pool, "edt_s@example.com", "edt_sender")
        .await
        .id;
    let recipient = create_user(&pool, "edt_r@example.com", "edt_recipient")
        .await
        .id;
    let repo = PostgresMessageRepository::new(pool);
    let conv = Conversation::new_direct_user(Uuid::now_v7(), sender, recipient);
    repo.create_conversation(&conv).await.unwrap();

    let mut message = Message::new(
        Uuid::now_v7(),
        conv.id,
        sender,
        None,
        RecipientType::User,
        Some(recipient),
        None,
        None,
        None,
        MessageType::Text,
        None,
        "original".to_string(),
        vec![],
        None,
    )
    .unwrap();
    repo.create_message(&message).await.unwrap();

    message.edit("updated".to_string());
    repo.update_message(&message).await.unwrap();

    let found = repo.find_message_by_id(message.id).await.unwrap().unwrap();
    assert_eq!(found.content, "updated");
    assert!(found.edited_at.is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn message_soft_delete(pool: PgPool) {
    let sender = create_user(&pool, "del_s@example.com", "del_sender")
        .await
        .id;
    let recipient = create_user(&pool, "del_r@example.com", "del_recipient")
        .await
        .id;
    let repo = PostgresMessageRepository::new(pool);
    let conv = Conversation::new_direct_user(Uuid::now_v7(), sender, recipient);
    repo.create_conversation(&conv).await.unwrap();

    let message = Message::new(
        Uuid::now_v7(),
        conv.id,
        sender,
        None,
        RecipientType::User,
        Some(recipient),
        None,
        None,
        None,
        MessageType::Text,
        None,
        "secret".to_string(),
        vec![],
        None,
    )
    .unwrap();
    repo.create_message(&message).await.unwrap();

    repo.soft_delete_message(message.id).await.unwrap();

    let found = repo.find_message_by_id(message.id).await.unwrap().unwrap();
    assert!(found.is_deleted);
}

#[sqlx::test(migrations = "../../migrations")]
async fn message_pin_and_unpin(pool: PgPool) {
    let sender = create_user(&pool, "pin_s@example.com", "pin_sender")
        .await
        .id;
    let recipient = create_user(&pool, "pin_r@example.com", "pin_recipient")
        .await
        .id;
    let repo = PostgresMessageRepository::new(pool);
    let conv = Conversation::new_direct_user(Uuid::now_v7(), sender, recipient);
    repo.create_conversation(&conv).await.unwrap();

    let message = Message::new(
        Uuid::now_v7(),
        conv.id,
        sender,
        None,
        RecipientType::User,
        Some(recipient),
        None,
        None,
        None,
        MessageType::Text,
        None,
        "pinned".to_string(),
        vec![],
        None,
    )
    .unwrap();
    repo.create_message(&message).await.unwrap();

    let now = OffsetDateTime::now_utc();
    repo.set_message_pinned(message.id, true, Some(now))
        .await
        .unwrap();

    let pinned = repo.list_pinned_messages(conv.id).await.unwrap();
    assert_eq!(pinned.len(), 1);
    assert!(pinned[0].message.is_pinned);

    repo.set_message_pinned(message.id, false, None)
        .await
        .unwrap();
    let pinned = repo.list_pinned_messages(conv.id).await.unwrap();
    assert!(pinned.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn message_read_receipt(pool: PgPool) {
    let sender = create_user(&pool, "read_s@example.com", "read_sender")
        .await
        .id;
    let recipient = create_user(&pool, "read_r@example.com", "read_recipient")
        .await
        .id;
    let repo = PostgresMessageRepository::new(pool);
    let conv = Conversation::new_direct_user(Uuid::now_v7(), sender, recipient);
    repo.create_conversation(&conv).await.unwrap();

    let message = Message::new(
        Uuid::now_v7(),
        conv.id,
        sender,
        None,
        RecipientType::User,
        Some(recipient),
        None,
        None,
        None,
        MessageType::Text,
        None,
        "read me".to_string(),
        vec![],
        None,
    )
    .unwrap();
    repo.create_message(&message).await.unwrap();

    let existing = repo.find_read(message.id, recipient).await.unwrap();
    assert!(existing.is_none());

    let read = MessageRead::new(Uuid::now_v7(), message.id, recipient, None);
    repo.mark_read(&read).await.unwrap();

    let found = repo.find_read(message.id, recipient).await.unwrap();
    assert!(found.is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn message_reaction_toggle(pool: PgPool) {
    let sender = create_user(&pool, "react_s@example.com", "react_sender")
        .await
        .id;
    let recipient = create_user(&pool, "react_r@example.com", "react_recipient")
        .await
        .id;
    let repo = PostgresMessageRepository::new(pool);
    let conv = Conversation::new_direct_user(Uuid::now_v7(), sender, recipient);
    repo.create_conversation(&conv).await.unwrap();

    let message = Message::new(
        Uuid::now_v7(),
        conv.id,
        sender,
        None,
        RecipientType::User,
        Some(recipient),
        None,
        None,
        None,
        MessageType::Text,
        None,
        "react".to_string(),
        vec![],
        None,
    )
    .unwrap();
    repo.create_message(&message).await.unwrap();

    let reaction = MessageReaction::new(
        Uuid::now_v7(),
        message.id,
        recipient,
        None,
        ReactionType::Like,
    );
    let added = repo.toggle_reaction(&reaction).await.unwrap();
    assert!(added.is_some());

    let list = repo.list_reactions_for_message(message.id).await.unwrap();
    assert_eq!(list.len(), 1);

    let removed = repo.toggle_reaction(&reaction).await.unwrap();
    assert!(removed.is_none());

    let list = repo.list_reactions_for_message(message.id).await.unwrap();
    assert!(list.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn message_unread_count_and_conversation_list(pool: PgPool) {
    let alice = create_user(&pool, "alice@example.com", "alice").await.id;
    let bob = create_user(&pool, "bob@example.com", "bob").await.id;
    let repo = PostgresMessageRepository::new(pool);

    let conv = Conversation::new_direct_user(Uuid::now_v7(), alice, bob);
    repo.create_conversation(&conv).await.unwrap();

    let message = Message::new(
        Uuid::now_v7(),
        conv.id,
        alice,
        None,
        RecipientType::User,
        Some(bob),
        None,
        None,
        None,
        MessageType::Text,
        None,
        "unread".to_string(),
        vec![],
        None,
    )
    .unwrap();
    repo.create_message(&message).await.unwrap();

    let unread = repo.unread_count_for_user(bob, None).await.unwrap();
    assert_eq!(unread, 1);

    let conversations = repo
        .list_conversations_for_user(bob, None, 10, 0)
        .await
        .unwrap();
    assert_eq!(conversations.len(), 1);
    assert_eq!(conversations[0].unread_count, 1);

    repo.mark_read(&MessageRead::new(Uuid::now_v7(), message.id, bob, None))
        .await
        .unwrap();

    let unread = repo.unread_count_for_user(bob, None).await.unwrap();
    assert_eq!(unread, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn message_duplicate_conversation_is_rejected(pool: PgPool) {
    let a = create_user(&pool, "dup_a@example.com", "dup_a").await.id;
    let b = create_user(&pool, "dup_b@example.com", "dup_b").await.id;
    let repo = PostgresMessageRepository::new(pool);
    let conv = Conversation::new_direct_user(Uuid::now_v7(), a, b);
    repo.create_conversation(&conv).await.unwrap();

    let duplicate = Conversation::new_direct_user(Uuid::now_v7(), a, b);
    let err = repo.create_conversation(&duplicate).await.unwrap_err();
    assert!(matches!(err, DomainError::RepositoryError(_)));
}

#[sqlx::test(migrations = "../../migrations")]
async fn chatroom_create_and_find(pool: PgPool) {
    let creator = create_user(&pool, "creator@example.com", "creator")
        .await
        .id;
    let repo = PostgresChatRoomRepository::new(pool);
    let room = ChatRoom::new(
        Uuid::now_v7(),
        ChatRoomName::new("General").unwrap(),
        Some("public room".to_string()),
        ChatRoomType::Public,
        creator,
    );

    repo.create_room(&room).await.unwrap();

    let found = repo.find_room_by_id(room.id).await.unwrap();
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.name.as_str(), "General");
    assert_eq!(found.room_type, ChatRoomType::Public);

    let by_name = repo.find_room_by_name("General").await.unwrap();
    assert!(by_name.is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn chatroom_update_and_soft_delete(pool: PgPool) {
    let creator = create_user(&pool, "upd_creator@example.com", "upd_creator")
        .await
        .id;
    let repo = PostgresChatRoomRepository::new(pool);
    let mut room = ChatRoom::new(
        Uuid::now_v7(),
        ChatRoomName::new("Old Name").unwrap(),
        None,
        ChatRoomType::Public,
        creator,
    );
    repo.create_room(&room).await.unwrap();

    room.update(
        Some(ChatRoomName::new("New Name").unwrap()),
        Some("desc".to_string()),
    );
    repo.update_room(&room).await.unwrap();

    let found = repo.find_room_by_id(room.id).await.unwrap().unwrap();
    assert_eq!(found.name.as_str(), "New Name");
    assert_eq!(found.description, Some("desc".to_string()));

    repo.soft_delete_room(room.id).await.unwrap();
    let found = repo.find_room_by_id(room.id).await.unwrap().unwrap();
    assert!(found.is_deleted);
}

#[sqlx::test(migrations = "../../migrations")]
async fn chatroom_list_and_count(pool: PgPool) {
    let creator = create_user(&pool, "list_creator@example.com", "list_creator")
        .await
        .id;
    let repo = PostgresChatRoomRepository::new(pool);

    let public = ChatRoom::new(
        Uuid::now_v7(),
        ChatRoomName::new("Public Room").unwrap(),
        None,
        ChatRoomType::Public,
        creator,
    );
    let private = ChatRoom::new(
        Uuid::now_v7(),
        ChatRoomName::new("Private Room").unwrap(),
        None,
        ChatRoomType::Private,
        creator,
    );
    repo.create_room(&public).await.unwrap();
    repo.create_room(&private).await.unwrap();

    let mut query = ChatRoomListQuery::default();
    query.limit = 10;
    let visible = vec![private.id];
    let rooms = repo.list_rooms(&query, &visible).await.unwrap();
    assert_eq!(rooms.len(), 2);

    let count = repo.count_rooms(&query, &visible).await.unwrap();
    assert_eq!(count, 2);

    let empty_visible: Vec<Uuid> = vec![];
    let rooms = repo.list_rooms(&query, &empty_visible).await.unwrap();
    assert_eq!(rooms.len(), 1);
    assert_eq!(rooms[0].id, public.id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn chatroom_membership_lifecycle(pool: PgPool) {
    let creator = create_user(&pool, "mem_creator@example.com", "mem_creator")
        .await
        .id;
    let repo = PostgresChatRoomRepository::new(pool);
    let room = ChatRoom::new(
        Uuid::now_v7(),
        ChatRoomName::new("Members").unwrap(),
        None,
        ChatRoomType::Public,
        creator,
    );
    repo.create_room(&room).await.unwrap();

    let membership =
        ChatRoomMembership::for_user(Uuid::now_v7(), room.id, creator, ChatRoomMemberRole::Member);
    repo.add_membership(&membership).await.unwrap();

    let found = repo
        .find_membership_for_user(room.id, creator)
        .await
        .unwrap();
    assert!(found.is_some());

    let memberships = repo.list_memberships_for_room(room.id).await.unwrap();
    assert_eq!(memberships.len(), 1);

    repo.update_membership_role(membership.id, ChatRoomMemberRole::Moderator)
        .await
        .unwrap();
    let updated = repo
        .find_membership_by_id(membership.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.member_role, ChatRoomMemberRole::Moderator);

    repo.remove_membership(membership.id).await.unwrap();
    let found = repo.find_membership_by_id(membership.id).await.unwrap();
    assert!(found.is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn chatroom_user_and_party_access(pool: PgPool) {
    let user = create_user(&pool, "member@example.com", "member").await;
    let party = create_party(&pool, "member-party@example.com").await;
    add_party_member(&pool, user.id, party.id).await;

    let room_repo = PostgresChatRoomRepository::new(pool);
    let room = ChatRoom::new(
        Uuid::now_v7(),
        ChatRoomName::new("Access Test").unwrap(),
        None,
        ChatRoomType::Private,
        user.id,
    );
    room_repo.create_room(&room).await.unwrap();

    let user_membership =
        ChatRoomMembership::for_user(Uuid::now_v7(), room.id, user.id, ChatRoomMemberRole::Member);
    room_repo.add_membership(&user_membership).await.unwrap();

    let party_membership = ChatRoomMembership::for_party(
        Uuid::now_v7(),
        room.id,
        party.id,
        ChatRoomMemberRole::Member,
    );
    room_repo.add_membership(&party_membership).await.unwrap();

    let ids = room_repo
        .list_room_ids_for_user(user.id, &[party.id])
        .await
        .unwrap();
    assert!(ids.contains(&room.id));

    assert!(room_repo
        .is_user_in_room(room.id, user.id, &[party.id])
        .await
        .unwrap());
    assert!(room_repo
        .is_party_in_room(room.id, &[party.id])
        .await
        .unwrap());

    let rooms = room_repo
        .list_rooms_for_user(user.id, &[party.id])
        .await
        .unwrap();
    assert_eq!(rooms.len(), 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn chatroom_duplicate_name_is_rejected(pool: PgPool) {
    let creator = create_user(&pool, "dup_creator@example.com", "dup_creator")
        .await
        .id;
    let repo = PostgresChatRoomRepository::new(pool);
    let room = ChatRoom::new(
        Uuid::now_v7(),
        ChatRoomName::new("Unique").unwrap(),
        None,
        ChatRoomType::Public,
        creator,
    );
    repo.create_room(&room).await.unwrap();

    let duplicate = ChatRoom::new(
        Uuid::now_v7(),
        ChatRoomName::new("Unique").unwrap(),
        None,
        ChatRoomType::Public,
        creator,
    );
    let err = repo.create_room(&duplicate).await.unwrap_err();
    assert!(matches!(err, DomainError::ChatRoomAlreadyExists));
}
