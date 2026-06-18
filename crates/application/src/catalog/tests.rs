use crate::catalog::dto::{
    AdminUpdateFlagsCommand, BindCatalogItemToDealCommand, CatalogSearchQuery,
    ContactCatalogOwnerCommand, CreateEnhancementCommand, CreateNeedCommand, CreateResourceCommand,
    DeleteCatalogItemCommand, UpdateEnhancementCommand, UpdateNeedCommand,
    UpdatePartyCatalogSettingsCommand, UpdateResourceCommand,
};
use crate::catalog::{
    AdminUpdateCatalogFlags, BindCatalogItemToDeal, ContactCatalogOwner, CreateEnhancement,
    CreateNeed, CreateResource, DeleteNeed, DeleteResource, GetResource, ListDealCatalogItems,
    ListResources, UpdateEnhancement, UpdateNeed, UpdatePartyCatalogSettings, UpdateResource,
};
use crate::errors::ApplicationError;
use crate::test_helpers::{
    test_enhancement, test_need, test_resource, FakeCatalogRepository, FakeDealRepo,
    FakeMessageRepo, FakePartyRepo,
};
use domain::entities::{
    Deal, DealParticipation, DealRole, DealTitle, DisplayName, Email, Party, PartyMembershipRole,
    PartyType, RoleProfile, UserPartyMembership,
};
use domain::repositories::{CatalogRepository, MessageRepository};
use rust_decimal::Decimal;
use std::sync::Arc;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn party_repo_with_member(role: DealRole) -> (Arc<FakePartyRepo>, Uuid, Uuid) {
    let repo = Arc::new(FakePartyRepo::default());
    let user_id = Uuid::now_v7();
    let party_id = Uuid::now_v7();
    let party = Party::new(
        party_id,
        PartyType::Organization,
        DisplayName::new("Test Party").unwrap(),
        Email::new("party@example.com").unwrap(),
    );
    repo.parties.lock().unwrap().insert(party_id, party);
    repo.roles
        .lock()
        .unwrap()
        .push((party_id, role, RoleProfile::for_role(role)));
    let membership = UserPartyMembership::new(
        Uuid::now_v7(),
        user_id,
        party_id,
        PartyMembershipRole::Owner,
    );
    repo.memberships.lock().unwrap().push(membership);
    (repo, user_id, party_id)
}

fn make_draft_deal(
    supplier_party_id: Uuid,
    consumer_party_id: Uuid,
    enhancer_party_id: Uuid,
) -> (Deal, Vec<DealParticipation>) {
    let deal_id = Uuid::now_v7();
    let deal = Deal::new(
        deal_id,
        "DL-2026-0001".to_string(),
        DealTitle::new("Test Deal").unwrap(),
        Uuid::now_v7(),
        supplier_party_id,
        DealRole::Supplier,
    );
    let participations = vec![
        DealParticipation::new(
            Uuid::now_v7(),
            deal_id,
            supplier_party_id,
            DealRole::Supplier,
            true,
        ),
        DealParticipation::new(
            Uuid::now_v7(),
            deal_id,
            consumer_party_id,
            DealRole::Consumer,
            false,
        ),
        DealParticipation::new(
            Uuid::now_v7(),
            deal_id,
            enhancer_party_id,
            DealRole::Enhancer,
            false,
        ),
    ];
    (deal, participations)
}

// ---------------------------------------------------------------------------
// Resource tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn create_resource_succeeds_with_supplier_role() {
    let (party_repo, user_id, party_id) = party_repo_with_member(DealRole::Supplier);
    let catalog_repo = Arc::new(FakeCatalogRepository::new());
    let use_case = CreateResource::new(catalog_repo.clone(), party_repo);

    let cmd = CreateResourceCommand {
        actor_user_id: user_id,
        actor_party_id: party_id,
        is_admin: false,
        resource_type_id: Uuid::now_v7(),
        resource_name: "Test Resource".to_string(),
        description: Some("A test resource".to_string()),
        quantity: Decimal::from(10),
        quantity_unit: "kg".to_string(),
        condition: None,
        latitude: None,
        longitude: None,
        location_address: None,
        availability_start: None,
        availability_end: None,
        document_urls: vec![],
        opportunity_cost: None,
        metadata: None,
    };

    let result = use_case.execute(cmd).await.unwrap();
    assert_eq!(result.resource_name, "Test Resource");
    assert_eq!(result.supplier_party_id, party_id);
    assert!(result.is_active);
}

#[tokio::test]
async fn create_resource_fails_without_supplier_role() {
    let (party_repo, user_id, party_id) = party_repo_with_member(DealRole::Consumer);
    let catalog_repo = Arc::new(FakeCatalogRepository::new());
    let use_case = CreateResource::new(catalog_repo, party_repo);

    let cmd = CreateResourceCommand {
        actor_user_id: user_id,
        actor_party_id: party_id,
        is_admin: false,
        resource_type_id: Uuid::now_v7(),
        resource_name: "Test Resource".to_string(),
        description: None,
        quantity: Decimal::from(1),
        quantity_unit: "kg".to_string(),
        condition: None,
        latitude: None,
        longitude: None,
        location_address: None,
        availability_start: None,
        availability_end: None,
        document_urls: vec![],
        opportunity_cost: None,
        metadata: None,
    };

    let err = use_case.execute(cmd).await.unwrap_err();
    assert!(matches!(err, ApplicationError::Validation(_)));
}

#[tokio::test]
async fn get_resource_returns_public_view_for_anonymous() {
    let (_party_repo, _user_id, party_id) = party_repo_with_member(DealRole::Supplier);
    let catalog_repo = Arc::new(FakeCatalogRepository::new());
    let resource = test_resource(party_id, Uuid::now_v7(), "Public Resource");
    catalog_repo.create_resource(&resource).await.unwrap();

    let use_case = GetResource::new(catalog_repo);
    let view = use_case.execute(resource.id, None, false).await.unwrap();
    assert!(matches!(view, crate::catalog::ResourceView::Public(_)));
}

#[tokio::test]
async fn update_resource_succeeds_for_owner() {
    let (party_repo, user_id, party_id) = party_repo_with_member(DealRole::Supplier);
    let catalog_repo = Arc::new(FakeCatalogRepository::new());
    let resource = test_resource(party_id, Uuid::now_v7(), "Resource to update");
    catalog_repo.create_resource(&resource).await.unwrap();

    let use_case = UpdateResource::new(catalog_repo, party_repo);
    let cmd = UpdateResourceCommand {
        actor_user_id: user_id,
        actor_party_id: party_id,
        is_admin: false,
        resource_type_id: None,
        resource_name: Some("Updated Name".to_string()),
        description: None,
        quantity: None,
        quantity_unit: None,
        condition: None,
        latitude: None,
        longitude: None,
        location_address: None,
        availability_start: None,
        availability_end: None,
        document_urls: None,
        opportunity_cost: None,
        metadata: None,
        is_active: None,
    };

    let result = use_case.execute(resource.id, cmd).await.unwrap();
    assert_eq!(result.resource_name, "Updated Name");
}

#[tokio::test]
async fn update_resource_fails_for_non_owner() {
    let (party_repo, _owner_user, owner_party) = party_repo_with_member(DealRole::Supplier);
    let (_, other_user, other_party) = party_repo_with_member(DealRole::Supplier);
    let catalog_repo = Arc::new(FakeCatalogRepository::new());
    let resource = test_resource(owner_party, Uuid::now_v7(), "Protected Resource");
    catalog_repo.create_resource(&resource).await.unwrap();

    // Merge the second party into the first repo so both are addressable.
    {
        let mut parties = party_repo.parties.lock().unwrap();
        let other = Party::new(
            other_party,
            PartyType::Organization,
            DisplayName::new("Other").unwrap(),
            Email::new("other@example.com").unwrap(),
        );
        parties.insert(other_party, other);
        drop(parties);
        party_repo
            .memberships
            .lock()
            .unwrap()
            .push(UserPartyMembership::new(
                Uuid::now_v7(),
                other_user,
                other_party,
                PartyMembershipRole::Owner,
            ));
    }

    let use_case = UpdateResource::new(catalog_repo, party_repo);
    let cmd = UpdateResourceCommand {
        actor_user_id: other_user,
        actor_party_id: other_party,
        is_admin: false,
        resource_type_id: None,
        resource_name: Some("Hacked".to_string()),
        description: None,
        quantity: None,
        quantity_unit: None,
        condition: None,
        latitude: None,
        longitude: None,
        location_address: None,
        availability_start: None,
        availability_end: None,
        document_urls: None,
        opportunity_cost: None,
        metadata: None,
        is_active: None,
    };

    let err = use_case.execute(resource.id, cmd).await.unwrap_err();
    assert!(matches!(err, ApplicationError::CatalogAccessDenied));
}

#[tokio::test]
async fn delete_resource_succeeds_when_no_active_deals() {
    let (party_repo, user_id, party_id) = party_repo_with_member(DealRole::Supplier);
    let catalog_repo = Arc::new(FakeCatalogRepository::new());
    let resource = test_resource(party_id, Uuid::now_v7(), "Deletable Resource");
    catalog_repo.create_resource(&resource).await.unwrap();

    let use_case = DeleteResource::new(catalog_repo, party_repo);
    let cmd = DeleteCatalogItemCommand {
        actor_user_id: user_id,
        actor_party_id: party_id,
        is_admin: false,
    };

    use_case.execute(resource.id, cmd).await.unwrap();
}

#[tokio::test]
async fn delete_resource_fails_when_active_deals() {
    let (party_repo, user_id, party_id) = party_repo_with_member(DealRole::Supplier);
    let catalog_repo = Arc::new(FakeCatalogRepository::new());
    let mut resource = test_resource(party_id, Uuid::now_v7(), "Locked Resource");
    resource.deal_count = 1;
    catalog_repo.create_resource(&resource).await.unwrap();

    let use_case = DeleteResource::new(catalog_repo, party_repo);
    let cmd = DeleteCatalogItemCommand {
        actor_user_id: user_id,
        actor_party_id: party_id,
        is_admin: false,
    };

    let err = use_case.execute(resource.id, cmd).await.unwrap_err();
    assert!(matches!(err, ApplicationError::CatalogItemHasActiveDeals));
}

#[tokio::test]
async fn list_resources_filters_by_text() {
    let (_party_repo, _user_id, party_id) = party_repo_with_member(DealRole::Supplier);
    let catalog_repo = Arc::new(FakeCatalogRepository::new());
    let r1 = test_resource(party_id, Uuid::now_v7(), "Apple Crates");
    let r2 = test_resource(party_id, Uuid::now_v7(), "Banana Boxes");
    catalog_repo.create_resource(&r1).await.unwrap();
    catalog_repo.create_resource(&r2).await.unwrap();

    let use_case = ListResources::new(catalog_repo);
    let query = CatalogSearchQuery {
        text: Some("Apple".to_string()),
        ..Default::default()
    };
    let result = use_case.execute(query, None, false).await.unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].resource_name, "Apple Crates");
}

// ---------------------------------------------------------------------------
// Need tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn create_need_succeeds_with_consumer_role() {
    let (party_repo, user_id, party_id) = party_repo_with_member(DealRole::Consumer);
    let catalog_repo = Arc::new(FakeCatalogRepository::new());
    let use_case = CreateNeed::new(catalog_repo, party_repo);

    let cmd = CreateNeedCommand {
        actor_user_id: user_id,
        actor_party_id: party_id,
        is_admin: false,
        need_category_id: Uuid::now_v7(),
        need_description: "I need organic apples for my store.".to_string(),
        required_quantity: Decimal::from(50),
        quantity_unit: "kg".to_string(),
        quality_requirements: None,
        required_by_date: None,
        max_budget: None,
        budget_currency: None,
        estimated_fulfillment_value: None,
        acceptable_variants: None,
        priority: None,
        latitude: None,
        longitude: None,
        location_address: None,
        delivery_preferences: None,
        metadata: None,
    };

    let result = use_case.execute(cmd).await.unwrap();
    assert_eq!(result.consumer_party_id, party_id);
}

#[tokio::test]
async fn create_need_fails_without_consumer_role() {
    let (party_repo, user_id, party_id) = party_repo_with_member(DealRole::Supplier);
    let catalog_repo = Arc::new(FakeCatalogRepository::new());
    let use_case = CreateNeed::new(catalog_repo, party_repo);

    let cmd = CreateNeedCommand {
        actor_user_id: user_id,
        actor_party_id: party_id,
        is_admin: false,
        need_category_id: Uuid::now_v7(),
        need_description: "I need organic apples.".to_string(),
        required_quantity: Decimal::from(10),
        quantity_unit: "kg".to_string(),
        quality_requirements: None,
        required_by_date: None,
        max_budget: None,
        budget_currency: None,
        estimated_fulfillment_value: None,
        acceptable_variants: None,
        priority: None,
        latitude: None,
        longitude: None,
        location_address: None,
        delivery_preferences: None,
        metadata: None,
    };

    let err = use_case.execute(cmd).await.unwrap_err();
    assert!(matches!(err, ApplicationError::Validation(_)));
}

#[tokio::test]
async fn update_need_succeeds_for_owner() {
    let (party_repo, user_id, party_id) = party_repo_with_member(DealRole::Consumer);
    let catalog_repo = Arc::new(FakeCatalogRepository::new());
    let need = test_need(party_id, Uuid::now_v7(), "Need to update");
    catalog_repo.create_need(&need).await.unwrap();

    let use_case = UpdateNeed::new(catalog_repo, party_repo);
    let cmd = UpdateNeedCommand {
        actor_user_id: user_id,
        actor_party_id: party_id,
        is_admin: false,
        need_category_id: None,
        need_description: Some("Updated description for the need.".to_string()),
        required_quantity: None,
        quantity_unit: None,
        quality_requirements: None,
        required_by_date: None,
        max_budget: None,
        budget_currency: None,
        estimated_fulfillment_value: None,
        acceptable_variants: None,
        priority: None,
        latitude: None,
        longitude: None,
        location_address: None,
        delivery_preferences: None,
        metadata: None,
        is_active: None,
    };

    let result = use_case.execute(need.id, cmd).await.unwrap();
    assert_eq!(result.need_description, "Updated description for the need.");
}

#[tokio::test]
async fn delete_need_fails_when_active_deals() {
    let (party_repo, user_id, party_id) = party_repo_with_member(DealRole::Consumer);
    let catalog_repo = Arc::new(FakeCatalogRepository::new());
    let mut need = test_need(party_id, Uuid::now_v7(), "Locked Need");
    need.deal_count = 3;
    catalog_repo.create_need(&need).await.unwrap();

    let use_case = DeleteNeed::new(catalog_repo, party_repo);
    let cmd = DeleteCatalogItemCommand {
        actor_user_id: user_id,
        actor_party_id: party_id,
        is_admin: false,
    };

    let err = use_case.execute(need.id, cmd).await.unwrap_err();
    assert!(matches!(err, ApplicationError::CatalogItemHasActiveDeals));
}

// ---------------------------------------------------------------------------
// Enhancement tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn create_enhancement_succeeds_with_enhancer_role() {
    let (party_repo, user_id, party_id) = party_repo_with_member(DealRole::Enhancer);
    let catalog_repo = Arc::new(FakeCatalogRepository::new());
    let use_case = CreateEnhancement::new(catalog_repo, party_repo);

    let cmd = CreateEnhancementCommand {
        actor_user_id: user_id,
        actor_party_id: party_id,
        is_admin: false,
        enhancement_type_id: Uuid::now_v7(),
        enhancement_name: "Crop Monitoring".to_string(),
        description: None,
        input_quantity: None,
        quantity_unit: None,
        estimated_input_cost: None,
        service_duration_hours: None,
        estimated_completion_days: None,
        deliverables: None,
        prerequisites: None,
        skills: vec![],
        certifications: None,
        equipment: vec![],
        pricing: None,
        availability: None,
        service_area: None,
        metadata: None,
    };

    let result = use_case.execute(cmd).await.unwrap();
    assert_eq!(result.enhancer_party_id, party_id);
}

#[tokio::test]
async fn create_enhancement_fails_without_enhancer_role() {
    let (party_repo, user_id, party_id) = party_repo_with_member(DealRole::Supplier);
    let catalog_repo = Arc::new(FakeCatalogRepository::new());
    let use_case = CreateEnhancement::new(catalog_repo, party_repo);

    let cmd = CreateEnhancementCommand {
        actor_user_id: user_id,
        actor_party_id: party_id,
        is_admin: false,
        enhancement_type_id: Uuid::now_v7(),
        enhancement_name: "Crop Monitoring".to_string(),
        description: None,
        input_quantity: None,
        quantity_unit: None,
        estimated_input_cost: None,
        service_duration_hours: None,
        estimated_completion_days: None,
        deliverables: None,
        prerequisites: None,
        skills: vec![],
        certifications: None,
        equipment: vec![],
        pricing: None,
        availability: None,
        service_area: None,
        metadata: None,
    };

    let err = use_case.execute(cmd).await.unwrap_err();
    assert!(matches!(err, ApplicationError::Validation(_)));
}

#[tokio::test]
async fn update_enhancement_succeeds_for_owner() {
    let (party_repo, user_id, party_id) = party_repo_with_member(DealRole::Enhancer);
    let catalog_repo = Arc::new(FakeCatalogRepository::new());
    let enhancement = test_enhancement(party_id, Uuid::now_v7(), "Enhancement to update");
    catalog_repo.create_enhancement(&enhancement).await.unwrap();

    let use_case = UpdateEnhancement::new(catalog_repo, party_repo);
    let cmd = UpdateEnhancementCommand {
        actor_user_id: user_id,
        actor_party_id: party_id,
        is_admin: false,
        enhancement_type_id: None,
        enhancement_name: Some("Updated Enhancement".to_string()),
        description: None,
        input_quantity: None,
        quantity_unit: None,
        estimated_input_cost: None,
        service_duration_hours: None,
        estimated_completion_days: None,
        deliverables: None,
        prerequisites: None,
        skills: None,
        certifications: None,
        equipment: None,
        pricing: None,
        availability: None,
        service_area: None,
        metadata: None,
        is_active: None,
    };

    let result = use_case.execute(enhancement.id, cmd).await.unwrap();
    assert_eq!(result.enhancement_name, "Updated Enhancement");
}

// ---------------------------------------------------------------------------
// Admin flag tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_flag_update_flips_visibility() {
    let (_party_repo, user_id, party_id) = party_repo_with_member(DealRole::Supplier);
    let catalog_repo = Arc::new(FakeCatalogRepository::new());
    let resource = test_resource(party_id, Uuid::now_v7(), "Flagged Resource");
    catalog_repo.create_resource(&resource).await.unwrap();

    let use_case = AdminUpdateCatalogFlags::new(catalog_repo);
    let cmd = AdminUpdateFlagsCommand {
        actor_user_id: user_id,
        actor_party_id: party_id,
        is_admin: true,
        platform_hidden: Some(true),
        platform_featured: Some(true),
        admin_notes: Some("reviewed".to_string()),
    };

    let result = use_case.update_resource(resource.id, cmd).await.unwrap();
    assert!(result.platform_hidden);
    assert!(result.platform_featured);
    assert_eq!(result.admin_notes, Some("reviewed".to_string()));
}

#[tokio::test]
async fn admin_flag_update_denied_for_non_admin() {
    let (_party_repo, user_id, party_id) = party_repo_with_member(DealRole::Supplier);
    let catalog_repo = Arc::new(FakeCatalogRepository::new());
    let resource = test_resource(party_id, Uuid::now_v7(), "Protected Resource");
    catalog_repo.create_resource(&resource).await.unwrap();

    let use_case = AdminUpdateCatalogFlags::new(catalog_repo);
    let cmd = AdminUpdateFlagsCommand {
        actor_user_id: user_id,
        actor_party_id: party_id,
        is_admin: false,
        platform_hidden: Some(true),
        platform_featured: None,
        admin_notes: None,
    };

    let err = use_case
        .update_resource(resource.id, cmd)
        .await
        .unwrap_err();
    assert!(matches!(err, ApplicationError::CatalogAccessDenied));
}

// ---------------------------------------------------------------------------
// Contact owner tests
// ---------------------------------------------------------------------------

fn add_party_with_user(repo: &Arc<FakePartyRepo>, role: DealRole, email: &str) -> (Uuid, Uuid) {
    let user_id = Uuid::now_v7();
    let party_id = Uuid::now_v7();
    let party = Party::new(
        party_id,
        PartyType::Organization,
        DisplayName::new("Contact Party").unwrap(),
        Email::new(email).unwrap(),
    );
    repo.parties.lock().unwrap().insert(party_id, party);
    repo.roles
        .lock()
        .unwrap()
        .push((party_id, role, RoleProfile::for_role(role)));
    repo.memberships
        .lock()
        .unwrap()
        .push(UserPartyMembership::new(
            Uuid::now_v7(),
            user_id,
            party_id,
            PartyMembershipRole::Owner,
        ));
    (user_id, party_id)
}

#[tokio::test]
async fn contact_owner_succeeds_when_owner_accepts_inquiries() {
    let party_repo = Arc::new(FakePartyRepo::default());
    let (_supplier_user, supplier_party) =
        add_party_with_user(&party_repo, DealRole::Supplier, "supplier@example.com");
    let (buyer_user, buyer_party) =
        add_party_with_user(&party_repo, DealRole::Consumer, "buyer@example.com");

    let catalog_repo = Arc::new(FakeCatalogRepository::new());
    let message_repo = Arc::new(FakeMessageRepo::default());
    let resource = test_resource(supplier_party, Uuid::now_v7(), "Contactable Resource");
    catalog_repo.create_resource(&resource).await.unwrap();

    let use_case = ContactCatalogOwner::new(catalog_repo, party_repo, message_repo.clone());
    let cmd = ContactCatalogOwnerCommand {
        actor_user_id: buyer_user,
        actor_party_id: buyer_party,
        is_admin: false,
        item_type: "RESOURCE".to_string(),
        item_id: resource.id,
        message: "Is this still available?".to_string(),
    };

    let result = use_case.execute(cmd).await.unwrap();
    let conversation = message_repo
        .find_conversation_by_id(result.conversation_id)
        .await
        .unwrap();
    assert!(conversation.is_some());
    let message = message_repo
        .find_message_by_id(result.message_id)
        .await
        .unwrap();
    assert!(message.is_some());
}

#[tokio::test]
async fn contact_owner_fails_when_owner_opted_out() {
    let party_repo = Arc::new(FakePartyRepo::default());
    let (_supplier_user, supplier_party) =
        add_party_with_user(&party_repo, DealRole::Supplier, "supplier@example.com");
    let (buyer_user, buyer_party) =
        add_party_with_user(&party_repo, DealRole::Consumer, "buyer@example.com");
    {
        let mut parties = party_repo.parties.lock().unwrap();
        if let Some(p) = parties.get_mut(&supplier_party) {
            p.accepts_catalog_inquiries = false;
        }
    }
    let catalog_repo = Arc::new(FakeCatalogRepository::new());
    let message_repo = Arc::new(FakeMessageRepo::default());
    let resource = test_resource(supplier_party, Uuid::now_v7(), "Opted-out Resource");
    catalog_repo.create_resource(&resource).await.unwrap();

    let use_case = ContactCatalogOwner::new(catalog_repo, party_repo, message_repo);
    let cmd = ContactCatalogOwnerCommand {
        actor_user_id: buyer_user,
        actor_party_id: buyer_party,
        is_admin: false,
        item_type: "RESOURCE".to_string(),
        item_id: resource.id,
        message: "Is this still available?".to_string(),
    };

    let err = use_case.execute(cmd).await.unwrap_err();
    assert!(matches!(err, ApplicationError::Validation(_)));
}

// ---------------------------------------------------------------------------
// Party settings tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn update_party_catalog_settings_succeeds_for_owner() {
    let (party_repo, user_id, party_id) = party_repo_with_member(DealRole::Supplier);
    let use_case = UpdatePartyCatalogSettings::new(party_repo);
    let cmd = UpdatePartyCatalogSettingsCommand {
        actor_user_id: user_id,
        actor_party_id: party_id,
        is_admin: false,
        accepts_catalog_inquiries: Some(false),
        public_contact_email: Some(true),
    };

    let result = use_case.execute(party_id, cmd).await.unwrap();
    assert!(!result.accepts_catalog_inquiries);
    assert!(result.public_contact_email);
}

// ---------------------------------------------------------------------------
// Deal binding tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn bind_resource_to_deal_succeeds_for_supplier_participant() {
    let (party_repo, supplier_user, supplier_party) = party_repo_with_member(DealRole::Supplier);
    let consumer_party = Uuid::now_v7();
    let enhancer_party = Uuid::now_v7();

    {
        let mut parties = party_repo.parties.lock().unwrap();
        parties.insert(
            consumer_party,
            Party::new(
                consumer_party,
                PartyType::Organization,
                DisplayName::new("Consumer").unwrap(),
                Email::new("consumer@example.com").unwrap(),
            ),
        );
        parties.insert(
            enhancer_party,
            Party::new(
                enhancer_party,
                PartyType::Organization,
                DisplayName::new("Enhancer").unwrap(),
                Email::new("enhancer@example.com").unwrap(),
            ),
        );
    }

    let (deal, participations) = make_draft_deal(supplier_party, consumer_party, enhancer_party);
    let deal_repo = Arc::new(FakeDealRepo::default());
    deal_repo
        .deals
        .lock()
        .unwrap()
        .insert(deal.id, deal.clone());
    for p in &participations {
        deal_repo.participations.lock().unwrap().push(p.clone());
    }

    let catalog_repo = Arc::new(FakeCatalogRepository::new());
    let resource = test_resource(supplier_party, Uuid::now_v7(), "Bindable Resource");
    catalog_repo.create_resource(&resource).await.unwrap();

    let use_case = BindCatalogItemToDeal::new(catalog_repo.clone(), deal_repo, party_repo);
    let cmd = BindCatalogItemToDealCommand {
        actor_user_id: supplier_user,
        actor_party_id: supplier_party,
        is_admin: false,
        item_type: "RESOURCE".to_string(),
        item_id: resource.id,
        deal_id: deal.id,
        overrides: None,
    };

    let result = use_case.execute(cmd).await.unwrap();
    assert_eq!(result.deal_id, deal.id);
    assert_eq!(result.catalog_item_id, resource.id);

    let bound = catalog_repo.find_resources_by_deal(deal.id).await.unwrap();
    assert_eq!(bound.len(), 1);
    assert_eq!(bound[0].catalog_item_id, Some(resource.id));

    let original = catalog_repo
        .find_resource_by_id(resource.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(original.deal_count, 1);
}

#[tokio::test]
async fn list_deal_catalog_items_returns_bound_items() {
    let (party_repo, supplier_user, supplier_party) = party_repo_with_member(DealRole::Supplier);
    let consumer_party = Uuid::now_v7();
    let enhancer_party = Uuid::now_v7();

    {
        let mut parties = party_repo.parties.lock().unwrap();
        parties.insert(
            consumer_party,
            Party::new(
                consumer_party,
                PartyType::Organization,
                DisplayName::new("Consumer").unwrap(),
                Email::new("consumer@example.com").unwrap(),
            ),
        );
        parties.insert(
            enhancer_party,
            Party::new(
                enhancer_party,
                PartyType::Organization,
                DisplayName::new("Enhancer").unwrap(),
                Email::new("enhancer@example.com").unwrap(),
            ),
        );
    }

    let (deal, participations) = make_draft_deal(supplier_party, consumer_party, enhancer_party);
    let deal_repo = Arc::new(FakeDealRepo::default());
    deal_repo
        .deals
        .lock()
        .unwrap()
        .insert(deal.id, deal.clone());
    for p in &participations {
        deal_repo.participations.lock().unwrap().push(p.clone());
    }

    let catalog_repo = Arc::new(FakeCatalogRepository::new());
    let resource = test_resource(supplier_party, Uuid::now_v7(), "Bound Resource");
    catalog_repo.create_resource(&resource).await.unwrap();

    let binder = BindCatalogItemToDeal::new(catalog_repo.clone(), deal_repo, party_repo);
    binder
        .execute(BindCatalogItemToDealCommand {
            actor_user_id: supplier_user,
            actor_party_id: supplier_party,
            is_admin: false,
            item_type: "RESOURCE".to_string(),
            item_id: resource.id,
            deal_id: deal.id,
            overrides: None,
        })
        .await
        .unwrap();

    let lister = ListDealCatalogItems::new(catalog_repo);
    let result = lister.list_resources(deal.id).await.unwrap();
    assert_eq!(result.items.len(), 1);
}
