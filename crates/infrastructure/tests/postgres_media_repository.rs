use domain::entities::{
    Email, MediaPurpose, MediaRelatedEntityType, MediaUpload, PasswordHash, User, Username,
};
use domain::repositories::{MediaFilters, MediaRepository, UserRepository};
use infrastructure::repositories::{PostgresMediaRepository, PostgresUserRepository};
use sqlx::PgPool;
use uuid::Uuid;

async fn create_user(pool: &PgPool, email: &str, username: &str) -> Uuid {
    let repo = PostgresUserRepository::new(pool.clone());
    let user = User::new(
        Uuid::now_v7(),
        Email::new(email).unwrap(),
        Username::new(username).unwrap(),
        PasswordHash::new("hash".to_string()).unwrap(),
    );
    repo.create(&user).await.unwrap();
    user.id
}

fn sample_upload(
    owner_user_id: Uuid,
    original_filename: &str,
    related_entity_type: Option<MediaRelatedEntityType>,
    related_entity_id: Option<Uuid>,
) -> MediaUpload {
    MediaUpload::new(
        Uuid::now_v7(),
        owner_user_id,
        None,
        MediaPurpose::Other,
        related_entity_type,
        related_entity_id,
        original_filename.to_string(),
        format!("stored-{original_filename}"),
        format!("uploads/stored-{original_filename}"),
        "text/plain".to_string(),
        100,
        "deadbeef".to_string(),
        false,
    )
    .unwrap()
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_and_find_media_upload(pool: PgPool) {
    let owner = create_user(&pool, "media_owner@example.com", "media_owner").await;
    let repo = PostgresMediaRepository::new(pool);
    let upload = sample_upload(owner, "file.txt", None, None);

    repo.create(&upload).await.unwrap();

    let found = repo.find_by_id(upload.id, false).await.unwrap();
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id, upload.id);
    assert_eq!(found.original_filename, "file.txt");
    assert_eq!(found.content_type, "text/plain");

    let found_deleted_ok = repo.find_by_id(upload.id, true).await.unwrap();
    assert!(found_deleted_ok.is_some());

    let missing = repo.find_by_id(Uuid::now_v7(), false).await.unwrap();
    assert!(missing.is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_media_with_filters(pool: PgPool) {
    let owner = create_user(&pool, "media_filter@example.com", "media_filter").await;
    let other_owner = create_user(&pool, "media_other@example.com", "media_other").await;
    let repo = PostgresMediaRepository::new(pool);

    let party_id = Uuid::now_v7();
    let message_id = Uuid::now_v7();

    let upload_a = sample_upload(
        owner,
        "a.txt",
        Some(MediaRelatedEntityType::Party),
        Some(party_id),
    );
    repo.create(&upload_a).await.unwrap();

    let upload_b = sample_upload(
        owner,
        "b.txt",
        Some(MediaRelatedEntityType::Message),
        Some(message_id),
    );
    repo.create(&upload_b).await.unwrap();

    let upload_other = sample_upload(
        other_owner,
        "other.txt",
        None,
        None,
    );
    repo.create(&upload_other).await.unwrap();

    let by_owner = repo
        .list(&MediaFilters {
            owner_user_id: Some(owner),
            limit: 10,
            offset: 0,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(by_owner.total, 2);

    let by_related = repo
        .list(&MediaFilters {
            related_entity_type: Some("PARTY".to_string()),
            related_entity_id: Some(party_id),
            limit: 10,
            offset: 0,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(by_related.total, 1);
    assert_eq!(by_related.items[0].id, upload_a.id);

    let paginated = repo
        .list(&MediaFilters {
            owner_user_id: Some(owner),
            limit: 1,
            offset: 1,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(paginated.total, 2);
    assert_eq!(paginated.items.len(), 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn soft_delete_and_permanent_delete(pool: PgPool) {
    let owner = create_user(&pool, "media_delete@example.com", "media_delete").await;
    let repo = PostgresMediaRepository::new(pool);
    let upload = sample_upload(owner, "delete.txt", None, None);

    repo.create(&upload).await.unwrap();

    let soft_deleted = repo.soft_delete(upload.id).await.unwrap();
    assert!(soft_deleted);

    let not_found = repo.find_by_id(upload.id, false).await.unwrap();
    assert!(not_found.is_none());

    let found_deleted = repo.find_by_id(upload.id, true).await.unwrap();
    assert!(found_deleted.is_some());
    assert!(found_deleted.unwrap().deleted_at.is_some());

    let without_deleted = repo
        .list(&MediaFilters {
            owner_user_id: Some(owner),
            include_deleted: false,
            limit: 10,
            offset: 0,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(without_deleted.total, 0);

    let with_deleted = repo
        .list(&MediaFilters {
            owner_user_id: Some(owner),
            include_deleted: true,
            limit: 10,
            offset: 0,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(with_deleted.total, 1);

    let permanently_deleted = repo.delete_permanently(upload.id).await.unwrap();
    assert!(permanently_deleted);

    let gone = repo.find_by_id(upload.id, true).await.unwrap();
    assert!(gone.is_none());

    let soft_delete_missing = repo.soft_delete(Uuid::now_v7()).await.unwrap();
    assert!(!soft_delete_missing);
    let permanent_delete_missing = repo.delete_permanently(Uuid::now_v7()).await.unwrap();
    assert!(!permanent_delete_missing);
}
