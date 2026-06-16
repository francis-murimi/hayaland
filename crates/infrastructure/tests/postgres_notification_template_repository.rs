use domain::entities::{NotificationChannel, NotificationTemplate, NotificationType};
use domain::repositories::{NotificationTemplateRepository, Pagination};
use infrastructure::repositories::PostgresNotificationTemplateRepository;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

fn sample_template(
    name: &str,
    notification_type: NotificationType,
    channel: NotificationChannel,
    locale: &str,
) -> NotificationTemplate {
    NotificationTemplate::new(
        Uuid::now_v7(),
        name.to_string(),
        notification_type,
        channel,
        locale.to_string(),
        "Subject".to_string(),
        "Body".to_string(),
        serde_json::json!({"foo": "string"}),
    )
    .unwrap()
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_and_find_by_id(pool: PgPool) {
    let repo = PostgresNotificationTemplateRepository::new(pool);

    let template = sample_template(
        "create_find_template",
        NotificationType::SystemMaintenance,
        NotificationChannel::Email,
        "en",
    );
    let id = template.id;

    repo.create(&template).await.unwrap();

    let found = repo.find_by_id(id).await.unwrap();
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id, id);
    assert_eq!(found.name, "create_find_template");
    assert_eq!(found.notification_type, NotificationType::SystemMaintenance);
    assert_eq!(found.channel, NotificationChannel::Email);
    assert_eq!(found.locale, "en");
    assert_eq!(found.subject_template, "Subject");
    assert_eq!(found.body_template, "Body");
    assert!(found.is_active);
}

#[sqlx::test(migrations = "../../migrations")]
async fn find_active(pool: PgPool) {
    let repo = PostgresNotificationTemplateRepository::new(pool);

    let mut active = sample_template(
        "find_active_active",
        NotificationType::DealSubmitted,
        NotificationChannel::InApp,
        "sw",
    );
    active.is_active = true;

    let mut inactive = sample_template(
        "find_active_inactive",
        NotificationType::DealSubmitted,
        NotificationChannel::Email,
        "sw",
    );
    inactive.is_active = false;

    repo.create(&active).await.unwrap();
    repo.create(&inactive).await.unwrap();

    let found = repo
        .find_active(
            NotificationType::DealSubmitted,
            NotificationChannel::InApp,
            "sw",
        )
        .await
        .unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, active.id);

    // Inactive template is not returned.
    let found = repo
        .find_active(
            NotificationType::DealSubmitted,
            NotificationChannel::Email,
            "sw",
        )
        .await
        .unwrap();
    assert!(found.is_none());

    // Different locale does not match.
    let found = repo
        .find_active(
            NotificationType::DealSubmitted,
            NotificationChannel::InApp,
            "en",
        )
        .await
        .unwrap();
    assert!(found.is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_with_pagination(pool: PgPool) {
    let repo = PostgresNotificationTemplateRepository::new(pool);

    for i in 0..3 {
        let locale = format!("sw_{i}");
        let template = sample_template(
            &format!("list_pagination_{i}"),
            NotificationType::EscrowFunded,
            NotificationChannel::Push,
            &locale,
        );
        repo.create(&template).await.unwrap();
    }

    let all = repo
        .list(Pagination {
            limit: 10,
            offset: 0,
        })
        .await
        .unwrap();
    // Seeded templates exist in addition to the three created above.
    assert!(all.len() >= 3);

    let page = repo
        .list(Pagination {
            limit: 2,
            offset: 0,
        })
        .await
        .unwrap();
    assert_eq!(page.len(), 2);

    let page = repo
        .list(Pagination {
            limit: 2,
            offset: 2,
        })
        .await
        .unwrap();
    assert_eq!(page.len(), 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn update_template(pool: PgPool) {
    let repo = PostgresNotificationTemplateRepository::new(pool);

    let mut template = sample_template(
        "update_template",
        NotificationType::PaymentDue,
        NotificationChannel::Email,
        "en",
    );
    repo.create(&template).await.unwrap();

    template.name = "update_template_renamed".to_string();
    template.notification_type = NotificationType::PaymentReceived;
    template.channel = NotificationChannel::Push;
    template.locale = "sw".to_string();
    template.subject_template = "New subject".to_string();
    template.body_template = "New body".to_string();
    template.variables_schema = serde_json::json!({"bar": "number"});
    template.is_active = false;
    template.updated_at = OffsetDateTime::now_utc();

    repo.update(&template).await.unwrap();

    let found = repo.find_by_id(template.id).await.unwrap().unwrap();
    assert_eq!(found.name, "update_template_renamed");
    assert_eq!(found.notification_type, NotificationType::PaymentReceived);
    assert_eq!(found.channel, NotificationChannel::Push);
    assert_eq!(found.locale, "sw");
    assert_eq!(found.subject_template, "New subject");
    assert_eq!(found.body_template, "New body");
    assert_eq!(found.variables_schema, serde_json::json!({"bar": "number"}));
    assert!(!found.is_active);
}

#[sqlx::test(migrations = "../../migrations")]
async fn delete_template(pool: PgPool) {
    let repo = PostgresNotificationTemplateRepository::new(pool);

    let template = sample_template(
        "delete_template",
        NotificationType::VerificationApproved,
        NotificationChannel::Email,
        "en",
    );
    let id = template.id;
    repo.create(&template).await.unwrap();

    repo.delete(id).await.unwrap();

    let found = repo.find_by_id(id).await.unwrap();
    assert!(found.is_none());
}
