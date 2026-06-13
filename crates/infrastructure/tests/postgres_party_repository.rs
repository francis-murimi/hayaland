use domain::entities::{
    DealRole, DisplayName, Email, GeoPoint, Party, PartyMembershipRole, PartyType, RoleProfile,
    UserPartyMembership, VerificationStatus,
};
use domain::entities::{PasswordHash, User, Username};
use domain::repositories::{PartyRepository, PartySearchCriteria, UserRepository};
use infrastructure::repositories::{PostgresPartyRepository, PostgresUserRepository};
use sqlx::PgPool;
use uuid::Uuid;

fn sample_party(email: &str) -> Party {
    Party::new(
        Uuid::now_v7(),
        PartyType::Organization,
        DisplayName::new("Green Acres Farm").unwrap(),
        Email::new(email).unwrap(),
    )
}

#[sqlx::test(migrations = "../../migrations")]
async fn creates_and_finds_party_by_email(pool: PgPool) {
    let repo = PostgresPartyRepository::new(pool);
    let party = sample_party("repo@example.com");
    let email = party.email.clone();

    repo.create(&party).await.unwrap();

    let found = repo.find_by_email(&email).await.unwrap();
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.email, email);
    assert!(found.is_active);
}

#[sqlx::test(migrations = "../../migrations")]
async fn finds_party_by_id(pool: PgPool) {
    let repo = PostgresPartyRepository::new(pool);
    let party = sample_party("byid@example.com");
    let id = party.id;

    repo.create(&party).await.unwrap();

    let found = repo.find_by_id(id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn returns_none_for_missing_party(pool: PgPool) {
    let repo = PostgresPartyRepository::new(pool);
    let found = repo.find_by_id(Uuid::now_v7()).await.unwrap();
    assert!(found.is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn updates_party(pool: PgPool) {
    let repo = PostgresPartyRepository::new(pool);
    let party = sample_party("update@example.com");
    let id = party.id;

    repo.create(&party).await.unwrap();

    let mut updated = repo.find_by_id(id).await.unwrap().unwrap();
    updated.display_name = DisplayName::new("Updated Farm").unwrap();
    updated.phone = Some(domain::entities::Phone::new("+1-555-0123").unwrap());
    updated.location = Some(GeoPoint::new(37.0, -122.0).unwrap());
    updated.verification_status = VerificationStatus::Verified;

    repo.update(&updated).await.unwrap();

    let found = repo.find_by_id(id).await.unwrap().unwrap();
    assert_eq!(found.display_name.as_str(), "Updated Farm");
    assert_eq!(
        found.phone.as_ref().map(|p| p.as_str()),
        Some("+1-555-0123")
    );
    assert_eq!(found.verification_status, VerificationStatus::Verified);
}

#[sqlx::test(migrations = "../../migrations")]
async fn soft_deletes_party(pool: PgPool) {
    let repo = PostgresPartyRepository::new(pool);
    let party = sample_party("delete@example.com");
    let id = party.id;

    repo.create(&party).await.unwrap();
    repo.soft_delete(id).await.unwrap();

    let found = repo.find_by_id(id).await.unwrap().unwrap();
    assert!(!found.is_active);
}

#[sqlx::test(migrations = "../../migrations")]
async fn lists_and_counts_parties(pool: PgPool) {
    let repo = PostgresPartyRepository::new(pool);

    repo.create(&sample_party("a@example.com")).await.unwrap();
    repo.create(&sample_party("b@example.com")).await.unwrap();

    let criteria = PartySearchCriteria {
        limit: 10,
        offset: 0,
        ..Default::default()
    };
    let parties = repo.list(&criteria).await.unwrap();
    let total = repo.count(&criteria).await.unwrap();

    assert_eq!(parties.len(), 2);
    assert_eq!(total, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn search_filters_by_query(pool: PgPool) {
    let repo = PostgresPartyRepository::new(pool);

    let mut alpha = sample_party("alpha@example.com");
    alpha.display_name = DisplayName::new("Alpha Farm").unwrap();
    let mut beta = sample_party("beta@example.com");
    beta.display_name = DisplayName::new("Beta Farm").unwrap();

    repo.create(&alpha).await.unwrap();
    repo.create(&beta).await.unwrap();

    let criteria = PartySearchCriteria {
        query: Some("Alpha".to_string()),
        limit: 10,
        offset: 0,
        ..Default::default()
    };
    let parties = repo.list(&criteria).await.unwrap();
    let total = repo.count(&criteria).await.unwrap();

    assert_eq!(parties.len(), 1);
    assert_eq!(total, 1);
    assert_eq!(parties[0].display_name.as_str(), "Alpha Farm");
}

#[sqlx::test(migrations = "../../migrations")]
async fn manages_party_roles(pool: PgPool) {
    let repo = PostgresPartyRepository::new(pool);
    let party = sample_party("roles@example.com");
    let id = party.id;

    repo.create(&party).await.unwrap();

    repo.add_role(
        id,
        DealRole::Supplier,
        RoleProfile::for_role(DealRole::Supplier),
    )
    .await
    .unwrap();
    repo.add_role(
        id,
        DealRole::Consumer,
        RoleProfile::for_role(DealRole::Consumer),
    )
    .await
    .unwrap();

    let roles = repo.list_roles(id).await.unwrap();
    assert_eq!(roles.len(), 2);

    repo.remove_role(id, DealRole::Supplier).await.unwrap();

    let roles = repo.list_roles(id).await.unwrap();
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0].0, DealRole::Consumer);
}

#[sqlx::test(migrations = "../../migrations")]
async fn manages_memberships(pool: PgPool) {
    let user_repo = PostgresUserRepository::new(pool.clone());
    let repo = PostgresPartyRepository::new(pool);
    let party = sample_party("membership@example.com");
    let id = party.id;
    let user = User::new(
        Uuid::now_v7(),
        Email::new("member@example.com").unwrap(),
        Username::new("member").unwrap(),
        PasswordHash::new("hash".to_string()).unwrap(),
    );
    let user_id = user.id;

    user_repo.create(&user).await.unwrap();
    repo.create(&party).await.unwrap();

    let membership =
        UserPartyMembership::new(Uuid::now_v7(), user_id, id, PartyMembershipRole::Owner);
    repo.add_membership(&membership).await.unwrap();

    let found = repo.find_membership(user_id, id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().member_role, PartyMembershipRole::Owner);

    let memberships = repo.list_memberships_for_user(user_id).await.unwrap();
    assert_eq!(memberships.len(), 1);
    assert_eq!(memberships[0].1.id, id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn search_filters_by_radius(pool: PgPool) {
    let repo = PostgresPartyRepository::new(pool);

    let mut nearby = sample_party("nearby@example.com");
    nearby.location = Some(GeoPoint::new(37.7749, -122.4194).unwrap());
    nearby.service_radius_km = Some(10.0);

    let mut far = sample_party("far@example.com");
    far.location = Some(GeoPoint::new(38.0, -123.0).unwrap());
    far.service_radius_km = Some(500.0);

    repo.create(&nearby).await.unwrap();
    repo.create(&far).await.unwrap();

    // Search within 50 km of the nearby party.
    let criteria = PartySearchCriteria {
        latitude: Some(37.7749),
        longitude: Some(-122.4194),
        radius_km: Some(50.0),
        limit: 10,
        offset: 0,
        ..Default::default()
    };
    let parties = repo.list(&criteria).await.unwrap();
    let total = repo.count(&criteria).await.unwrap();

    assert_eq!(parties.len(), 1);
    assert_eq!(total, 1);
    assert_eq!(parties[0].id, nearby.id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn duplicate_email_is_rejected(pool: PgPool) {
    let repo = PostgresPartyRepository::new(pool);
    let first = sample_party("dup@example.com");
    let mut second = sample_party("other@example.com");
    second.email = first.email.clone();

    repo.create(&first).await.unwrap();
    let err = repo.create(&second).await.unwrap_err();

    assert!(matches!(
        err,
        domain::errors::DomainError::DuplicatePartyEmail
    ));
}
