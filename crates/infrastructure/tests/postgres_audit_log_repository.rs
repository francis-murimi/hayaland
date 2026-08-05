use domain::entities::{
    AdminAction, AdminActionTargetType, AdminActionType, Email, PasswordHash, User, Username,
};
use domain::repositories::{AuditLogFilters, AuditLogRepository, UserRepository};
use infrastructure::repositories::{PostgresAuditLogRepository, PostgresUserRepository};
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

fn sample_action(
    admin_user_id: Uuid,
    action_type: AdminActionType,
    target_type: AdminActionTargetType,
    target_id: Uuid,
) -> AdminAction {
    AdminAction::new(
        Uuid::now_v7(),
        admin_user_id,
        action_type,
        target_type,
        target_id,
        Some(serde_json::json!({ "before": true })),
        Some(serde_json::json!({ "after": true })),
        Some("audit reason".to_string()),
        Some("127.0.0.1".to_string()),
    )
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_and_list_audit_log(pool: PgPool) {
    let admin = create_user(&pool, "audit_admin@example.com", "audit_admin").await;
    let target_id = Uuid::now_v7();
    let action = sample_action(
        admin,
        AdminActionType::PartyUpdated,
        AdminActionTargetType::Party,
        target_id,
    );

    let repo = PostgresAuditLogRepository::new(pool);
    repo.create(&action).await.unwrap();

    let result = repo.list(&AuditLogFilters::default()).await.unwrap();
    assert_eq!(result.total, 1);
    assert_eq!(result.items[0].id, action.id);
    assert_eq!(result.items[0].action_type, AdminActionType::PartyUpdated);
    assert_eq!(result.items[0].target_type, AdminActionTargetType::Party);
    assert_eq!(result.items[0].target_id, target_id);

    let for_target = repo
        .list_for_target(AdminActionTargetType::Party, target_id, 10, 0)
        .await
        .unwrap();
    assert_eq!(for_target.total, 1);
    assert_eq!(for_target.items[0].id, action.id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_audit_log_with_filters(pool: PgPool) {
    let admin_a = create_user(&pool, "audit_a@example.com", "audit_a").await;
    let admin_b = create_user(&pool, "audit_b@example.com", "audit_b").await;
    let party_target = Uuid::now_v7();
    let deal_target = Uuid::now_v7();

    let repo = PostgresAuditLogRepository::new(pool.clone());

    let action_a = sample_action(
        admin_a,
        AdminActionType::PartyUpdated,
        AdminActionTargetType::Party,
        party_target,
    );
    repo.create(&action_a).await.unwrap();

    // Sleep briefly so the second action has a later created_at, avoiding ordering flakiness
    // when filtering by time windows.
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    let action_b = sample_action(
        admin_b,
        AdminActionType::DealStateChanged,
        AdminActionTargetType::Deal,
        deal_target,
    );
    repo.create(&action_b).await.unwrap();

    let by_admin = repo
        .list(&AuditLogFilters {
            admin_user_id: Some(admin_a),
            limit: 10,
            offset: 0,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(by_admin.total, 1);
    assert_eq!(by_admin.items[0].id, action_a.id);

    let by_action_type = repo
        .list(&AuditLogFilters {
            action_type: Some(AdminActionType::DealStateChanged),
            limit: 10,
            offset: 0,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(by_action_type.total, 1);
    assert_eq!(by_action_type.items[0].id, action_b.id);

    let by_target_type = repo
        .list(&AuditLogFilters {
            target_type: Some(AdminActionTargetType::Party),
            limit: 10,
            offset: 0,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(by_target_type.total, 1);
    assert_eq!(by_target_type.items[0].id, action_a.id);

    let by_target_id = repo
        .list(&AuditLogFilters {
            target_id: Some(deal_target),
            limit: 10,
            offset: 0,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(by_target_id.total, 1);
    assert_eq!(by_target_id.items[0].id, action_b.id);

    let from = action_a.created_at - time::Duration::seconds(1);
    let to = action_b.created_at + time::Duration::seconds(1);
    let by_time = repo
        .list(&AuditLogFilters {
            from: Some(from),
            to: Some(to),
            limit: 10,
            offset: 0,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(by_time.total, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_for_target_filters_by_target(pool: PgPool) {
    let admin = create_user(&pool, "audit_target@example.com", "audit_target").await;
    let party_target = Uuid::now_v7();
    let deal_target = Uuid::now_v7();

    let repo = PostgresAuditLogRepository::new(pool.clone());

    let party_action = sample_action(
        admin,
        AdminActionType::PartyUpdated,
        AdminActionTargetType::Party,
        party_target,
    );
    repo.create(&party_action).await.unwrap();

    let deal_action = sample_action(
        admin,
        AdminActionType::DealStateChanged,
        AdminActionTargetType::Deal,
        deal_target,
    );
    repo.create(&deal_action).await.unwrap();

    let party_result = repo
        .list_for_target(AdminActionTargetType::Party, party_target, 10, 0)
        .await
        .unwrap();
    assert_eq!(party_result.total, 1);
    assert_eq!(party_result.items[0].id, party_action.id);

    let deal_result = repo
        .list_for_target(AdminActionTargetType::Deal, deal_target, 10, 0)
        .await
        .unwrap();
    assert_eq!(deal_result.total, 1);
    assert_eq!(deal_result.items[0].id, deal_action.id);

    let wrong_target = Uuid::now_v7();
    let empty = repo
        .list_for_target(AdminActionTargetType::Party, wrong_target, 10, 0)
        .await
        .unwrap();
    assert_eq!(empty.total, 0);
    assert!(empty.items.is_empty());
}
