use crate::repositories::{
    PostgresCatalogRepository, PostgresChatRoomRepository, PostgresMatchRepository,
    PostgresMessageRepository, PostgresPartyRepository, PostgresUserRepository,
};
use domain::entities::{
    ChatRoom, ChatRoomMemberRole, ChatRoomMembership, ChatRoomName, ChatRoomType, Conversation,
    DealRole, DisplayName, Email, Enhancement, MatchScoreBreakdown, MatchScoreWeights, MatchStatus,
    MatchSuggestion, Message, MessageReaction, MessageRead, MessageType, Need, Party,
    PartyMembershipRole, PartyType, PasswordHash, ReactionType, RecipientType, Resource,
    RoleProfile, User, UserPartyMembership, Username,
};
use domain::errors::DomainError;
use domain::repositories::{
    CatalogRepository, ChatRoomListQuery, ChatRoomRepository, MatchFilters, MatchRepository,
    MessageListQuery, MessageRepository, PartyRepository, UserRepository,
};
use rust_decimal::prelude::{Decimal, FromPrimitive};
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

async fn create_three_parties(pool: &PgPool) -> (Party, Party, Party) {
    let supplier = create_party(&pool, "match-supplier@example.com").await;
    let consumer = create_party(&pool, "match-consumer@example.com").await;
    let enhancer = create_party(&pool, "match-enhancer@example.com").await;
    (supplier, consumer, enhancer)
}

fn sample_match(
    supplier_id: Uuid,
    consumer_id: Uuid,
    enhancer_id: Uuid,
    score: Option<f64>,
) -> MatchSuggestion {
    let weights = MatchScoreWeights::default();
    let scores = score.map(|s| [s; 7]).unwrap_or([0.8; 7]);
    let breakdown = MatchScoreBreakdown::new(scores, weights);
    let mut suggestion = MatchSuggestion::new(
        Uuid::now_v7(),
        supplier_id,
        consumer_id,
        enhancer_id,
        breakdown,
        "Test match reason".to_string(),
    )
    .unwrap();
    suggestion.match_score = breakdown.total();
    suggestion
}

#[sqlx::test(migrations = "../../migrations")]
async fn match_create_and_find(pool: PgPool) {
    let (supplier, consumer, enhancer) = create_three_parties(&pool).await;
    let repo = PostgresMatchRepository::new(pool);
    let suggestion = sample_match(supplier.id, consumer.id, enhancer.id, None);

    repo.create(&suggestion).await.unwrap();

    let found = repo.find_by_id(suggestion.id).await.unwrap().unwrap();
    assert_eq!(found.id, suggestion.id);
    assert_eq!(found.supplier_party_id, supplier.id);
    assert_eq!(found.consumer_party_id, consumer.id);
    assert_eq!(found.enhancer_party_id, enhancer.id);
    assert_eq!(found.match_status, MatchStatus::Pending);
    assert!((found.match_score - suggestion.match_score).abs() < 1e-9);
}

#[sqlx::test(migrations = "../../migrations")]
async fn match_list_for_party_with_role_filter(pool: PgPool) {
    let (supplier, consumer, enhancer) = create_three_parties(&pool).await;
    let repo = PostgresMatchRepository::new(pool);
    let suggestion = sample_match(supplier.id, consumer.id, enhancer.id, None);
    repo.create(&suggestion).await.unwrap();

    let filters = MatchFilters {
        status: Some(MatchStatus::Pending),
        limit: 10,
        offset: 0,
        ..MatchFilters::default()
    };

    let supplier_list = repo
        .list_for_party(supplier.id, Some(DealRole::Supplier), &filters)
        .await
        .unwrap();
    assert_eq!(supplier_list.len(), 1);

    let consumer_list = repo
        .list_for_party(consumer.id, Some(DealRole::Consumer), &filters)
        .await
        .unwrap();
    assert_eq!(consumer_list.len(), 1);

    let enhancer_list = repo
        .list_for_party(enhancer.id, Some(DealRole::Enhancer), &filters)
        .await
        .unwrap();
    assert_eq!(enhancer_list.len(), 1);

    let wrong_role = repo
        .list_for_party(supplier.id, Some(DealRole::Consumer), &filters)
        .await
        .unwrap();
    assert!(wrong_role.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn match_status_lifecycle(pool: PgPool) {
    let (supplier, consumer, enhancer) = create_three_parties(&pool).await;
    let repo = PostgresMatchRepository::new(pool);
    let suggestion = sample_match(supplier.id, consumer.id, enhancer.id, None);
    repo.create(&suggestion).await.unwrap();

    repo.update_status(
        suggestion.id,
        MatchStatus::Accepted,
        Some("Looks good".to_string()),
    )
    .await
    .unwrap();
    let found = repo.find_by_id(suggestion.id).await.unwrap().unwrap();
    assert_eq!(found.match_status, MatchStatus::Accepted);
    assert!(found.responded_at.is_some());

    repo.update_counter_proposal(suggestion.id, None, Some("Lower price".to_string()))
        .await
        .unwrap();
    let found = repo.find_by_id(suggestion.id).await.unwrap().unwrap();
    assert_eq!(found.match_status, MatchStatus::CounterProposed);
    assert_eq!(found.counter_notes, Some("Lower price".to_string()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn match_count_by_status(pool: PgPool) {
    let (supplier, consumer, enhancer) = create_three_parties(&pool).await;
    let repo = PostgresMatchRepository::new(pool);
    let suggestion = sample_match(supplier.id, consumer.id, enhancer.id, None);
    repo.create(&suggestion).await.unwrap();

    let counts = repo.count_by_status(supplier.id).await.unwrap();
    assert_eq!(counts.pending, 1);
    assert_eq!(counts.accepted, 0);

    let all_counts = repo.count_all_by_status().await.unwrap();
    assert_eq!(all_counts.pending, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn match_delete_by_party(pool: PgPool) {
    let (supplier, consumer, enhancer) = create_three_parties(&pool).await;
    let repo = PostgresMatchRepository::new(pool);
    let suggestion = sample_match(supplier.id, consumer.id, enhancer.id, None);
    repo.create(&suggestion).await.unwrap();

    let deleted = repo.delete_by_party(supplier.id, None).await.unwrap();
    assert_eq!(deleted, 1);
    assert!(repo.find_by_id(suggestion.id).await.unwrap().is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn match_find_existing_pending(pool: PgPool) {
    let (supplier, consumer, enhancer) = create_three_parties(&pool).await;
    let repo = PostgresMatchRepository::new(pool);
    let suggestion = sample_match(supplier.id, consumer.id, enhancer.id, None);
    repo.create(&suggestion).await.unwrap();

    let existing = repo
        .find_existing_pending(supplier.id, consumer.id, enhancer.id)
        .await
        .unwrap();
    assert!(existing.is_some());

    repo.update_status(suggestion.id, MatchStatus::Accepted, None)
        .await
        .unwrap();
    let existing = repo
        .find_existing_pending(supplier.id, consumer.id, enhancer.id)
        .await
        .unwrap();
    assert!(existing.is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn match_list_all_filters_by_score(pool: PgPool) {
    let (supplier, consumer, enhancer) = create_three_parties(&pool).await;
    let repo = PostgresMatchRepository::new(pool);
    let high = sample_match(supplier.id, consumer.id, enhancer.id, Some(0.9));
    repo.create(&high).await.unwrap();

    let filters = MatchFilters {
        min_score: Some(0.95),
        limit: 10,
        offset: 0,
        ..MatchFilters::default()
    };
    let list = repo.list_all(&filters).await.unwrap();
    assert!(list.is_empty());

    let filters = MatchFilters {
        min_score: Some(0.85),
        limit: 10,
        offset: 0,
        ..MatchFilters::default()
    };
    let list = repo.list_all(&filters).await.unwrap();
    assert_eq!(list.len(), 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn match_update_counter_with_value(pool: PgPool) {
    let (supplier, consumer, enhancer) = create_three_parties(&pool).await;
    let repo = PostgresMatchRepository::new(pool);
    let suggestion = sample_match(supplier.id, consumer.id, enhancer.id, None);
    repo.create(&suggestion).await.unwrap();

    let value = Decimal::from_f64(123.45).unwrap();
    repo.update_counter_proposal(
        suggestion.id,
        Some(value),
        Some("Counter offer".to_string()),
    )
    .await
    .unwrap();

    let found = repo.find_by_id(suggestion.id).await.unwrap().unwrap();
    assert_eq!(found.match_status, MatchStatus::CounterProposed);
    assert_eq!(found.suggested_deal_value, Some(value));
    assert_eq!(found.counter_notes, Some("Counter offer".to_string()));
    assert!(found.responded_at.is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn match_set_converted_deal(pool: PgPool) {
    let (supplier, consumer, enhancer) = create_three_parties(&pool).await;
    let repo = PostgresMatchRepository::new(pool.clone());
    let suggestion = sample_match(supplier.id, consumer.id, enhancer.id, None);
    repo.create(&suggestion).await.unwrap();

    let category = create_category(&pool, "Converted Deal Category", "RESOURCE_TYPE").await;
    let deal_id = Uuid::now_v7();
    sqlx::query!(
        r#"
        INSERT INTO deals (
            id, deal_reference, deal_title, domain_category_id, initiator_party_id, initiator_role, deal_status
        )
        VALUES ($1, $2, $3, $4, $5, 'SUPPLIER', 'DRAFT')
        "#,
        deal_id,
        format!("DEAL-{}", deal_id),
        "Converted deal",
        category,
        supplier.id
    )
    .execute(&pool)
    .await
    .unwrap();

    repo.set_converted_deal(suggestion.id, deal_id)
        .await
        .unwrap();

    let found = repo.find_by_id(suggestion.id).await.unwrap().unwrap();
    assert_eq!(found.match_status, MatchStatus::ConvertedToDeal);
    assert_eq!(found.converted_deal_id, Some(deal_id));
}

#[sqlx::test(migrations = "../../migrations")]
async fn match_delete_by_party_with_status_filter(pool: PgPool) {
    let (supplier, consumer, enhancer) = create_three_parties(&pool).await;
    let repo = PostgresMatchRepository::new(pool);
    let suggestion = sample_match(supplier.id, consumer.id, enhancer.id, None);
    repo.create(&suggestion).await.unwrap();

    repo.update_status(suggestion.id, MatchStatus::Accepted, None)
        .await
        .unwrap();

    let pending_deleted = repo
        .delete_by_party(supplier.id, Some(MatchStatus::Pending))
        .await
        .unwrap();
    assert_eq!(pending_deleted, 0);
    assert!(repo.find_by_id(suggestion.id).await.unwrap().is_some());

    let accepted_deleted = repo
        .delete_by_party(supplier.id, Some(MatchStatus::Accepted))
        .await
        .unwrap();
    assert_eq!(accepted_deleted, 1);
    assert!(repo.find_by_id(suggestion.id).await.unwrap().is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn match_delete_all(pool: PgPool) {
    let (supplier, consumer, enhancer) = create_three_parties(&pool).await;
    let repo = PostgresMatchRepository::new(pool);
    let first = sample_match(supplier.id, consumer.id, enhancer.id, None);
    let mut second = sample_match(supplier.id, consumer.id, enhancer.id, None);
    second.id = Uuid::now_v7();
    repo.create(&first).await.unwrap();
    repo.create(&second).await.unwrap();

    let deleted = repo.delete_all().await.unwrap();
    assert_eq!(deleted, 2);
    assert!(repo.find_by_id(first.id).await.unwrap().is_none());
    assert!(repo.find_by_id(second.id).await.unwrap().is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn match_list_all_filters_by_status_and_generated_by(pool: PgPool) {
    let (supplier, consumer, enhancer) = create_three_parties(&pool).await;
    let repo = PostgresMatchRepository::new(pool);
    let suggestion = sample_match(supplier.id, consumer.id, enhancer.id, None);
    repo.create(&suggestion).await.unwrap();

    let accepted_filters = MatchFilters {
        status: Some(MatchStatus::Accepted),
        limit: 10,
        offset: 0,
        ..MatchFilters::default()
    };
    assert!(repo.list_all(&accepted_filters).await.unwrap().is_empty());

    let algorithm_filters = MatchFilters {
        generated_by: Some("ALGORITHM".to_string()),
        limit: 10,
        offset: 0,
        ..MatchFilters::default()
    };
    assert_eq!(repo.list_all(&algorithm_filters).await.unwrap().len(), 1);

    let admin_filters = MatchFilters {
        generated_by: Some("PLATFORM_ADMIN".to_string()),
        limit: 10,
        offset: 0,
        ..MatchFilters::default()
    };
    assert!(repo.list_all(&admin_filters).await.unwrap().is_empty());
}

async fn create_category(pool: &PgPool, name: &str, category_type: &str) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        r#"
        INSERT INTO categories (id, category_name, category_code, category_type, created_at, updated_at)
        VALUES ($1, $2, $3, $4, now(), now())
        "#,
        id,
        name,
        name.to_lowercase().replace(' ', "_"),
        category_type
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

#[sqlx::test(migrations = "../../migrations")]
async fn match_generate_use_case_creates_suggestions(pool: PgPool) {
    let user = create_user(&pool, "match_gen_user@example.com", "match_gen_user").await;
    let supplier = create_party(&pool, "match_gen_supplier@example.com").await;
    let consumer = create_party(&pool, "match_gen_consumer@example.com").await;
    let enhancer = create_party(&pool, "match_gen_enhancer@example.com").await;
    add_party_member(&pool, user.id, supplier.id).await;
    add_party_member(&pool, user.id, consumer.id).await;
    add_party_member(&pool, user.id, enhancer.id).await;

    let party_repo = PostgresPartyRepository::new(pool.clone());
    party_repo
        .add_role(
            supplier.id,
            DealRole::Supplier,
            RoleProfile::for_role(DealRole::Supplier),
        )
        .await
        .unwrap();
    party_repo
        .add_role(
            consumer.id,
            DealRole::Consumer,
            RoleProfile::for_role(DealRole::Consumer),
        )
        .await
        .unwrap();
    party_repo
        .add_role(
            enhancer.id,
            DealRole::Enhancer,
            RoleProfile::for_role(DealRole::Enhancer),
        )
        .await
        .unwrap();

    let category = create_category(&pool, "Match Category", "RESOURCE_TYPE").await;

    let catalog_repo = PostgresCatalogRepository::new(pool.clone());
    let resource = Resource::new(
        Uuid::now_v7(),
        supplier.id,
        category,
        "Test Resource".to_string(),
        rust_decimal::Decimal::from(10),
        "unit".to_string(),
    )
    .unwrap();
    catalog_repo.create_resource(&resource).await.unwrap();

    let need = Need::new(
        Uuid::now_v7(),
        consumer.id,
        category,
        "Test Need Description".to_string(),
        rust_decimal::Decimal::from(5),
        "unit".to_string(),
    )
    .unwrap();
    catalog_repo.create_need(&need).await.unwrap();

    let enhancement = Enhancement::new(
        Uuid::now_v7(),
        enhancer.id,
        category,
        "Test Enhancement".to_string(),
    )
    .unwrap();
    catalog_repo.create_enhancement(&enhancement).await.unwrap();

    let match_repo: std::sync::Arc<dyn MatchRepository> =
        std::sync::Arc::new(PostgresMatchRepository::new(pool.clone()));
    let party_repo: std::sync::Arc<dyn PartyRepository> =
        std::sync::Arc::new(PostgresPartyRepository::new(pool.clone()));
    let catalog_repo: std::sync::Arc<dyn CatalogRepository> =
        std::sync::Arc::new(PostgresCatalogRepository::new(pool.clone()));

    let generate = application::matching::GenerateMatches::new(
        match_repo.clone(),
        party_repo.clone(),
        catalog_repo.clone(),
    );

    let cmd = application::matching::dto::GenerateMatchesCommand {
        actor_user_id: user.id,
        actor_party_id: Some(supplier.id),
        is_admin: true,
        min_score: None,
        max_suggestions: Some(10),
        weights: None,
    };

    let suggestions = generate.execute(cmd).await.unwrap();
    assert_eq!(suggestions.len(), 1);
    assert_eq!(suggestions[0].supplier_party_id, supplier.id);
    assert_eq!(suggestions[0].consumer_party_id, consumer.id);
    assert_eq!(suggestions[0].enhancer_party_id, enhancer.id);

    // List for the consumer party.
    let list_matches =
        application::matching::ListMatches::new(match_repo.clone(), party_repo.clone());
    let query = application::matching::dto::ListMatchesQuery {
        party_id: Some(consumer.id),
        role: None,
        status: None,
        min_score: None,
        max_score: None,
        limit: 10,
        offset: 0,
    };
    let listed = list_matches
        .execute(user.id, Some(consumer.id), false, query)
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);

    // Respond as supplier.
    let respond =
        application::matching::RespondToMatch::new(match_repo.clone(), party_repo.clone());
    let response_cmd = application::matching::dto::RespondToMatchCommand {
        actor_user_id: user.id,
        actor_party_id: supplier.id,
        match_suggestion_id: suggestions[0].id,
        response: application::matching::dto::MatchResponseAction::Accept,
        notes: None,
        counter_value: None,
    };
    respond.execute(response_cmd).await.unwrap();

    let admin_controls = application::matching::AdminMatchControls::new(match_repo.clone());
    let counts = admin_controls.count_for_party(supplier.id).await.unwrap();
    assert_eq!(counts.accepted, 1);
    assert_eq!(counts.pending, 0);

    // Admin updates the suggestion status.
    let update_cmd = application::matching::dto::AdminUpdateMatchCommand {
        admin_user_id: user.id,
        match_suggestion_id: suggestions[0].id,
        new_status: domain::entities::MatchStatus::Declined,
        reason: Some("No longer relevant".to_string()),
    };
    admin_controls.update_status(update_cmd).await.unwrap();
    let found = match_repo
        .find_by_id(suggestions[0].id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.match_status, domain::entities::MatchStatus::Declined);

    // Admin deletes suggestions for the consumer party.
    let delete_cmd = application::matching::dto::AdminDeleteMatchesCommand {
        admin_user_id: user.id,
        party_id: consumer.id,
        status: None,
    };
    let deleted = admin_controls.delete_for_party(delete_cmd).await.unwrap();
    assert_eq!(deleted, 1);

    // Admin deletes any remaining suggestions (covers the delete_all path).
    let all_deleted = admin_controls.delete_all(user.id).await.unwrap();
    assert_eq!(all_deleted, 0);
}
