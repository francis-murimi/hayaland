use application::deals::{DealTimeoutConfig, ProcessDealTimeouts};
use infrastructure::repositories::{PostgresDealRepository, PostgresMilestoneRepository};
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

async fn setup_pool() -> sqlx::PgPool {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://hayaland:hayaland@127.0.0.1:5432/hayaland_test".to_string()
    });
    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
    sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn process_timeouts_expires_draft_deal_in_database() {
    let pool = setup_pool().await;
    let category_id = Uuid::now_v7();
    let party_id = Uuid::now_v7();
    let deal_id = Uuid::now_v7();
    let category_code = format!("TIMEOUT-CAT-{}", Uuid::now_v7());

    sqlx::query(
        "INSERT INTO categories (id, category_name, category_code, category_type, display_order)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(category_id)
    .bind("Timeout Test Category")
    .bind(&category_code)
    .bind("DOMAIN")
    .bind(1i32)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO parties (id, party_type, display_name, email)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(party_id)
    .bind("ORGANIZATION")
    .bind("Timeout Party")
    .bind(format!("timeout-party-{}@example.com", Uuid::now_v7()))
    .execute(&pool)
    .await
    .unwrap();

    let entered_at = OffsetDateTime::now_utc() - time::Duration::seconds(120);
    sqlx::query(
        "INSERT INTO deals (
            id, deal_reference, deal_title, domain_category_id, initiator_party_id,
            initiator_role, deal_status, current_state_entered_at, created_at, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        ON CONFLICT (id) DO UPDATE SET
            deal_status = EXCLUDED.deal_status,
            current_state_entered_at = EXCLUDED.current_state_entered_at",
    )
    .bind(deal_id)
    .bind(format!("DL-TIMEOUT-{}", Uuid::now_v7()))
    .bind("Timeout Test Deal")
    .bind(category_id)
    .bind(party_id)
    .bind("SUPPLIER")
    .bind("DRAFT")
    .bind(entered_at)
    .bind(entered_at)
    .bind(entered_at)
    .execute(&pool)
    .await
    .unwrap();

    let deal_repo: Arc<dyn domain::repositories::DealRepository> =
        Arc::new(PostgresDealRepository::new(pool.clone()));
    let milestone_repo: Arc<dyn domain::repositories::MilestoneRepository> =
        Arc::new(PostgresMilestoneRepository::new(pool.clone()));
    let config = DealTimeoutConfig::new(60, 120, 120, 200, 120, 120, 80, 200, 120);

    let use_case = ProcessDealTimeouts::new(deal_repo.clone(), milestone_repo, config);
    let result = use_case.execute(10).await.unwrap();

    assert_eq!(result.transitioned, vec![deal_id]);

    let row = sqlx::query_scalar::<_, String>("SELECT deal_status FROM deals WHERE id = $1")
        .bind(deal_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row, "EXPIRED");

    let history_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM deal_history WHERE deal_id = $1 AND event_type = $2",
    )
    .bind(deal_id)
    .bind("DEAL_TIMEOUT_TRANSITION")
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(history_count, 1);
}
