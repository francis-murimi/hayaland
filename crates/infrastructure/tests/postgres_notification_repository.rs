use domain::entities::{
    ActionType, Notification, NotificationAction, NotificationChannel, NotificationPriority,
    NotificationStatus, NotificationType,
};
use domain::errors::DomainError;
use domain::repositories::{
    DeliveryResult, NotificationFilters, NotificationRepository, Pagination,
};
use infrastructure::repositories::PostgresNotificationRepository;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

async fn create_user(pool: &PgPool) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        r#"
        INSERT INTO users (id, email, username, password_hash, is_active, created_at, updated_at)
        VALUES ($1, $2, $3, $4, true, now(), now())
        "#,
        id,
        format!("notif-user-{id}@example.com"),
        format!("notif_user_{id}"),
        "hash"
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

fn sample_notification(user_id: Option<Uuid>, party_id: Option<Uuid>) -> Notification {
    Notification::new(
        Uuid::now_v7(),
        user_id,
        party_id,
        NotificationType::DealSubmitted,
        "Test title".to_string(),
        "Test body".to_string(),
        NotificationPriority::Normal,
        None,
        vec![NotificationAction {
            label: "Open".to_string(),
            action_type: ActionType::Navigate,
            url: Some("/deal/1".to_string()),
            method: None,
        }],
        None,
        None,
        serde_json::Value::Null,
        None,
    )
    .unwrap()
}

#[sqlx::test(migrations = "../../migrations")]
async fn creates_and_finds_notification(pool: PgPool) {
    let repo = PostgresNotificationRepository::new(pool.clone());
    let user_id = create_user(&pool).await;
    let mut notification = sample_notification(Some(user_id), None);
    notification.channels = vec![NotificationChannel::InApp];

    repo.create(&notification).await.unwrap();

    let found = repo.find_by_id(notification.id).await.unwrap();
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id, notification.id);
    assert_eq!(found.title, "Test title");
    assert_eq!(found.channels.len(), 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn duplicate_create_returns_error(pool: PgPool) {
    let repo = PostgresNotificationRepository::new(pool.clone());
    let user_id = create_user(&pool).await;
    let mut notification = sample_notification(Some(user_id), None);
    notification.channels = vec![NotificationChannel::InApp];

    repo.create(&notification).await.unwrap();
    let result = repo.create(&notification).await;
    assert!(matches!(result, Err(DomainError::RepositoryError(_))));
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_for_recipient_filters_and_paginates(pool: PgPool) {
    let repo = PostgresNotificationRepository::new(pool.clone());
    let user_id = create_user(&pool).await;

    for i in 0..3 {
        let mut notification = sample_notification(Some(user_id), None);
        notification.title = format!("Notification {i}");
        notification.channels = vec![NotificationChannel::InApp];
        repo.create(&notification).await.unwrap();
    }

    let result = repo
        .list_for_recipient(
            Some(user_id),
            None,
            NotificationFilters::default(),
            Pagination {
                limit: 2,
                offset: 0,
            },
        )
        .await
        .unwrap();
    assert_eq!(result.items.len(), 2);
    assert_eq!(result.total, 3);
    assert_eq!(result.unread_count, 3);

    let filters = NotificationFilters {
        notification_type: Some(NotificationType::DealSubmitted),
        is_read: None,
        is_actioned: None,
        priority: None,
    };
    let result = repo
        .list_for_recipient(
            Some(user_id),
            None,
            filters,
            Pagination {
                limit: 50,
                offset: 0,
            },
        )
        .await
        .unwrap();
    assert_eq!(result.items.len(), 3);

    let filters = NotificationFilters {
        notification_type: Some(NotificationType::DealInvite),
        ..Default::default()
    };
    let result = repo
        .list_for_recipient(
            Some(user_id),
            None,
            filters,
            Pagination {
                limit: 50,
                offset: 0,
            },
        )
        .await
        .unwrap();
    assert_eq!(result.items.len(), 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn mark_read_and_actioned_and_delete(pool: PgPool) {
    let repo = PostgresNotificationRepository::new(pool.clone());
    let user_id = create_user(&pool).await;
    let mut notification = sample_notification(Some(user_id), None);
    notification.channels = vec![NotificationChannel::InApp];
    repo.create(&notification).await.unwrap();

    let marked = repo
        .mark_read(notification.id, user_id, None, OffsetDateTime::now_utc())
        .await
        .unwrap();
    assert!(marked);

    let filters = NotificationFilters {
        is_read: Some(true),
        ..Default::default()
    };
    let result = repo
        .list_for_recipient(
            Some(user_id),
            None,
            filters,
            Pagination {
                limit: 50,
                offset: 0,
            },
        )
        .await
        .unwrap();
    assert_eq!(result.items.len(), 1);

    let actioned = repo
        .mark_actioned(notification.id, user_id, None, OffsetDateTime::now_utc())
        .await
        .unwrap();
    assert!(actioned);

    let filters = NotificationFilters {
        is_actioned: Some(true),
        ..Default::default()
    };
    let result = repo
        .list_for_recipient(
            Some(user_id),
            None,
            filters,
            Pagination {
                limit: 50,
                offset: 0,
            },
        )
        .await
        .unwrap();
    assert_eq!(result.items.len(), 1);

    let deleted = repo.delete(notification.id, user_id, None).await.unwrap();
    assert!(deleted);
    assert!(repo.find_by_id(notification.id).await.unwrap().is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn mark_all_read_respects_type_filter(pool: PgPool) {
    let repo = PostgresNotificationRepository::new(pool.clone());
    let user_id = create_user(&pool).await;

    let mut deal = sample_notification(Some(user_id), None);
    deal.notification_type = NotificationType::DealSubmitted;
    deal.channels = vec![NotificationChannel::InApp];
    repo.create(&deal).await.unwrap();

    let mut invite = sample_notification(Some(user_id), None);
    invite.notification_type = NotificationType::DealInvite;
    invite.channels = vec![NotificationChannel::InApp];
    repo.create(&invite).await.unwrap();

    let marked = repo
        .mark_all_read(
            Some(user_id),
            None,
            None,
            Some(NotificationType::DealInvite),
        )
        .await
        .unwrap();
    assert_eq!(marked, 1);

    let result = repo
        .list_for_recipient(
            Some(user_id),
            None,
            NotificationFilters {
                is_read: Some(true),
                ..Default::default()
            },
            Pagination {
                limit: 50,
                offset: 0,
            },
        )
        .await
        .unwrap();
    assert_eq!(result.items.len(), 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn update_status_and_record_delivery(pool: PgPool) {
    let repo = PostgresNotificationRepository::new(pool.clone());
    let user_id = create_user(&pool).await;
    let mut notification = sample_notification(Some(user_id), None);
    notification.channels = vec![NotificationChannel::Email];
    repo.create(&notification).await.unwrap();

    repo.update_status(notification.id, NotificationStatus::Sent)
        .await
        .unwrap();

    let found = repo.find_by_id(notification.id).await.unwrap().unwrap();
    assert_eq!(found.status, NotificationStatus::Sent);

    repo.record_delivery(
        notification.id,
        NotificationChannel::Email,
        DeliveryResult::Sent,
    )
    .await
    .unwrap();
    repo.record_delivery(
        notification.id,
        NotificationChannel::Email,
        DeliveryResult::Delivered,
    )
    .await
    .unwrap();
    repo.record_delivery(
        notification.id,
        NotificationChannel::Email,
        DeliveryResult::Failed {
            message: "bounce".to_string(),
        },
    )
    .await
    .unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_pending_returns_pending_notifications(pool: PgPool) {
    let repo = PostgresNotificationRepository::new(pool.clone());
    let user_id = create_user(&pool).await;
    let mut notification = sample_notification(Some(user_id), None);
    notification.status = NotificationStatus::Pending;
    notification.channels = vec![NotificationChannel::InApp];
    repo.create(&notification).await.unwrap();

    let pending = repo.list_pending(10, None).await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, notification.id);
}
