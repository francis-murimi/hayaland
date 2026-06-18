use domain::entities::{
    Deal, DealParticipation, DealRole, DealTitle, DisplayName, Email, Enhancement, GeoPoint, Need,
    Party, PartyType, PasswordHash, Resource, ResourceCondition, RoleProfile, User, Username,
};
use domain::repositories::{
    AdminFlags, CatalogItemStatus, CatalogItemType, CatalogRepository, CatalogSearchCriteria,
    CatalogSort, DealAggregate, DealRepository, PartyRepository, UserRepository,
};
use infrastructure::repositories::{
    PostgresCatalogRepository, PostgresDealRepository, PostgresPartyRepository,
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

fn sample_party(email: &str, name: &str) -> Party {
    Party::new(
        Uuid::now_v7(),
        PartyType::Organization,
        DisplayName::new(name).unwrap(),
        Email::new(email).unwrap(),
    )
}

async fn create_party(
    repo: &PostgresPartyRepository,
    role: DealRole,
    email: &str,
    name: &str,
) -> Uuid {
    let party = sample_party(email, name);
    let id = party.id;
    repo.create(&party).await.unwrap();
    repo.add_role(id, role, RoleProfile::for_role(role))
        .await
        .unwrap();
    id
}

async fn create_user(repo: &PostgresUserRepository, email: &str, username: &str) -> Uuid {
    let user = User::new(
        Uuid::now_v7(),
        Email::new(email).unwrap(),
        Username::new(username).unwrap(),
        PasswordHash::new("hash".to_string()).unwrap(),
    );
    let id = user.id;
    repo.create(&user).await.unwrap();
    id
}

async fn create_deal(
    pool: PgPool,
    supplier: Uuid,
    consumer: Uuid,
    enhancer: Uuid,
    reference: &str,
) -> Uuid {
    let deal_repo = PostgresDealRepository::new(pool);

    let deal_id = Uuid::now_v7();
    let deal = Deal::new(
        deal_id,
        reference.to_string(),
        DealTitle::new("Sample Deal").unwrap(),
        agriculture_domain_id(),
        supplier,
        DealRole::Supplier,
    );

    let participations = vec![
        DealParticipation::new(Uuid::now_v7(), deal_id, supplier, DealRole::Supplier, true),
        DealParticipation::new(Uuid::now_v7(), deal_id, consumer, DealRole::Consumer, false),
        DealParticipation::new(Uuid::now_v7(), deal_id, enhancer, DealRole::Enhancer, false),
    ];

    deal_repo
        .create(&DealAggregate {
            deal,
            participations,
        })
        .await
        .unwrap();

    deal_id
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
async fn creates_and_finds_resource(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool);
    let party_id = create_party(&party_repo, DealRole::Supplier, "r@example.com", "R Farm").await;

    let resource = sample_resource(party_id);
    let id = resource.id;

    repo.create_resource(&resource).await.unwrap();

    let found = repo.find_resource_by_id(id).await.unwrap();
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id, id);
    assert_eq!(found.supplier_party_id, party_id);
    assert_eq!(found.resource_name, "Irrigated Farmland");
    assert!(found.is_active);
}

#[sqlx::test(migrations = "../../migrations")]
async fn creates_and_finds_need(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool);
    let party_id = create_party(&party_repo, DealRole::Consumer, "n@example.com", "N Store").await;

    let need = sample_need(party_id);
    let id = need.id;

    repo.create_need(&need).await.unwrap();

    let found = repo.find_need_by_id(id).await.unwrap();
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id, id);
    assert_eq!(found.consumer_party_id, party_id);
    assert_eq!(
        found.need_description,
        "I need organic produce for my store."
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn creates_and_finds_enhancement(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool);
    let party_id = create_party(
        &party_repo,
        DealRole::Enhancer,
        "e@example.com",
        "E Services",
    )
    .await;

    let enhancement = sample_enhancement(party_id);
    let id = enhancement.id;

    repo.create_enhancement(&enhancement).await.unwrap();

    let found = repo.find_enhancement_by_id(id).await.unwrap();
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id, id);
    assert_eq!(found.enhancer_party_id, party_id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn updates_resource(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool);
    let party_id = create_party(&party_repo, DealRole::Supplier, "ru@example.com", "RU Farm").await;

    let mut resource = sample_resource(party_id);
    repo.create_resource(&resource).await.unwrap();

    resource.set_name("Updated Farmland".to_string()).unwrap();
    resource.set_quantity(Decimal::from(20)).unwrap();
    resource.set_condition(Some(ResourceCondition::Good));
    resource.set_location(Some(GeoPoint::new(37.7749, -122.4194).unwrap()));
    repo.update_resource(&resource).await.unwrap();

    let found = repo
        .find_resource_by_id(resource.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.resource_name, "Updated Farmland");
    assert_eq!(found.quantity, Decimal::from(20));
    assert_eq!(found.condition, Some(ResourceCondition::Good));
    assert_eq!(
        found.location,
        Some(GeoPoint::new(37.7749, -122.4194).unwrap())
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn updates_need(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool);
    let party_id = create_party(
        &party_repo,
        DealRole::Consumer,
        "nu@example.com",
        "NU Store",
    )
    .await;

    let mut need = sample_need(party_id);
    repo.create_need(&need).await.unwrap();

    need.set_description("I need organic vegetables for my restaurant.".to_string())
        .unwrap();
    need.set_quantity(Decimal::from(500)).unwrap();
    need.set_location(Some(GeoPoint::new(37.7749, -122.4194).unwrap()));
    repo.update_need(&need).await.unwrap();

    let found = repo.find_need_by_id(need.id).await.unwrap().unwrap();
    assert_eq!(
        found.need_description,
        "I need organic vegetables for my restaurant."
    );
    assert_eq!(found.required_quantity, Decimal::from(500));
}

#[sqlx::test(migrations = "../../migrations")]
async fn updates_enhancement(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool);
    let party_id = create_party(
        &party_repo,
        DealRole::Enhancer,
        "eu@example.com",
        "EU Services",
    )
    .await;

    let mut enhancement = sample_enhancement(party_id);
    repo.create_enhancement(&enhancement).await.unwrap();

    enhancement
        .set_name("Updated Agricultural Support".to_string())
        .unwrap();
    enhancement
        .set_input_quantity(Some(Decimal::from(5)))
        .unwrap();
    enhancement.mark_complete();
    repo.update_enhancement(&enhancement).await.unwrap();

    let found = repo
        .find_enhancement_by_id(enhancement.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.enhancement_name, "Updated Agricultural Support");
    assert_eq!(found.input_quantity, Some(Decimal::from(5)));
    assert!(found.is_complete);
}

#[sqlx::test(migrations = "../../migrations")]
async fn soft_deletes_resource(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool);
    let party_id = create_party(
        &party_repo,
        DealRole::Supplier,
        "rsd@example.com",
        "RSD Farm",
    )
    .await;

    let mut resource = sample_resource(party_id);
    repo.create_resource(&resource).await.unwrap();

    resource.set_active(false);
    repo.update_resource(&resource).await.unwrap();

    let found = repo
        .find_resource_by_id(resource.id)
        .await
        .unwrap()
        .unwrap();
    assert!(!found.is_active);
}

#[sqlx::test(migrations = "../../migrations")]
async fn hard_deletes_resource_when_no_deals(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool);
    let party_id = create_party(
        &party_repo,
        DealRole::Supplier,
        "rhd@example.com",
        "RHD Farm",
    )
    .await;

    let resource = sample_resource(party_id);
    let id = resource.id;
    repo.create_resource(&resource).await.unwrap();

    repo.delete_resource(id).await.unwrap();

    let found = repo.find_resource_by_id(id).await.unwrap();
    assert!(found.is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn hard_delete_fails_when_resource_has_deals(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool.clone());

    let supplier = create_party(
        &party_repo,
        DealRole::Supplier,
        "rhs@example.com",
        "RHS Farm",
    )
    .await;
    let _consumer = create_party(
        &party_repo,
        DealRole::Consumer,
        "rhc@example.com",
        "RHC Store",
    )
    .await;
    let _enhancer = create_party(
        &party_repo,
        DealRole::Enhancer,
        "rhe@example.com",
        "RHE Services",
    )
    .await;

    let resource = sample_resource(supplier);
    let id = resource.id;
    repo.create_resource(&resource).await.unwrap();

    repo.increment_deal_count(CatalogItemType::Resource, id)
        .await
        .unwrap();

    let err = repo.delete_resource(id).await.unwrap_err();
    assert!(matches!(
        err,
        domain::errors::DomainError::CatalogItemHasActiveDeals
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn hard_deletes_need_and_enhancement(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool);

    let consumer = create_party(
        &party_repo,
        DealRole::Consumer,
        "nhd@example.com",
        "NHD Store",
    )
    .await;
    let enhancer = create_party(
        &party_repo,
        DealRole::Enhancer,
        "ehd@example.com",
        "EHD Services",
    )
    .await;

    let need = sample_need(consumer);
    let enhancement = sample_enhancement(enhancer);
    repo.create_need(&need).await.unwrap();
    repo.create_enhancement(&enhancement).await.unwrap();

    repo.delete_need(need.id).await.unwrap();
    repo.delete_enhancement(enhancement.id).await.unwrap();

    assert!(repo.find_need_by_id(need.id).await.unwrap().is_none());
    assert!(repo
        .find_enhancement_by_id(enhancement.id)
        .await
        .unwrap()
        .is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn lists_resources_with_party_filter(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool);

    let party_a = create_party(
        &party_repo,
        DealRole::Supplier,
        "rpa@example.com",
        "RPA Farm",
    )
    .await;
    let party_b = create_party(
        &party_repo,
        DealRole::Supplier,
        "rpb@example.com",
        "RPB Farm",
    )
    .await;

    repo.create_resource(&sample_resource(party_a))
        .await
        .unwrap();
    repo.create_resource(&sample_resource(party_b))
        .await
        .unwrap();
    repo.create_resource(&sample_resource(party_b))
        .await
        .unwrap();

    let criteria = CatalogSearchCriteria {
        party_id: Some(party_b),
        limit: 10,
        offset: 0,
        ..Default::default()
    };

    let result = repo.list_resources(&criteria).await.unwrap();
    assert_eq!(result.items.len(), 2);
    assert_eq!(result.total, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn lists_resources_with_category_filter(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool);

    let party_id = create_party(&party_repo, DealRole::Supplier, "rc@example.com", "RC Farm").await;

    let mut resource = sample_resource(party_id);
    resource.resource_type_id = farmland_resource_type_id();
    repo.create_resource(&resource).await.unwrap();

    let criteria = CatalogSearchCriteria {
        category_id: Some(farmland_resource_type_id()),
        limit: 10,
        offset: 0,
        ..Default::default()
    };

    let result = repo.list_resources(&criteria).await.unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.total, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn lists_resources_with_domain_filter(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool);

    let party_id = create_party(&party_repo, DealRole::Supplier, "rd@example.com", "RD Farm").await;

    repo.create_resource(&sample_resource(party_id))
        .await
        .unwrap();

    let criteria = CatalogSearchCriteria {
        domain_category_id: Some(agriculture_domain_id()),
        limit: 10,
        offset: 0,
        ..Default::default()
    };

    let result = repo.list_resources(&criteria).await.unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.total, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn text_search_finds_resources(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool.clone());

    let party_id = create_party(&party_repo, DealRole::Supplier, "rt@example.com", "RT Farm").await;

    let mut matching = sample_resource(party_id);
    matching.resource_name = "Organic Apple Orchard".to_string();
    repo.create_resource(&matching).await.unwrap();

    let mut other = sample_resource(party_id);
    other.resource_name = "Concrete Mixer".to_string();
    repo.create_resource(&other).await.unwrap();

    sqlx::query!("SELECT set_limit(0.1)")
        .fetch_one(&pool)
        .await
        .unwrap();

    let criteria = CatalogSearchCriteria {
        query: Some("apple".to_string()),
        sort: CatalogSort::Relevance,
        limit: 10,
        offset: 0,
        ..Default::default()
    };

    let result = repo.list_resources(&criteria).await.unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].id, matching.id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn geo_search_finds_nearby_resources(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool);

    let party_id = create_party(&party_repo, DealRole::Supplier, "rg@example.com", "RG Farm").await;

    let mut nearby = sample_resource(party_id);
    nearby.set_location(Some(GeoPoint::new(37.7749, -122.4194).unwrap()));
    repo.create_resource(&nearby).await.unwrap();

    let mut far = sample_resource(party_id);
    far.set_location(Some(GeoPoint::new(40.7128, -74.0060).unwrap()));
    repo.create_resource(&far).await.unwrap();

    let criteria = CatalogSearchCriteria {
        geo: Some(domain::repositories::GeoSearch {
            latitude: 37.7749,
            longitude: -122.4194,
            radius_km: 10.0,
        }),
        sort: CatalogSort::Nearest,
        limit: 10,
        offset: 0,
        ..Default::default()
    };

    let result = repo.list_resources(&criteria).await.unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].id, nearby.id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn pagination_works(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool);

    let party_id = create_party(
        &party_repo,
        DealRole::Supplier,
        "rpg@example.com",
        "RPG Farm",
    )
    .await;

    for i in 0..5 {
        let mut resource = sample_resource(party_id);
        resource.resource_name = format!("Resource {i}");
        repo.create_resource(&resource).await.unwrap();
    }

    let criteria = CatalogSearchCriteria {
        limit: 2,
        offset: 0,
        ..Default::default()
    };

    let page1 = repo.list_resources(&criteria).await.unwrap();
    assert_eq!(page1.items.len(), 2);
    assert_eq!(page1.total, 5);

    let criteria = CatalogSearchCriteria {
        limit: 2,
        offset: 2,
        ..Default::default()
    };

    let page2 = repo.list_resources(&criteria).await.unwrap();
    assert_eq!(page2.items.len(), 2);
    assert_eq!(page2.total, 5);
}

#[sqlx::test(migrations = "../../migrations")]
async fn admin_hide_unhide_affects_public_list(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let user_repo = PostgresUserRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool);

    let party_id = create_party(
        &party_repo,
        DealRole::Supplier,
        "rah@example.com",
        "RAH Farm",
    )
    .await;
    let admin_id = create_user(&user_repo, "admin@example.com", "adminuser").await;

    let resource = sample_resource(party_id);
    let id = resource.id;
    repo.create_resource(&resource).await.unwrap();

    let criteria = CatalogSearchCriteria {
        limit: 10,
        offset: 0,
        ..Default::default()
    };

    let result = repo.list_resources(&criteria).await.unwrap();
    assert_eq!(result.items.len(), 1);

    repo.update_resource_admin_flags(
        id,
        AdminFlags {
            platform_hidden: Some(true),
            platform_featured: None,
            admin_notes: Some("spam".to_string()),
            admin_reviewed_by: Some(admin_id),
        },
    )
    .await
    .unwrap();

    let result = repo.list_resources(&criteria).await.unwrap();
    assert_eq!(result.items.len(), 0);

    let hidden = repo.find_resource_by_id(id).await.unwrap().unwrap();
    assert!(hidden.platform_hidden);
    assert!(hidden.admin_reviewed_at.is_some());
    assert_eq!(hidden.admin_reviewed_by, Some(admin_id));

    repo.update_resource_admin_flags(
        id,
        AdminFlags {
            platform_hidden: Some(false),
            platform_featured: None,
            admin_notes: None,
            admin_reviewed_by: None,
        },
    )
    .await
    .unwrap();

    let result = repo.list_resources(&criteria).await.unwrap();
    assert_eq!(result.items.len(), 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn status_filter_lists_inactive_when_requested(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool);

    let party_id = create_party(
        &party_repo,
        DealRole::Supplier,
        "rst@example.com",
        "RST Farm",
    )
    .await;

    let mut active = sample_resource(party_id);
    active.resource_name = "Active Resource".to_string();
    repo.create_resource(&active).await.unwrap();

    let mut inactive = sample_resource(party_id);
    inactive.resource_name = "Inactive Resource".to_string();
    inactive.set_active(false);
    repo.create_resource(&inactive).await.unwrap();

    let criteria = CatalogSearchCriteria {
        status: Some(CatalogItemStatus::Inactive),
        limit: 10,
        offset: 0,
        ..Default::default()
    };

    let result = repo.list_resources(&criteria).await.unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].id, inactive.id);

    let criteria = CatalogSearchCriteria {
        status: Some(CatalogItemStatus::All),
        include_hidden: true,
        limit: 10,
        offset: 0,
        ..Default::default()
    };

    let result = repo.list_resources(&criteria).await.unwrap();
    assert_eq!(result.items.len(), 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn verified_only_filter_works(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool);

    let party_id = create_party(&party_repo, DealRole::Supplier, "rv@example.com", "RV Farm").await;

    let mut verified = sample_resource(party_id);
    verified.resource_name = "Verified Resource".to_string();
    verified.verified_by_platform = true;
    repo.create_resource(&verified).await.unwrap();

    let mut unverified = sample_resource(party_id);
    unverified.resource_name = "Unverified Resource".to_string();
    repo.create_resource(&unverified).await.unwrap();

    let criteria = CatalogSearchCriteria {
        verified_only: true,
        limit: 10,
        offset: 0,
        ..Default::default()
    };

    let result = repo.list_resources(&criteria).await.unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].id, verified.id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn featured_only_filter_works(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool);

    let party_id = create_party(&party_repo, DealRole::Supplier, "rf@example.com", "RF Farm").await;

    let mut featured = sample_resource(party_id);
    featured.resource_name = "Featured Resource".to_string();
    featured.platform_featured = true;
    repo.create_resource(&featured).await.unwrap();

    let mut normal = sample_resource(party_id);
    normal.resource_name = "Normal Resource".to_string();
    repo.create_resource(&normal).await.unwrap();

    let criteria = CatalogSearchCriteria {
        featured_only: true,
        limit: 10,
        offset: 0,
        ..Default::default()
    };

    let result = repo.list_resources(&criteria).await.unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].id, featured.id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn deal_binding_increments_count(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool);

    let supplier = create_party(
        &party_repo,
        DealRole::Supplier,
        "rbs@example.com",
        "RBS Farm",
    )
    .await;
    let consumer = create_party(
        &party_repo,
        DealRole::Consumer,
        "rbc@example.com",
        "RBC Store",
    )
    .await;
    let enhancer = create_party(
        &party_repo,
        DealRole::Enhancer,
        "rbe@example.com",
        "RBE Services",
    )
    .await;

    let resource = sample_resource(supplier);
    let need = sample_need(consumer);
    let enhancement = sample_enhancement(enhancer);

    repo.create_resource(&resource).await.unwrap();
    repo.create_need(&need).await.unwrap();
    repo.create_enhancement(&enhancement).await.unwrap();

    repo.increment_deal_count(CatalogItemType::Resource, resource.id)
        .await
        .unwrap();
    repo.increment_deal_count(CatalogItemType::Need, need.id)
        .await
        .unwrap();
    repo.increment_deal_count(CatalogItemType::Enhancement, enhancement.id)
        .await
        .unwrap();

    let found_resource = repo
        .find_resource_by_id(resource.id)
        .await
        .unwrap()
        .unwrap();
    let found_need = repo.find_need_by_id(need.id).await.unwrap().unwrap();
    let found_enhancement = repo
        .find_enhancement_by_id(enhancement.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(found_resource.deal_count, 1);
    assert_eq!(found_need.deal_count, 1);
    assert_eq!(found_enhancement.deal_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn finds_catalog_items_by_deal(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let _deal_repo = PostgresDealRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool.clone());

    let supplier = create_party(
        &party_repo,
        DealRole::Supplier,
        "fds@example.com",
        "FDS Farm",
    )
    .await;
    let consumer = create_party(
        &party_repo,
        DealRole::Consumer,
        "fdc@example.com",
        "FDC Store",
    )
    .await;
    let enhancer = create_party(
        &party_repo,
        DealRole::Enhancer,
        "fde@example.com",
        "FDE Services",
    )
    .await;

    let deal_id = create_deal(pool, supplier, consumer, enhancer, "DL-CAT-0001").await;

    let mut resource = sample_resource(supplier);
    resource.deal_id = Some(deal_id);
    repo.create_resource(&resource).await.unwrap();

    let mut need = sample_need(consumer);
    need.deal_id = Some(deal_id);
    repo.create_need(&need).await.unwrap();

    let mut enhancement = sample_enhancement(enhancer);
    enhancement.deal_id = Some(deal_id);
    repo.create_enhancement(&enhancement).await.unwrap();

    let resources = repo.find_resources_by_deal(deal_id).await.unwrap();
    let needs = repo.find_needs_by_deal(deal_id).await.unwrap();
    let enhancements = repo.find_enhancements_by_deal(deal_id).await.unwrap();

    assert_eq!(resources.len(), 1);
    assert_eq!(needs.len(), 1);
    assert_eq!(enhancements.len(), 1);
    assert_eq!(resources[0].id, resource.id);
    assert_eq!(needs[0].id, need.id);
    assert_eq!(enhancements[0].id, enhancement.id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn counts_for_party(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool);

    let party_id = create_party(
        &party_repo,
        DealRole::Supplier,
        "rcp@example.com",
        "RCP Farm",
    )
    .await;

    repo.create_resource(&sample_resource(party_id))
        .await
        .unwrap();
    repo.create_resource(&sample_resource(party_id))
        .await
        .unwrap();

    let count = repo.count_resources_for_party(party_id).await.unwrap();
    assert_eq!(count, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn count_active_items_by_category(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool);

    let supplier = create_party(
        &party_repo,
        DealRole::Supplier,
        "cacs@example.com",
        "CACS Farm",
    )
    .await;
    let consumer = create_party(
        &party_repo,
        DealRole::Consumer,
        "cacc@example.com",
        "CACC Store",
    )
    .await;
    let enhancer = create_party(
        &party_repo,
        DealRole::Enhancer,
        "cace@example.com",
        "CACE Services",
    )
    .await;

    repo.create_resource(&sample_resource(supplier))
        .await
        .unwrap();
    repo.create_need(&sample_need(consumer)).await.unwrap();
    repo.create_enhancement(&sample_enhancement(enhancer))
        .await
        .unwrap();

    let counts = repo
        .count_active_items_by_category(farmland_resource_type_id())
        .await
        .unwrap();
    assert_eq!(counts.resource_count, 1);
    assert_eq!(counts.need_count, 0);
    assert_eq!(counts.enhancement_count, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn lists_needs_and_enhancements(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool);

    let consumer = create_party(
        &party_repo,
        DealRole::Consumer,
        "lnc@example.com",
        "LNC Store",
    )
    .await;
    let enhancer = create_party(
        &party_repo,
        DealRole::Enhancer,
        "lne@example.com",
        "LNE Services",
    )
    .await;

    let need = sample_need(consumer);
    let enhancement = sample_enhancement(enhancer);
    repo.create_need(&need).await.unwrap();
    repo.create_enhancement(&enhancement).await.unwrap();

    let need_result = repo
        .list_needs(&CatalogSearchCriteria {
            party_id: Some(consumer),
            limit: 10,
            offset: 0,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(need_result.items.len(), 1);
    assert_eq!(need_result.total, 1);

    let enhancement_result = repo
        .list_enhancements(&CatalogSearchCriteria {
            party_id: Some(enhancer),
            limit: 10,
            offset: 0,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(enhancement_result.items.len(), 1);
    assert_eq!(enhancement_result.total, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn admin_flags_for_need_and_enhancement(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let user_repo = PostgresUserRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool);

    let consumer = create_party(
        &party_repo,
        DealRole::Consumer,
        "afn@example.com",
        "AFN Store",
    )
    .await;
    let enhancer = create_party(
        &party_repo,
        DealRole::Enhancer,
        "afe@example.com",
        "AFE Services",
    )
    .await;
    let admin_id = create_user(&user_repo, "afadmin@example.com", "afadmin").await;

    let need = sample_need(consumer);
    let enhancement = sample_enhancement(enhancer);
    repo.create_need(&need).await.unwrap();
    repo.create_enhancement(&enhancement).await.unwrap();

    repo.update_need_admin_flags(
        need.id,
        AdminFlags {
            platform_hidden: Some(true),
            platform_featured: None,
            admin_notes: None,
            admin_reviewed_by: Some(admin_id),
        },
    )
    .await
    .unwrap();

    repo.update_enhancement_admin_flags(
        enhancement.id,
        AdminFlags {
            platform_hidden: Some(true),
            platform_featured: None,
            admin_notes: None,
            admin_reviewed_by: Some(admin_id),
        },
    )
    .await
    .unwrap();

    let hidden_need = repo.find_need_by_id(need.id).await.unwrap().unwrap();
    let hidden_enhancement = repo
        .find_enhancement_by_id(enhancement.id)
        .await
        .unwrap()
        .unwrap();

    assert!(hidden_need.platform_hidden);
    assert!(hidden_need.admin_reviewed_at.is_some());
    assert!(hidden_enhancement.platform_hidden);
    assert!(hidden_enhancement.admin_reviewed_at.is_some());
}

#[sqlx::test(migrations = "../../migrations")]
async fn counts_needs_and_enhancements_for_party(pool: PgPool) {
    let party_repo = PostgresPartyRepository::new(pool.clone());
    let repo = PostgresCatalogRepository::new(pool);

    let consumer = create_party(
        &party_repo,
        DealRole::Consumer,
        "cnp@example.com",
        "CNP Store",
    )
    .await;
    let enhancer = create_party(
        &party_repo,
        DealRole::Enhancer,
        "cep@example.com",
        "CEP Services",
    )
    .await;

    repo.create_need(&sample_need(consumer)).await.unwrap();
    repo.create_need(&sample_need(consumer)).await.unwrap();
    repo.create_enhancement(&sample_enhancement(enhancer))
        .await
        .unwrap();

    assert_eq!(repo.count_needs_for_party(consumer).await.unwrap(), 2);
    assert_eq!(
        repo.count_enhancements_for_party(enhancer).await.unwrap(),
        1
    );
}
