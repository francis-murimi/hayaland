use domain::entities::{
    DisplayName, Email, Enhancement, Need, Party, PartyType, PasswordHash, Resource, User,
    Username,
};
use domain::repositories::{CatalogRepository, PartyRepository, SearchRepository, SearchTarget, UserRepository};
use infrastructure::repositories::{
    PostgresCatalogRepository, PostgresPartyRepository, PostgresSearchRepository,
    PostgresUserRepository,
};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

fn agriculture_domain_id() -> Uuid {
    Uuid::parse_str("a1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap()
}

fn farmland_resource_type_id() -> Uuid {
    Uuid::parse_str("f6a7b8c9-d0e1-2345-fabc-456789012345").unwrap()
}

fn crop_produce_need_type_id() -> Uuid {
    Uuid::parse_str("a7b8c9d0-e1f2-3456-abcd-567890123456").unwrap()
}

fn agro_inputs_enhancement_type_id() -> Uuid {
    Uuid::parse_str("b8c9d0e1-f2a3-4567-bcde-678901234567").unwrap()
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

async fn create_public_deal(pool: &PgPool, title: &str, party_id: Uuid) -> Uuid {
    let id = Uuid::now_v7();
    let reference = format!("DEAL-{id}");
    sqlx::query!(
        r#"
        INSERT INTO deals (
            id, deal_reference, deal_title, deal_description, domain_category_id,
            initiator_party_id, initiator_role, deal_status, is_public, total_deal_value,
            created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, 'SUPPLIER', 'DRAFT', true, 1000, now(), now())
        "#,
        id,
        reference,
        title,
        Some("A public deal description".to_string()),
        agriculture_domain_id(),
        party_id
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

fn sample_resource(party_id: Uuid) -> Resource {
    Resource::new(
        Uuid::now_v7(),
        party_id,
        farmland_resource_type_id(),
        "Irrigated Farmland".to_string(),
        Decimal::from(10),
        "acres".to_string(),
    )
    .unwrap()
}

fn sample_need(party_id: Uuid) -> Need {
    Need::new(
        Uuid::now_v7(),
        party_id,
        crop_produce_need_type_id(),
        "I need organic produce for my store.".to_string(),
        Decimal::from(1000),
        "lbs".to_string(),
    )
    .unwrap()
}

fn sample_enhancement(party_id: Uuid) -> Enhancement {
    Enhancement::new(
        Uuid::now_v7(),
        party_id,
        agro_inputs_enhancement_type_id(),
        "Full Season Agricultural Support".to_string(),
    )
    .unwrap()
}

#[sqlx::test(migrations = "../../migrations")]
async fn search_parties_by_name_and_email(pool: PgPool) {
    let _user = create_user(&pool, "party_search_user@example.com", "party_search_user").await;
    let party_id = create_party(&pool, "sunrise@example.com", "Sunrise Farm").await;
    let repo = PostgresSearchRepository::new(pool);

    let by_name = repo.search("Sunrise", SearchTarget::Party, 10, 0).await.unwrap();
    assert_eq!(by_name.total, 1);
    assert!(matches!(&by_name.items[0], domain::repositories::SearchResultItem::Party(p) if p.id == party_id));

    let by_word = repo.search("Farm", SearchTarget::Party, 10, 0).await.unwrap();
    assert_eq!(by_word.total, 1);

    let by_email = repo.search("sunrise@example.com", SearchTarget::Party, 10, 0)
        .await
        .unwrap();
    assert_eq!(by_email.total, 1);

    let empty = repo.search("NoSuchParty", SearchTarget::Party, 10, 0).await.unwrap();
    assert_eq!(empty.total, 0);
    assert!(empty.items.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn search_resources_by_name_and_description(pool: PgPool) {
    let supplier = create_party(&pool, "supplier_search@example.com", "Supplier Farm").await;
    let catalog_repo = PostgresCatalogRepository::new(pool.clone());
    let mut resource = sample_resource(supplier);
    resource.description = Some("Fertile organic soil".to_string());
    catalog_repo.create_resource(&resource).await.unwrap();

    let repo = PostgresSearchRepository::new(pool);

    let by_name = repo.search("Farmland", SearchTarget::Resource, 10, 0).await.unwrap();
    assert_eq!(by_name.total, 1);
    assert!(matches!(&by_name.items[0], domain::repositories::SearchResultItem::Resource(r) if r.id == resource.id));

    let by_desc = repo.search("organic", SearchTarget::Resource, 10, 0).await.unwrap();
    assert_eq!(by_desc.total, 1);

    let empty = repo.search("concrete", SearchTarget::Resource, 10, 0).await.unwrap();
    assert_eq!(empty.total, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn search_needs_by_description(pool: PgPool) {
    let consumer = create_party(&pool, "consumer_search@example.com", "Consumer Store").await;
    let catalog_repo = PostgresCatalogRepository::new(pool.clone());
    let need = sample_need(consumer);
    catalog_repo.create_need(&need).await.unwrap();

    let repo = PostgresSearchRepository::new(pool);

    let result = repo.search("organic", SearchTarget::Need, 10, 0).await.unwrap();
    assert_eq!(result.total, 1);
    assert!(matches!(&result.items[0], domain::repositories::SearchResultItem::Need(n) if n.id == need.id));

    let empty = repo.search("machinery", SearchTarget::Need, 10, 0).await.unwrap();
    assert_eq!(empty.total, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn search_enhancements_by_name(pool: PgPool) {
    let enhancer = create_party(&pool, "enhancer_search@example.com", "Enhancer Services").await;
    let catalog_repo = PostgresCatalogRepository::new(pool.clone());
    let enhancement = sample_enhancement(enhancer);
    catalog_repo.create_enhancement(&enhancement).await.unwrap();

    let repo = PostgresSearchRepository::new(pool);

    let result = repo.search("agricultural", SearchTarget::Enhancement, 10, 0)
        .await
        .unwrap();
    assert_eq!(result.total, 1);
    assert!(matches!(&result.items[0], domain::repositories::SearchResultItem::Enhancement(e) if e.id == enhancement.id));

    let empty = repo.search("plumbing", SearchTarget::Enhancement, 10, 0).await.unwrap();
    assert_eq!(empty.total, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn search_deals_by_title(pool: PgPool) {
    let party = create_party(&pool, "deal_search@example.com", "Deal Party").await;
    let public_deal_id = create_public_deal(&pool, "Community Farming Partnership", party).await;
    let _private_deal_id = create_public_deal(&pool, "Private Agriculture Deal", party).await;
    // Flip the private deal back to is_public = false so only the public one is searchable.
    sqlx::query!("UPDATE deals SET is_public = false WHERE deal_title = $1", "Private Agriculture Deal")
        .execute(&pool)
        .await
        .unwrap();

    let repo = PostgresSearchRepository::new(pool);

    let result = repo.search("farming", SearchTarget::Deal, 10, 0).await.unwrap();
    assert_eq!(result.total, 1);
    assert!(matches!(&result.items[0], domain::repositories::SearchResultItem::Deal(d) if d.id == public_deal_id));

    let empty = repo.search("real estate", SearchTarget::Deal, 10, 0).await.unwrap();
    assert_eq!(empty.total, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn search_pagination_and_empty_query(pool: PgPool) {
    let _user = create_user(&pool, "pagination_user@example.com", "pagination_user").await;
    let first = create_party(&pool, "first@example.com", "First Farm").await;
    let second = create_party(&pool, "second@example.com", "Second Farm").await;
    let repo = PostgresSearchRepository::new(pool);

    let all = repo.search("", SearchTarget::Party, 10, 0).await.unwrap();
    assert_eq!(all.total, 2);

    let page = repo.search("", SearchTarget::Party, 1, 0).await.unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.total, 2);

    let page_two = repo.search("", SearchTarget::Party, 1, 1).await.unwrap();
    assert_eq!(page_two.items.len(), 1);
    assert_eq!(page_two.total, 2);

    let ids: Vec<Uuid> = page.items.iter().chain(page_two.items.iter()).map(|item| {
        match item {
            domain::repositories::SearchResultItem::Party(p) => p.id,
            _ => panic!("expected party"),
        }
    }).collect();
    assert!(ids.contains(&first));
    assert!(ids.contains(&second));
}
