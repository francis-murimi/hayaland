use domain::entities::{
    Conversation, Email, Message, MessageRead, PasswordHash, RecipientType, User, Username,
};
use domain::repositories::{MessageListQuery, MessageRepository, UserRepository};
use infrastructure::repositories::{PostgresMessageRepository, PostgresUserRepository};
use sqlx::PgPool;
use uuid::Uuid;

fn sample_user(email: &str, username: &str) -> User {
    User::new(
        Uuid::now_v7(),
        Email::new(email).unwrap(),
        Username::new(username).unwrap(),
        PasswordHash::new(format!("hash-{username}")).unwrap(),
    )
}

#[sqlx::test(migrations = "../../migrations")]
async fn creates_and_finds_direct_user_conversation(pool: PgPool) {
    let user_repo = PostgresUserRepository::new(pool.clone());
    let repo = PostgresMessageRepository::new(pool);

    let user_a = sample_user("msg-a@example.com", "msg_a");
    let user_b = sample_user("msg-b@example.com", "msg_b");
    let user_a_id = user_a.id;
    let user_b_id = user_b.id;

    user_repo.create(&user_a).await.unwrap();
    user_repo.create(&user_b).await.unwrap();

    let conversation = Conversation::new_direct_user(Uuid::now_v7(), user_a_id, user_b_id);
    let id = conversation.id;
    repo.create_conversation(&conversation).await.unwrap();

    let found = repo.find_conversation_by_id(id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, id);

    let direct = repo
        .find_direct_user_conversation(user_a_id, user_b_id)
        .await
        .unwrap();
    assert!(direct.is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn sends_and_lists_messages(pool: PgPool) {
    let user_repo = PostgresUserRepository::new(pool.clone());
    let repo = PostgresMessageRepository::new(pool);

    let user_a = sample_user("msg-sender@example.com", "msg_sender");
    let user_b = sample_user("msg-recipient@example.com", "msg_recipient");
    let user_a_id = user_a.id;
    let user_b_id = user_b.id;

    user_repo.create(&user_a).await.unwrap();
    user_repo.create(&user_b).await.unwrap();

    let conversation = Conversation::new_direct_user(Uuid::now_v7(), user_a_id, user_b_id);
    repo.create_conversation(&conversation).await.unwrap();

    let message = Message::new(
        Uuid::now_v7(),
        conversation.id,
        user_a_id,
        None,
        RecipientType::User,
        Some(user_b_id),
        None,
        None,
        None,
        domain::entities::MessageType::Text,
        None,
        "Hello".into(),
        vec![],
        None,
    )
    .unwrap();
    let message_id = message.id;

    repo.create_message(&message).await.unwrap();

    let found = repo.find_message_by_id(message_id).await.unwrap();
    assert!(found.is_some());

    let query = MessageListQuery {
        before_id: None,
        limit: 50,
    };
    let messages = repo.list_messages(conversation.id, &query).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].message.content, "Hello");

    repo.mark_read(&MessageRead::new(
        Uuid::now_v7(),
        message_id,
        user_b_id,
        None,
    ))
    .await
    .unwrap();

    let messages = repo.list_messages(conversation.id, &query).await.unwrap();
    assert_eq!(messages[0].read_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn updates_and_soft_deletes_message(pool: PgPool) {
    let user_repo = PostgresUserRepository::new(pool.clone());
    let repo = PostgresMessageRepository::new(pool);

    let user_a = sample_user("msg-update-a@example.com", "msg_update_a");
    let user_b = sample_user("msg-update-b@example.com", "msg_update_b");
    let user_a_id = user_a.id;
    let user_b_id = user_b.id;

    user_repo.create(&user_a).await.unwrap();
    user_repo.create(&user_b).await.unwrap();

    let conversation = Conversation::new_direct_user(Uuid::now_v7(), user_a_id, user_b_id);
    repo.create_conversation(&conversation).await.unwrap();

    let mut message = Message::new(
        Uuid::now_v7(),
        conversation.id,
        user_a_id,
        None,
        RecipientType::User,
        Some(user_b_id),
        None,
        None,
        None,
        domain::entities::MessageType::Text,
        None,
        "Hello".into(),
        vec![],
        None,
    )
    .unwrap();
    repo.create_message(&message).await.unwrap();

    message.edit("Edited".into());
    repo.update_message(&message).await.unwrap();

    let found = repo.find_message_by_id(message.id).await.unwrap().unwrap();
    assert_eq!(found.content, "Edited");
    assert!(found.edited_at.is_some());

    repo.set_message_pinned(message.id, true, Some(time::OffsetDateTime::now_utc()))
        .await
        .unwrap();
    let pinned = repo.list_pinned_messages(conversation.id).await.unwrap();
    assert_eq!(pinned.len(), 1);

    repo.soft_delete_message(message.id).await.unwrap();
    let found = repo.find_message_by_id(message.id).await.unwrap().unwrap();
    assert!(found.is_deleted);
}

#[sqlx::test(migrations = "../../migrations")]
async fn lists_conversations_for_user(pool: PgPool) {
    let user_repo = PostgresUserRepository::new(pool.clone());
    let repo = PostgresMessageRepository::new(pool);

    let user_a = sample_user("msg-conv-a@example.com", "msg_conv_a");
    let user_b = sample_user("msg-conv-b@example.com", "msg_conv_b");
    let user_a_id = user_a.id;
    let user_b_id = user_b.id;

    user_repo.create(&user_a).await.unwrap();
    user_repo.create(&user_b).await.unwrap();

    let conversation = Conversation::new_direct_user(Uuid::now_v7(), user_a_id, user_b_id);
    repo.create_conversation(&conversation).await.unwrap();

    let conversations = repo
        .list_conversations_for_user(user_a_id, None, 10, 0)
        .await
        .unwrap();
    assert_eq!(conversations.len(), 1);

    let unread = repo.unread_count_for_user(user_a_id, None).await.unwrap();
    assert_eq!(unread, 0);
}
