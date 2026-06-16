use domain::entities::{
    ActionType, DisplayName, Email, Notification, NotificationAction, NotificationChannel,
    NotificationPriority, NotificationStatus, NotificationType, Party, PartyType, PasswordHash,
    User, Username,
};
use domain::repositories::{
    DeliveryResult, NotificationFilters, NotificationRepository, Pagination, PartyRepository,
    UserRepository,
};
use infrastructure::repositories::{
    PostgresNotificationRepository, PostgresPartyRepository, PostgresUserRepository,
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

fn sample_notification(
    user_id: Option<Uuid>,
    party_id: Option<Uuid>,
    notification_type: NotificationType,
    priority: NotificationPriority,
) -> Notification {
    let mut notification = Notification::new(
        Uuid::now_v7(),
        user_id,
        party_id,
        notification_type,
        "Test title".to_string(),
        "Test body".to_string(),
        priority,
        Some("/action".to_string()),
        vec![NotificationAction {
            label: "Open".to_string(),
            action_type: ActionType::Navigate,
            url: Some("/open".to_string()),
            method: None,
        }],
        Some("deal".to_string()),
        Some(Uuid::now_v7()),
        serde_json::json!({"key": "value"}),
        None,
    )
    .unwrap();
    notification.channels = vec![NotificationChannel::InApp, NotificationChannel::Email];
    notification
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_and_find_by_id(pool: PgPool) {
    let user_repo = PostgresUserRepository::new(pool.clone());
    let repo = PostgresNotificationRepository::new(pool);
    let user = sample_user("notif-create@example.com", "notif_create");
    let user_id = user.id;

    user_repo.create(&user).await.unwrap();

    let notification = sample_notification(
        Some(user_id),
        None,
        NotificationType::DealInvite,
        NotificationPriority::High,
    );
    let id = notification.id;

    repo.create(&notification).await.unwrap();

    let found = repo.find_by_id(id).await.unwrap();
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id, id);
    assert_eq!(found.user_id, Some(user_id));
    assert_eq!(found.party_id, None);
    assert_eq!(found.notification_type, NotificationType::DealInvite);
    assert_eq!(found.title, "Test title");
    assert_eq!(found.body, "Test body");
    assert_eq!(found.priority, NotificationPriority::High);
    assert_eq!(found.status, NotificationStatus::Pending);
    assert_eq!(
        found.channels,
        vec![NotificationChannel::InApp, NotificationChannel::Email]
    );
    assert_eq!(found.action_url, Some("/action".to_string()));
    assert_eq!(found.actions.len(), 1);
    assert_eq!(found.related_entity_type, Some("deal".to_string()));
    assert!(found.related_entity_id.is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_for_recipient_with_filters_and_pagination(pool: PgPool) {
    let user_repo = PostgresUserRepository::new(pool.clone());
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let repo = PostgresNotificationRepository::new(pool);

    let user = sample_user("notif-list@example.com", "notif_list");
    let user_id = user.id;
    user_repo.create(&user).await.unwrap();

    let party = sample_party("notif-party@example.com");
    let party_id = party.id;
    party_repo.create(&party).await.unwrap();

    // Create notifications for the user.
    let mut n1 = sample_notification(
        Some(user_id),
        None,
        NotificationType::DealInvite,
        NotificationPriority::High,
    );
    n1.title = "Invite".to_string();
    repo.create(&n1).await.unwrap();

    let mut n2 = sample_notification(
        Some(user_id),
        None,
        NotificationType::MessageReceived,
        NotificationPriority::Normal,
    );
    n2.title = "Message".to_string();
    repo.create(&n2).await.unwrap();

    // Create notification for the party.
    let mut n3 = sample_notification(
        None,
        Some(party_id),
        NotificationType::DealCompleted,
        NotificationPriority::Low,
    );
    n3.title = "Completed".to_string();
    repo.create(&n3).await.unwrap();

    // List all for user.
    let result = repo
        .list_for_recipient(
            Some(user_id),
            None,
            NotificationFilters::default(),
            Pagination {
                limit: 10,
                offset: 0,
            },
        )
        .await
        .unwrap();
    assert_eq!(result.items.len(), 2);
    assert_eq!(result.total, 2);

    // Filter by type.
    let result = repo
        .list_for_recipient(
            Some(user_id),
            None,
            NotificationFilters {
                notification_type: Some(NotificationType::DealInvite),
                ..Default::default()
            },
            Pagination {
                limit: 10,
                offset: 0,
            },
        )
        .await
        .unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].title, "Invite");

    // Filter by priority.
    let result = repo
        .list_for_recipient(
            Some(user_id),
            None,
            NotificationFilters {
                priority: Some(NotificationPriority::Normal),
                ..Default::default()
            },
            Pagination {
                limit: 10,
                offset: 0,
            },
        )
        .await
        .unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].title, "Message");

    // Pagination.
    let result = repo
        .list_for_recipient(
            Some(user_id),
            None,
            NotificationFilters::default(),
            Pagination {
                limit: 1,
                offset: 0,
            },
        )
        .await
        .unwrap();
    assert_eq!(result.items.len(), 1);

    // Party recipient.
    let result = repo
        .list_for_recipient(
            None,
            Some(party_id),
            NotificationFilters::default(),
            Pagination {
                limit: 10,
                offset: 0,
            },
        )
        .await
        .unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].title, "Completed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn count_unread_for_recipient(pool: PgPool) {
    let user_repo = PostgresUserRepository::new(pool.clone());
    let repo = PostgresNotificationRepository::new(pool);
    let user = sample_user("notif-unread@example.com", "notif_unread");
    let user_id = user.id;

    user_repo.create(&user).await.unwrap();

    let mut n1 = sample_notification(
        Some(user_id),
        None,
        NotificationType::DealInvite,
        NotificationPriority::High,
    );
    let n2 = sample_notification(
        Some(user_id),
        None,
        NotificationType::MessageReceived,
        NotificationPriority::Normal,
    );
    repo.create(&n1).await.unwrap();
    repo.create(&n2).await.unwrap();

    let count = repo
        .count_unread_for_recipient(Some(user_id), None)
        .await
        .unwrap();
    assert_eq!(count, 2);

    let now = OffsetDateTime::now_utc();
    n1.mark_read(now);
    repo.mark_read(n1.id, user_id, None, now).await.unwrap();

    let count = repo
        .count_unread_for_recipient(Some(user_id), None)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn mark_read_and_mark_all_read(pool: PgPool) {
    let user_repo = PostgresUserRepository::new(pool.clone());
    let repo = PostgresNotificationRepository::new(pool);
    let user = sample_user("notif-read@example.com", "notif_read");
    let user_id = user.id;

    user_repo.create(&user).await.unwrap();

    let n1 = sample_notification(
        Some(user_id),
        None,
        NotificationType::DealInvite,
        NotificationPriority::High,
    );
    let n2 = sample_notification(
        Some(user_id),
        None,
        NotificationType::MessageReceived,
        NotificationPriority::Normal,
    );
    repo.create(&n1).await.unwrap();
    repo.create(&n2).await.unwrap();

    let now = OffsetDateTime::now_utc();
    let marked = repo.mark_read(n1.id, user_id, None, now).await.unwrap();
    assert!(marked);

    let found = repo.find_by_id(n1.id).await.unwrap().unwrap();
    assert_eq!(found.status, NotificationStatus::Delivered);
    assert!(found.read_at.is_some());

    // Marking the same notification again returns false.
    let marked = repo.mark_read(n1.id, user_id, None, now).await.unwrap();
    assert!(!marked);

    // Mark all remaining unread messages read.
    let count = repo
        .mark_all_read(Some(user_id), None, None, None)
        .await
        .unwrap();
    assert_eq!(count, 1);

    let found = repo.find_by_id(n2.id).await.unwrap().unwrap();
    assert!(found.read_at.is_some());

    // Filter by notification type.
    let n3 = sample_notification(
        Some(user_id),
        None,
        NotificationType::DealCompleted,
        NotificationPriority::Low,
    );
    repo.create(&n3).await.unwrap();
    let count = repo
        .mark_all_read(
            Some(user_id),
            None,
            None,
            Some(NotificationType::DealCompleted),
        )
        .await
        .unwrap();
    assert_eq!(count, 1);

    let found = repo.find_by_id(n3.id).await.unwrap().unwrap();
    assert!(found.read_at.is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn mark_actioned(pool: PgPool) {
    let user_repo = PostgresUserRepository::new(pool.clone());
    let repo = PostgresNotificationRepository::new(pool);
    let user = sample_user("notif-action@example.com", "notif_action");
    let user_id = user.id;

    user_repo.create(&user).await.unwrap();

    let notification = sample_notification(
        Some(user_id),
        None,
        NotificationType::DealInvite,
        NotificationPriority::High,
    );
    repo.create(&notification).await.unwrap();

    let now = OffsetDateTime::now_utc();
    let actioned = repo
        .mark_actioned(notification.id, user_id, None, now)
        .await
        .unwrap();
    assert!(actioned);

    let found = repo.find_by_id(notification.id).await.unwrap().unwrap();
    assert!(found.actioned_at.is_some());

    // Actioning again returns false.
    let actioned = repo
        .mark_actioned(notification.id, user_id, None, now)
        .await
        .unwrap();
    assert!(!actioned);
}

#[sqlx::test(migrations = "../../migrations")]
async fn delete_notification(pool: PgPool) {
    let user_repo = PostgresUserRepository::new(pool.clone());
    let repo = PostgresNotificationRepository::new(pool);
    let user = sample_user("notif-delete@example.com", "notif_delete");
    let user_id = user.id;

    user_repo.create(&user).await.unwrap();

    let notification = sample_notification(
        Some(user_id),
        None,
        NotificationType::DealInvite,
        NotificationPriority::High,
    );
    let id = notification.id;
    repo.create(&notification).await.unwrap();

    let deleted = repo.delete(id, user_id, None).await.unwrap();
    assert!(deleted);

    let found = repo.find_by_id(id).await.unwrap();
    assert!(found.is_none());

    // Deleting again returns false.
    let deleted = repo.delete(id, user_id, None).await.unwrap();
    assert!(!deleted);
}

#[sqlx::test(migrations = "../../migrations")]
async fn update_status(pool: PgPool) {
    let user_repo = PostgresUserRepository::new(pool.clone());
    let repo = PostgresNotificationRepository::new(pool);
    let user = sample_user("notif-status@example.com", "notif_status");
    let user_id = user.id;

    user_repo.create(&user).await.unwrap();

    let notification = sample_notification(
        Some(user_id),
        None,
        NotificationType::DealInvite,
        NotificationPriority::High,
    );
    let id = notification.id;
    repo.create(&notification).await.unwrap();

    repo.update_status(id, NotificationStatus::Sent)
        .await
        .unwrap();

    let found = repo.find_by_id(id).await.unwrap().unwrap();
    assert_eq!(found.status, NotificationStatus::Sent);

    repo.update_status(id, NotificationStatus::Failed)
        .await
        .unwrap();

    let found = repo.find_by_id(id).await.unwrap().unwrap();
    assert_eq!(found.status, NotificationStatus::Failed);
}

#[sqlx::test(migrations = "../../migrations")]
async fn record_delivery(pool: PgPool) {
    let user_repo = PostgresUserRepository::new(pool.clone());
    let repo = PostgresNotificationRepository::new(pool.clone());
    let user = sample_user("notif-delivery@example.com", "notif_delivery");
    let user_id = user.id;

    user_repo.create(&user).await.unwrap();

    let notification = sample_notification(
        Some(user_id),
        None,
        NotificationType::DealInvite,
        NotificationPriority::High,
    );
    let id = notification.id;
    repo.create(&notification).await.unwrap();

    repo.record_delivery(id, NotificationChannel::Email, DeliveryResult::Sent)
        .await
        .unwrap();
    repo.record_delivery(
        id,
        NotificationChannel::Email,
        DeliveryResult::Failed {
            message: "bounce".to_string(),
        },
    )
    .await
    .unwrap();
    repo.record_delivery(id, NotificationChannel::Push, DeliveryResult::Delivered)
        .await
        .unwrap();

    let rows: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT channel, status FROM notification_delivery_records WHERE notification_id = $1 ORDER BY attempted_at",
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0], ("EMAIL".to_string(), Some("SENT".to_string())));
    assert_eq!(rows[1], ("EMAIL".to_string(), Some("FAILED".to_string())));
    assert_eq!(rows[2], ("PUSH".to_string(), Some("DELIVERED".to_string())));
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_pending(pool: PgPool) {
    let user_repo = PostgresUserRepository::new(pool.clone());
    let repo = PostgresNotificationRepository::new(pool);
    let user = sample_user("notif-pending@example.com", "notif_pending");
    let user_id = user.id;

    user_repo.create(&user).await.unwrap();

    let mut n1 = sample_notification(
        Some(user_id),
        None,
        NotificationType::DealInvite,
        NotificationPriority::High,
    );
    n1.status = NotificationStatus::Pending;
    let mut n2 = sample_notification(
        Some(user_id),
        None,
        NotificationType::MessageReceived,
        NotificationPriority::Normal,
    );
    n2.status = NotificationStatus::Pending;
    let mut n3 = sample_notification(
        Some(user_id),
        None,
        NotificationType::DealCompleted,
        NotificationPriority::Low,
    );
    n3.status = NotificationStatus::Sent;

    repo.create(&n1).await.unwrap();
    repo.create(&n2).await.unwrap();
    repo.create(&n3).await.unwrap();

    let pending = repo.list_pending(10, None).await.unwrap();
    assert_eq!(pending.len(), 2);

    // Only pending notifications older than the threshold are returned.
    let past = OffsetDateTime::now_utc() - time::Duration::hours(1);
    let pending = repo.list_pending(10, Some(past)).await.unwrap();
    assert_eq!(pending.len(), 0);

    // Batch size limit.
    let pending = repo.list_pending(1, None).await.unwrap();
    assert_eq!(pending.len(), 1);
}
