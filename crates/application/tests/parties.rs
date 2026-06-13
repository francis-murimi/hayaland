use application::errors::ApplicationError;
use application::parties::dto::{
    AddPartyRoleCommand, CreatePartyCommand, SearchPartiesQuery, UpdatePartyCommand,
};
use application::parties::{
    AddPartyRole, CreateParty, GetParty, ListMyParties, ListPartyRoles, RemovePartyRole,
    SearchParties, SoftDeleteParty, UpdateParty,
};
use async_trait::async_trait;
use domain::entities::{
    DealRole, Email, PartyMembershipRole, PartyType, RoleProfile, UserPartyMembership,
    VerificationStatus,
};
use domain::repositories::{PartyRepository, PartySearchCriteria};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

fn owner_user_id() -> Uuid {
    Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap()
}

fn admin_user_id() -> Uuid {
    Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap()
}

fn member_user_id() -> Uuid {
    Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap()
}

#[derive(Default)]
struct FakePartyRepo {
    parties: Mutex<HashMap<Uuid, domain::entities::Party>>,
    memberships: Mutex<Vec<UserPartyMembership>>,
    roles: Mutex<Vec<(Uuid, DealRole, RoleProfile)>>,
}

#[async_trait]
impl PartyRepository for FakePartyRepo {
    async fn create(
        &self,
        party: &domain::entities::Party,
    ) -> Result<(), domain::errors::DomainError> {
        self.parties.lock().unwrap().insert(party.id, party.clone());
        Ok(())
    }

    async fn find_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<domain::entities::Party>, domain::errors::DomainError> {
        Ok(self.parties.lock().unwrap().get(&id).cloned())
    }

    async fn find_by_email(
        &self,
        email: &Email,
    ) -> Result<Option<domain::entities::Party>, domain::errors::DomainError> {
        Ok(self
            .parties
            .lock()
            .unwrap()
            .values()
            .find(|p| p.email == *email)
            .cloned())
    }

    async fn update(
        &self,
        party: &domain::entities::Party,
    ) -> Result<(), domain::errors::DomainError> {
        self.parties.lock().unwrap().insert(party.id, party.clone());
        Ok(())
    }

    async fn soft_delete(&self, id: Uuid) -> Result<(), domain::errors::DomainError> {
        if let Some(p) = self.parties.lock().unwrap().get_mut(&id) {
            p.is_active = false;
        }
        Ok(())
    }

    async fn list(
        &self,
        criteria: &PartySearchCriteria,
    ) -> Result<Vec<domain::entities::Party>, domain::errors::DomainError> {
        let mut parties: Vec<domain::entities::Party> =
            self.parties.lock().unwrap().values().cloned().collect();
        if let Some(q) = &criteria.query {
            let q = q.to_lowercase();
            parties.retain(|p| {
                p.display_name.as_str().to_lowercase().contains(&q)
                    || p.email.as_str().to_lowercase().contains(&q)
            });
        }
        if let Some(true) = criteria.active_only {
            parties.retain(|p| p.is_active);
        }
        if let Some(min) = criteria.min_trust_score {
            parties.retain(|p| p.trust_score >= min);
        }
        if let Some(max) = criteria.max_trust_score {
            parties.retain(|p| p.trust_score <= max);
        }
        Ok(parties)
    }

    async fn count(
        &self,
        criteria: &PartySearchCriteria,
    ) -> Result<i64, domain::errors::DomainError> {
        let mut parties: Vec<domain::entities::Party> =
            self.parties.lock().unwrap().values().cloned().collect();
        if let Some(q) = &criteria.query {
            let q = q.to_lowercase();
            parties.retain(|p| {
                p.display_name.as_str().to_lowercase().contains(&q)
                    || p.email.as_str().to_lowercase().contains(&q)
            });
        }
        if let Some(true) = criteria.active_only {
            parties.retain(|p| p.is_active);
        }
        if let Some(min) = criteria.min_trust_score {
            parties.retain(|p| p.trust_score >= min);
        }
        if let Some(max) = criteria.max_trust_score {
            parties.retain(|p| p.trust_score <= max);
        }
        Ok(parties.len() as i64)
    }

    async fn add_role(
        &self,
        party_id: Uuid,
        role: DealRole,
        profile: RoleProfile,
    ) -> Result<(), domain::errors::DomainError> {
        let mut roles = self.roles.lock().unwrap();
        if let Some(entry) = roles
            .iter_mut()
            .find(|(pid, r, _)| *pid == party_id && *r == role)
        {
            entry.2 = profile;
        } else {
            roles.push((party_id, role, profile));
        }
        Ok(())
    }

    async fn remove_role(
        &self,
        party_id: Uuid,
        role: DealRole,
    ) -> Result<(), domain::errors::DomainError> {
        self.roles
            .lock()
            .unwrap()
            .retain(|(pid, r, _)| !(*pid == party_id && *r == role));
        Ok(())
    }

    async fn list_roles(
        &self,
        party_id: Uuid,
    ) -> Result<Vec<(DealRole, RoleProfile)>, domain::errors::DomainError> {
        Ok(self
            .roles
            .lock()
            .unwrap()
            .iter()
            .filter(|(pid, _, _)| *pid == party_id)
            .map(|(_, r, p)| (*r, p.clone()))
            .collect())
    }

    async fn has_role(
        &self,
        party_id: Uuid,
        role: DealRole,
    ) -> Result<bool, domain::errors::DomainError> {
        Ok(self
            .roles
            .lock()
            .unwrap()
            .iter()
            .any(|(pid, r, _)| *pid == party_id && *r == role))
    }

    async fn count_active_deals_for_role(
        &self,
        _party_id: Uuid,
        _role: DealRole,
    ) -> Result<i64, domain::errors::DomainError> {
        Ok(0)
    }

    async fn count_active_deals(
        &self,
        _party_id: Uuid,
    ) -> Result<i64, domain::errors::DomainError> {
        Ok(0)
    }

    async fn add_membership(
        &self,
        membership: &UserPartyMembership,
    ) -> Result<(), domain::errors::DomainError> {
        self.memberships.lock().unwrap().push(membership.clone());
        Ok(())
    }

    async fn list_memberships_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<(UserPartyMembership, domain::entities::Party)>, domain::errors::DomainError>
    {
        let parties = self.parties.lock().unwrap();
        Ok(self
            .memberships
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.user_id == user_id)
            .map(|m| {
                (
                    m.clone(),
                    parties.get(&m.party_id).cloned().expect("party exists"),
                )
            })
            .collect())
    }

    async fn find_membership(
        &self,
        user_id: Uuid,
        party_id: Uuid,
    ) -> Result<Option<UserPartyMembership>, domain::errors::DomainError> {
        Ok(self
            .memberships
            .lock()
            .unwrap()
            .iter()
            .find(|m| m.user_id == user_id && m.party_id == party_id)
            .cloned())
    }

    async fn touch(
        &self,
        _id: Uuid,
        _updated_at: time::OffsetDateTime,
    ) -> Result<(), domain::errors::DomainError> {
        Ok(())
    }
}

fn sample_party_cmd(display_name: &str, email: &str) -> CreatePartyCommand {
    CreatePartyCommand {
        actor_user_id: owner_user_id(),
        party_type: PartyType::Organization,
        display_name: display_name.to_string(),
        email: email.to_string(),
        phone: None,
        tax_id: None,
        primary_domain_id: None,
        latitude: None,
        longitude: None,
        service_radius_km: None,
        roles: vec![],
    }
}

async fn add_owner_membership(repo: &FakePartyRepo, user_id: Uuid, party_id: Uuid) {
    let membership = UserPartyMembership::new(
        Uuid::now_v7(),
        user_id,
        party_id,
        PartyMembershipRole::Owner,
    );
    repo.add_membership(&membership).await.unwrap();
}

#[tokio::test]
async fn create_party_persists_and_returns_result() {
    let repo = Arc::new(FakePartyRepo::default());
    let use_case = CreateParty::new(repo.clone());

    let result = use_case
        .execute(sample_party_cmd("Test Farm", "farm@example.com"))
        .await
        .unwrap();

    assert_eq!(result.display_name, "Test Farm");
    assert_eq!(result.email, "farm@example.com");
    assert_eq!(result.party_type, PartyType::Organization);

    let found = repo
        .find_by_email(&Email::new("farm@example.com").unwrap())
        .await
        .unwrap();
    assert!(found.is_some());

    let memberships = repo
        .list_memberships_for_user(owner_user_id())
        .await
        .unwrap();
    assert_eq!(memberships.len(), 1);
    assert_eq!(memberships[0].0.member_role, PartyMembershipRole::Owner);
}

#[tokio::test]
async fn create_party_rejects_duplicate_email() {
    let repo = Arc::new(FakePartyRepo::default());
    let use_case = CreateParty::new(repo.clone());

    use_case
        .execute(sample_party_cmd("First", "dup@example.com"))
        .await
        .unwrap();
    let err = use_case
        .execute(sample_party_cmd("Second", "dup@example.com"))
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::DuplicatePartyEmail));
}

#[tokio::test]
async fn create_party_assigns_initial_roles() {
    let repo = Arc::new(FakePartyRepo::default());
    let use_case = CreateParty::new(repo.clone());

    let mut cmd = sample_party_cmd("Role Farm", "roles@example.com");
    cmd.roles = vec![DealRole::Supplier, DealRole::Consumer];

    let result = use_case.execute(cmd).await.unwrap();
    let roles = repo.list_roles(result.id).await.unwrap();
    let role_types: Vec<_> = roles.iter().map(|(r, _)| *r).collect();
    assert!(role_types.contains(&DealRole::Supplier));
    assert!(role_types.contains(&DealRole::Consumer));
}

#[tokio::test]
async fn get_party_returns_existing_party() {
    let repo = Arc::new(FakePartyRepo::default());
    let create = CreateParty::new(repo.clone());
    let get = GetParty::new(repo.clone());

    let created = create
        .execute(sample_party_cmd("Gettable", "get@example.com"))
        .await
        .unwrap();
    let found = get.execute(created.id).await.unwrap();

    assert_eq!(found.id, created.id);
}

#[tokio::test]
async fn get_party_returns_not_found_for_missing() {
    let repo = Arc::new(FakePartyRepo::default());
    let get = GetParty::new(repo);

    let err = get.execute(Uuid::now_v7()).await.unwrap_err();
    assert!(matches!(err, ApplicationError::PartyNotFound));
}

#[tokio::test]
async fn update_party_by_owner_succeeds() {
    let repo = Arc::new(FakePartyRepo::default());
    let create = CreateParty::new(repo.clone());
    let update = UpdateParty::new(repo.clone());

    let created = create
        .execute(sample_party_cmd("Before", "before@example.com"))
        .await
        .unwrap();
    add_owner_membership(&repo, owner_user_id(), created.id).await;

    let result = update
        .execute(
            created.id,
            UpdatePartyCommand {
                actor_user_id: owner_user_id(),
                is_admin: false,
                display_name: Some("After".to_string()),
                email: None,
                phone: Some("+1-555-0000".to_string()),
                tax_id: None,
                primary_domain_id: None,
                latitude: Some(12.0),
                longitude: Some(34.0),
                service_radius_km: Some(50.0),
                verification_status: None,
                is_active: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(result.display_name, "After");
    assert_eq!(result.phone, Some("+1-555-0000".to_string()));
    assert_eq!(result.latitude, Some(12.0));
}

#[tokio::test]
async fn update_party_forbidden_for_non_member() {
    let repo = Arc::new(FakePartyRepo::default());
    let create = CreateParty::new(repo.clone());
    let update = UpdateParty::new(repo);

    let created = create
        .execute(sample_party_cmd("Locked", "locked@example.com"))
        .await
        .unwrap();

    let err = update
        .execute(
            created.id,
            UpdatePartyCommand {
                actor_user_id: member_user_id(),
                is_admin: false,
                display_name: Some("Hacker".to_string()),
                email: None,
                phone: None,
                tax_id: None,
                primary_domain_id: None,
                latitude: None,
                longitude: None,
                service_radius_km: None,
                verification_status: None,
                is_active: None,
            },
        )
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::Forbidden));
}

#[tokio::test]
async fn update_party_admin_can_set_status() {
    let repo = Arc::new(FakePartyRepo::default());
    let create = CreateParty::new(repo.clone());
    let update = UpdateParty::new(repo);

    let created = create
        .execute(sample_party_cmd("Verify Me", "verify@example.com"))
        .await
        .unwrap();

    let result = update
        .execute(
            created.id,
            UpdatePartyCommand {
                actor_user_id: admin_user_id(),
                is_admin: true,
                display_name: None,
                email: None,
                phone: None,
                tax_id: None,
                primary_domain_id: None,
                latitude: None,
                longitude: None,
                service_radius_km: None,
                verification_status: Some(VerificationStatus::Verified),
                is_active: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(result.verification_status, VerificationStatus::Verified);
}

#[tokio::test]
async fn soft_delete_party_by_owner_succeeds() {
    let repo = Arc::new(FakePartyRepo::default());
    let create = CreateParty::new(repo.clone());
    let delete = SoftDeleteParty::new(repo.clone());

    let created = create
        .execute(sample_party_cmd("Deletable", "delete@example.com"))
        .await
        .unwrap();
    add_owner_membership(&repo, owner_user_id(), created.id).await;

    delete
        .execute(created.id, owner_user_id(), false)
        .await
        .unwrap();
    let found = repo.find_by_id(created.id).await.unwrap().unwrap();
    assert!(!found.is_active);
}

#[tokio::test]
async fn soft_delete_party_forbidden_for_member() {
    let repo = Arc::new(FakePartyRepo::default());
    let create = CreateParty::new(repo.clone());
    let delete = SoftDeleteParty::new(repo);

    let created = create
        .execute(sample_party_cmd("Protected", "protected@example.com"))
        .await
        .unwrap();

    let err = delete
        .execute(created.id, member_user_id(), false)
        .await
        .unwrap_err();
    assert!(matches!(err, ApplicationError::Forbidden));
}

#[tokio::test]
async fn list_my_parties_returns_only_owned_parties() {
    let repo = Arc::new(FakePartyRepo::default());
    let create = CreateParty::new(repo.clone());
    let list = ListMyParties::new(repo.clone());

    create
        .execute(sample_party_cmd("First", "first@example.com"))
        .await
        .unwrap();
    create
        .execute(sample_party_cmd("Second", "second@example.com"))
        .await
        .unwrap();

    let parties = list.execute(owner_user_id()).await.unwrap();
    assert_eq!(parties.len(), 2);
}

#[tokio::test]
async fn search_parties_returns_all_matching() {
    let repo = Arc::new(FakePartyRepo::default());
    let create = CreateParty::new(repo.clone());
    let search = SearchParties::new(repo);

    create
        .execute(sample_party_cmd("Alpha", "alpha@example.com"))
        .await
        .unwrap();
    create
        .execute(sample_party_cmd("Beta", "beta@example.com"))
        .await
        .unwrap();

    let result = search
        .execute(SearchPartiesQuery {
            query: Some("Alpha".to_string()),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(result.parties.len(), 1);
    assert_eq!(result.total, 1);
}

#[tokio::test]
async fn add_party_role_by_owner_succeeds() {
    let repo = Arc::new(FakePartyRepo::default());
    let create = CreateParty::new(repo.clone());
    let add_role = AddPartyRole::new(repo.clone());

    let created = create
        .execute(sample_party_cmd("Role Add", "roleadd@example.com"))
        .await
        .unwrap();
    add_owner_membership(&repo, owner_user_id(), created.id).await;

    add_role
        .execute(
            created.id,
            AddPartyRoleCommand {
                actor_user_id: owner_user_id(),
                is_admin: false,
                role: DealRole::Enhancer,
                profile: RoleProfile::for_role(DealRole::Enhancer),
            },
        )
        .await
        .unwrap();

    let roles = repo.list_roles(created.id).await.unwrap();
    assert!(roles.iter().any(|(r, _)| *r == DealRole::Enhancer));
}

#[tokio::test]
async fn remove_party_role_by_owner_succeeds() {
    let repo = Arc::new(FakePartyRepo::default());
    let create = CreateParty::new(repo.clone());
    let add_role = AddPartyRole::new(repo.clone());
    let remove_role = RemovePartyRole::new(repo.clone());

    let created = create
        .execute(sample_party_cmd("Role Remove", "roleremove@example.com"))
        .await
        .unwrap();
    add_owner_membership(&repo, owner_user_id(), created.id).await;

    add_role
        .execute(
            created.id,
            AddPartyRoleCommand {
                actor_user_id: owner_user_id(),
                is_admin: false,
                role: DealRole::Supplier,
                profile: RoleProfile::for_role(DealRole::Supplier),
            },
        )
        .await
        .unwrap();

    remove_role
        .execute(created.id, DealRole::Supplier, owner_user_id(), false)
        .await
        .unwrap();

    let roles = repo.list_roles(created.id).await.unwrap();
    assert!(roles.is_empty());
}

#[tokio::test]
async fn list_party_roles_returns_roles() {
    let repo = Arc::new(FakePartyRepo::default());
    let create = CreateParty::new(repo.clone());
    let list_roles = ListPartyRoles::new(repo.clone());

    let created = create
        .execute(sample_party_cmd("Role List", "rolelist@example.com"))
        .await
        .unwrap();
    repo.add_role(
        created.id,
        DealRole::Consumer,
        RoleProfile::for_role(DealRole::Consumer),
    )
    .await
    .unwrap();

    let roles = list_roles.execute(created.id).await.unwrap();
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0].role_type, DealRole::Consumer);
}
