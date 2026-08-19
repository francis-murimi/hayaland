use domain::entities::{DisplayName, Email, Party, PartyType, PasswordHash, User, Username};
use domain::errors::DomainError;
use domain::repositories::{AnalyticsRepository, MetricFilters, PartyRepository, UserRepository};
use infrastructure::repositories::{
    PostgresAnalyticsRepository, PostgresPartyRepository, PostgresUserRepository,
};
use sqlx::PgPool;
use time::{Date, Month, OffsetDateTime};
use uuid::Uuid;

fn agriculture_domain_id() -> Uuid {
    Uuid::parse_str("a1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap()
}

fn date_at(year: i32, month: Month, day: u8) -> Date {
    Date::from_calendar_date(year, month, day).unwrap()
}

fn start_of_day(date: Date) -> OffsetDateTime {
    date.with_hms(0, 0, 0).unwrap().assume_utc()
}

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

async fn create_party(pool: &PgPool, email: &str, name: &str) -> Uuid {
    let repo = PostgresPartyRepository::new(pool.clone());
    let party = Party::new(
        Uuid::now_v7(),
        PartyType::Organization,
        DisplayName::new(name).unwrap(),
        Email::new(email).unwrap(),
    );
    repo.create(&party).await.unwrap();
    party.id
}

async fn insert_deal(
    pool: &PgPool,
    party_id: Uuid,
    reference: &str,
    title: &str,
    status: &str,
    created_at: OffsetDateTime,
) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        r#"
        INSERT INTO deals (
            id, deal_reference, deal_title, deal_description, domain_category_id,
            initiator_party_id, initiator_role, deal_status, is_public, total_deal_value,
            created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, 'SUPPLIER', $7, false, 1000, $8, $9)
        "#,
        id,
        reference,
        title,
        Some("Analytics test deal".to_string()),
        agriculture_domain_id(),
        party_id,
        status,
        created_at,
        created_at
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

#[sqlx::test(migrations = "../../migrations")]
async fn refresh_and_dashboard_summary(pool: PgPool) {
    let _user = create_user(&pool, "analytics_user@example.com", "analytics_user").await;
    let party = create_party(&pool, "analytics_party@example.com", "Analytics Farm").await;

    // Use a far-future date so this snapshot is the latest when get_dashboard_summary runs.
    let date = date_at(3000, Month::January, 1);
    let created_at = start_of_day(date);

    insert_deal(&pool, party, "REF-1", "Draft Deal", "DRAFT", created_at).await;
    insert_deal(
        &pool,
        party,
        "REF-2",
        "Completed Deal",
        "COMPLETED",
        created_at,
    )
    .await;

    let repo = PostgresAnalyticsRepository::new(pool.clone());
    repo.refresh_daily_metrics(date).await.unwrap();

    let count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM deals")
        .fetch_one(&pool)
        .await
        .unwrap()
        .unwrap();
    let row = sqlx::query!(
        "SELECT date, total_deals, deals_completed FROM platform_metrics WHERE date = $1",
        date
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    eprintln!(
        "DEBUG dashboard: deals={count}, metrics date={:?} total={} completed={}",
        row.date, row.total_deals, row.deals_completed
    );

    let raw_summary = sqlx::query!("SELECT date, total_deals, deals_completed FROM platform_metrics ORDER BY date DESC LIMIT 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    eprintln!(
        "DEBUG raw latest: date={:?} total={} completed={}",
        raw_summary.date, raw_summary.total_deals, raw_summary.deals_completed
    );

    let summary = repo.get_dashboard_summary().await.unwrap();
    eprintln!(
        "DEBUG summary: total_deals={} completed_deals={}",
        summary.total_deals, summary.completed_deals
    );
    assert_eq!(summary.total_deals, 2);
    assert_eq!(summary.completed_deals, 1);
    assert_eq!(summary.active_deals, 1);
    assert_eq!(summary.total_parties, 1);
    assert_eq!(summary.active_parties, 1);
    assert_eq!(summary.total_users, 1);
    assert_eq!(summary.active_users, 1);

    let metrics = repo
        .list_daily_metrics(MetricFilters {
            from_date: Some(date),
            to_date: Some(date),
            limit: 10,
            offset: 0,
        })
        .await
        .unwrap();
    assert_eq!(metrics.total, 1);
    assert_eq!(metrics.items.len(), 1);
    assert_eq!(metrics.items[0].total_deals, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn deal_trends_and_party_activity(pool: PgPool) {
    let _user = create_user(&pool, "trend_user@example.com", "trend_user").await;
    let party = create_party(&pool, "trend_party@example.com", "Trend Farm").await;

    let first_date = date_at(2999, Month::January, 1);
    let second_date = date_at(2999, Month::January, 2);

    insert_deal(
        &pool,
        party,
        "YESTERDAY-1",
        "Yesterday Deal",
        "DRAFT",
        start_of_day(first_date),
    )
    .await;
    insert_deal(
        &pool,
        party,
        "TODAY-1",
        "Today Deal",
        "COMPLETED",
        start_of_day(second_date),
    )
    .await;

    let repo = PostgresAnalyticsRepository::new(pool);
    repo.refresh_daily_metrics(first_date).await.unwrap();
    repo.refresh_daily_metrics(second_date).await.unwrap();

    let trends = repo.get_deal_trends(first_date, second_date).await.unwrap();
    assert_eq!(trends.len(), 2);
    assert_eq!(trends[0].total_deals, 1);
    assert_eq!(trends[0].completed_deals, 0);
    assert_eq!(trends[1].total_deals, 2);
    assert_eq!(trends[1].completed_deals, 1);

    let activity = repo
        .get_party_activity(first_date, second_date)
        .await
        .unwrap();
    assert_eq!(activity.len(), 2);
    assert_eq!(activity[0].total_parties, 1);
    assert_eq!(activity[1].total_parties, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_daily_metrics_with_filters(pool: PgPool) {
    let _user = create_user(&pool, "metrics_user@example.com", "metrics_user").await;
    let party = create_party(&pool, "metrics_party@example.com", "Metrics Farm").await;

    let date = date_at(2998, Month::January, 1);
    insert_deal(
        &pool,
        party,
        "METRICS-1",
        "Metrics Deal",
        "DRAFT",
        start_of_day(date),
    )
    .await;

    let repo = PostgresAnalyticsRepository::new(pool);
    repo.refresh_daily_metrics(date).await.unwrap();

    let all = repo
        .list_daily_metrics(MetricFilters {
            limit: 10,
            offset: 0,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(all.total, 1);
    assert_eq!(all.items.len(), 1);
    assert_eq!(all.items[0].total_deals, 1);

    let filtered = repo
        .list_daily_metrics(MetricFilters {
            from_date: Some(date),
            to_date: Some(date),
            limit: 10,
            offset: 0,
        })
        .await
        .unwrap();
    assert_eq!(filtered.total, 1);
    assert_eq!(filtered.items.len(), 1);

    let paged = repo
        .list_daily_metrics(MetricFilters {
            limit: 1,
            offset: 1,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(paged.total, 1);
    assert!(paged.items.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_daily_metrics_empty_for_unused_date(pool: PgPool) {
    let repo = PostgresAnalyticsRepository::new(pool);
    let unused = date_at(1800, Month::January, 1);

    let metrics = repo
        .list_daily_metrics(MetricFilters {
            from_date: Some(unused),
            to_date: Some(unused),
            limit: 10,
            offset: 0,
        })
        .await
        .unwrap();
    assert_eq!(metrics.total, 0);
    assert!(metrics.items.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn refresh_fails_when_no_deals_for_date(pool: PgPool) {
    let repo = PostgresAnalyticsRepository::new(pool);
    let unused = date_at(1500, Month::January, 1);

    let err = repo.refresh_daily_metrics(unused).await.unwrap_err();
    assert!(matches!(err, DomainError::RepositoryError(_)));
}
