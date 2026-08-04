use domain::entities::{
    ChatRoom, ChatRoomMemberRole, ChatRoomMembership, ChatRoomName, ChatRoomType,
};
use domain::entities::{Email, PasswordHash, User, Username};
use domain::repositories::{ChatRoomListQuery, ChatRoomRepository, UserRepository};
use infrastructure::repositories::PostgresChatRoomRepository;
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

async fn create_user(pool: &PgPool) -> Uuid {
    let user = sample_user(&format!("user-{}@example.com", Uuid::now_v7()), "chat_user");
    let id = user.id;
    let repo = infrastructure::repositories::PostgresUserRepository::new(pool.clone());
    repo.create(&user).await.unwrap();
    id
}

fn sample_room(created_by: Uuid) -> ChatRoom {
    ChatRoom::new(
        Uuid::now_v7(),
        ChatRoomName::new("Test Room").unwrap(),
        Some("a room for tests".into()),
        ChatRoomType::Public,
        created_by,
    )
}

#[sqlx::test(migrations = "../../migrations")]
async fn creates_and_finds_room_by_id(pool: PgPool) {
    let user_id = create_user(&pool).await;
    let repo = PostgresChatRoomRepository::new(pool);
    let room = sample_room(user_id);
    let id = room.id;

    repo.create_room(&room).await.unwrap();

    let found = repo.find_room_by_id(id).await.unwrap();
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id, id);
    assert_eq!(found.name.as_str(), "Test Room");
}

#[sqlx::test(migrations = "../../migrations")]
async fn finds_room_by_name(pool: PgPool) {
    let user_id = create_user(&pool).await;
    let repo = PostgresChatRoomRepository::new(pool);
    let room = sample_room(user_id);

    repo.create_room(&room).await.unwrap();

    let found = repo.find_room_by_name("Test Room").await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, room.id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn updates_room(pool: PgPool) {
    let user_id = create_user(&pool).await;
    let repo = PostgresChatRoomRepository::new(pool);
    let mut room = sample_room(user_id);
    repo.create_room(&room).await.unwrap();

    room.update(
        Some(ChatRoomName::new("Updated Room").unwrap()),
        Some("new description".into()),
    );
    repo.update_room(&room).await.unwrap();

    let found = repo.find_room_by_id(room.id).await.unwrap().unwrap();
    assert_eq!(found.name.as_str(), "Updated Room");
    assert_eq!(found.description, Some("new description".into()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn soft_deletes_room(pool: PgPool) {
    let user_id = create_user(&pool).await;
    let repo = PostgresChatRoomRepository::new(pool);
    let room = sample_room(user_id);
    repo.create_room(&room).await.unwrap();

    repo.soft_delete_room(room.id).await.unwrap();

    let found = repo.find_room_by_id(room.id).await.unwrap().unwrap();
    assert!(found.is_deleted);

    let listed = repo
        .list_rooms(&ChatRoomListQuery::default(), &[])
        .await
        .unwrap();
    assert!(!listed.iter().any(|r| r.id == room.id));
}

#[sqlx::test(migrations = "../../migrations")]
async fn lists_and_counts_rooms(pool: PgPool) {
    let user_id = create_user(&pool).await;
    let repo = PostgresChatRoomRepository::new(pool);
    let room = ChatRoom::new(
        Uuid::now_v7(),
        ChatRoomName::new("Private Room").unwrap(),
        None,
        ChatRoomType::Private,
        user_id,
    );
    repo.create_room(&room).await.unwrap();

    let query = ChatRoomListQuery {
        limit: 10,
        ..Default::default()
    };
    let listed = repo.list_rooms(&query, &[room.id]).await.unwrap();
    assert_eq!(listed.len(), 1);

    let count = repo.count_rooms(&query, &[room.id]).await.unwrap();
    assert_eq!(count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn manages_memberships(pool: PgPool) {
    let user_id = create_user(&pool).await;
    let repo = PostgresChatRoomRepository::new(pool);
    let room = sample_room(user_id);
    repo.create_room(&room).await.unwrap();

    let membership =
        ChatRoomMembership::for_user(Uuid::now_v7(), room.id, user_id, ChatRoomMemberRole::Member);
    repo.add_membership(&membership).await.unwrap();

    let found = repo.find_membership_by_id(membership.id).await.unwrap();
    assert!(found.is_some());

    let for_user = repo
        .find_membership_for_user(room.id, user_id)
        .await
        .unwrap();
    assert!(for_user.is_some());

    repo.update_membership_role(membership.id, ChatRoomMemberRole::Moderator)
        .await
        .unwrap();
    let updated = repo
        .find_membership_by_id(membership.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.member_role, ChatRoomMemberRole::Moderator);

    let members = repo.list_memberships_for_room(room.id).await.unwrap();
    assert_eq!(members.len(), 1);

    let room_ids = repo.list_room_ids_for_user(user_id, &[]).await.unwrap();
    assert_eq!(room_ids, vec![room.id]);

    assert!(repo.is_user_in_room(room.id, user_id, &[]).await.unwrap());

    repo.remove_membership(membership.id).await.unwrap();
    assert!(repo
        .find_membership_by_id(membership.id)
        .await
        .unwrap()
        .is_none());
}
