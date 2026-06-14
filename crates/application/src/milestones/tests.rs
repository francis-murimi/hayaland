use crate::milestones::dto::{
    CreateMilestoneCommand, GetDealProgressQuery, ListMilestonesQuery, MilestoneActionCommand,
    UpdateMilestoneCommand,
};
use crate::milestones::{
    CompleteMilestone, CreateMilestone, DeleteMilestone, GetDealProgress, ListMilestones,
    StartMilestone, UpdateMilestone, VerifyMilestone,
};
use crate::test_helpers::{FakeDealRepo, FakePartyRepo, FakeWalletRepo};
use async_trait::async_trait;
use domain::entities::{
    Currency, Deal, DealParticipation, DealRole, DealStatus, DealTitle, Milestone, MilestoneStatus,
    Party, PlatformWallet, UserPartyMembership,
};
use domain::errors::DomainError;
use domain::repositories::MilestoneRepository;
use rust_decimal::Decimal;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Default)]
struct FakeMilestoneRepo {
    milestones: Mutex<Vec<Milestone>>,
}

#[async_trait]
impl MilestoneRepository for FakeMilestoneRepo {
    async fn create(&self, milestone: &Milestone) -> Result<(), DomainError> {
        self.milestones.lock().unwrap().push(milestone.clone());
        Ok(())
    }

    async fn update(&self, milestone: &Milestone) -> Result<(), DomainError> {
        let mut milestones = self.milestones.lock().unwrap();
        let idx = milestones
            .iter()
            .position(|m| m.id == milestone.id)
            .ok_or(DomainError::MilestoneNotFound)?;
        milestones[idx] = milestone.clone();
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let mut milestones = self.milestones.lock().unwrap();
        let idx = milestones
            .iter()
            .position(|m| m.id == id)
            .ok_or(DomainError::MilestoneNotFound)?;
        milestones.remove(idx);
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Milestone>, DomainError> {
        Ok(self
            .milestones
            .lock()
            .unwrap()
            .iter()
            .find(|m| m.id == id)
            .cloned())
    }

    async fn find_by_deal(
        &self,
        deal_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Milestone>, DomainError> {
        let milestones = self.milestones.lock().unwrap();
        let mut matched: Vec<Milestone> = milestones
            .iter()
            .filter(|m| m.deal_id == deal_id)
            .cloned()
            .collect();
        matched.sort_by(|a, b| {
            a.display_order
                .cmp(&b.display_order)
                .then_with(|| a.created_at.cmp(&b.created_at))
        });
        let start = offset as usize;
        let end = (offset + limit) as usize;
        Ok(matched
            .into_iter()
            .skip(start)
            .take(end.saturating_sub(start))
            .collect())
    }

    async fn count_by_deal(&self, deal_id: Uuid) -> Result<i64, DomainError> {
        Ok(self
            .milestones
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.deal_id == deal_id)
            .count() as i64)
    }

    async fn count_verified_by_deal(&self, deal_id: Uuid) -> Result<i64, DomainError> {
        Ok(self
            .milestones
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.deal_id == deal_id && m.milestone_status == MilestoneStatus::Verified)
            .count() as i64)
    }

    async fn count_by_status(&self, deal_id: Uuid, status: &str) -> Result<i64, DomainError> {
        let target = MilestoneStatus::try_from(status)?;
        Ok(self
            .milestones
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.deal_id == deal_id && m.milestone_status == target)
            .count() as i64)
    }
}

fn test_party(id: Uuid, email: &str) -> Party {
    Party::new(
        id,
        domain::entities::PartyType::Organization,
        domain::entities::DisplayName::new("Test Party").unwrap(),
        domain::entities::Email::new(email).unwrap(),
    )
}

fn membership(user_id: Uuid, party_id: Uuid) -> UserPartyMembership {
    UserPartyMembership::new(
        Uuid::now_v7(),
        user_id,
        party_id,
        domain::entities::PartyMembershipRole::Member,
    )
}

fn setup() -> (
    Arc<FakePartyRepo>,
    Arc<FakeDealRepo>,
    Arc<FakeMilestoneRepo>,
    Arc<FakeWalletRepo>,
    Uuid,
    Uuid,
    Uuid,
    Uuid,
    Uuid,
) {
    let user_id = Uuid::now_v7();
    let supplier_id = Uuid::now_v7();
    let consumer_id = Uuid::now_v7();
    let enhancer_id = Uuid::now_v7();
    let deal_id = Uuid::now_v7();

    let party_repo: Arc<FakePartyRepo> = Arc::new(FakePartyRepo {
        parties: Mutex::new(
            [
                (supplier_id, test_party(supplier_id, "supplier@example.com")),
                (consumer_id, test_party(consumer_id, "consumer@example.com")),
                (enhancer_id, test_party(enhancer_id, "enhancer@example.com")),
            ]
            .into_iter()
            .collect(),
        ),
        memberships: Mutex::new(vec![
            membership(user_id, supplier_id),
            membership(user_id, consumer_id),
            membership(user_id, enhancer_id),
        ]),
        roles: Mutex::new(vec![]),
    });

    let mut deal = Deal::new(
        deal_id,
        "DL-TEST-0001".to_string(),
        DealTitle::new("Test Deal").unwrap(),
        Uuid::now_v7(),
        supplier_id,
        DealRole::Supplier,
    );
    deal.deal_status = DealStatus::Committed;

    let deal_repo = FakeDealRepo::default();
    deal_repo.deals.lock().unwrap().insert(deal_id, deal);
    for (party_id, role) in [
        (supplier_id, DealRole::Supplier),
        (consumer_id, DealRole::Consumer),
        (enhancer_id, DealRole::Enhancer),
    ] {
        deal_repo
            .participations
            .lock()
            .unwrap()
            .push(DealParticipation::new(
                Uuid::now_v7(),
                deal_id,
                party_id,
                role,
                party_id == supplier_id,
            ));
    }
    let deal_repo: Arc<FakeDealRepo> = Arc::new(deal_repo);
    let milestone_repo: Arc<FakeMilestoneRepo> = Arc::new(FakeMilestoneRepo::default());
    let wallet_repo: Arc<FakeWalletRepo> = Arc::new(FakeWalletRepo::default());

    (
        party_repo,
        deal_repo,
        milestone_repo,
        wallet_repo,
        user_id,
        supplier_id,
        consumer_id,
        enhancer_id,
        deal_id,
    )
}

#[tokio::test]
async fn create_milestone_succeeds_for_participant() {
    let (
        party_repo,
        deal_repo,
        milestone_repo,
        _,
        user_id,
        supplier_id,
        consumer_id,
        enhancer_id,
        deal_id,
    ) = setup();

    let use_case = CreateMilestone::new(party_repo, deal_repo, milestone_repo.clone());
    let result = use_case
        .execute(CreateMilestoneCommand {
            actor_user_id: user_id,
            actor_party_id: supplier_id,
            deal_id,
            milestone_name: "Build prototype".to_string(),
            description: None,
            assigned_to_party_id: consumer_id,
            verified_by_party_id: enhancer_id,
            due_date: None,
            completion_criteria: "Prototype delivered".to_string(),
            payment_trigger_amount: Some(Decimal::from(100)),
            display_order: 1,
        })
        .await
        .unwrap();

    assert_eq!(result.deal_id, deal_id);
    assert_eq!(result.milestone_status, "PENDING");
    assert_eq!(milestone_repo.count_by_deal(deal_id).await.unwrap(), 1);
}

#[tokio::test]
async fn create_milestone_fails_when_deal_not_committed() {
    let (
        party_repo,
        deal_repo,
        milestone_repo,
        _,
        user_id,
        supplier_id,
        consumer_id,
        enhancer_id,
        deal_id,
    ) = setup();
    deal_repo
        .deals
        .lock()
        .unwrap()
        .get_mut(&deal_id)
        .unwrap()
        .deal_status = DealStatus::Draft;

    let use_case = CreateMilestone::new(party_repo, deal_repo, milestone_repo);
    let result = use_case
        .execute(CreateMilestoneCommand {
            actor_user_id: user_id,
            actor_party_id: supplier_id,
            deal_id,
            milestone_name: "Build prototype".to_string(),
            description: None,
            assigned_to_party_id: consumer_id,
            verified_by_party_id: enhancer_id,
            due_date: None,
            completion_criteria: "Prototype delivered".to_string(),
            payment_trigger_amount: None,
            display_order: 1,
        })
        .await;

    assert!(matches!(
        result,
        Err(crate::errors::ApplicationError::Validation(_))
    ));
}

#[tokio::test]
async fn list_milestones_and_progress() {
    let (
        party_repo,
        deal_repo,
        milestone_repo,
        _,
        user_id,
        supplier_id,
        consumer_id,
        enhancer_id,
        deal_id,
    ) = setup();

    let create = CreateMilestone::new(
        party_repo.clone(),
        deal_repo.clone(),
        milestone_repo.clone(),
    );
    create
        .execute(CreateMilestoneCommand {
            actor_user_id: user_id,
            actor_party_id: supplier_id,
            deal_id,
            milestone_name: "First".to_string(),
            description: None,
            assigned_to_party_id: consumer_id,
            verified_by_party_id: enhancer_id,
            due_date: None,
            completion_criteria: "done".to_string(),
            payment_trigger_amount: None,
            display_order: 1,
        })
        .await
        .unwrap();
    create
        .execute(CreateMilestoneCommand {
            actor_user_id: user_id,
            actor_party_id: supplier_id,
            deal_id,
            milestone_name: "Second".to_string(),
            description: None,
            assigned_to_party_id: enhancer_id,
            verified_by_party_id: consumer_id,
            due_date: None,
            completion_criteria: "done".to_string(),
            payment_trigger_amount: None,
            display_order: 2,
        })
        .await
        .unwrap();

    let list = ListMilestones::new(
        party_repo.clone(),
        deal_repo.clone(),
        milestone_repo.clone(),
    );
    let result = list
        .execute(ListMilestonesQuery {
            actor_user_id: user_id,
            actor_party_id: supplier_id,
            deal_id,
            limit: Some(10),
            offset: Some(0),
        })
        .await
        .unwrap();
    assert_eq!(result.total, 2);
    assert_eq!(result.milestones.len(), 2);

    let progress = GetDealProgress::new(party_repo, deal_repo, milestone_repo);
    let progress = progress
        .execute(GetDealProgressQuery {
            actor_user_id: user_id,
            actor_party_id: supplier_id,
            deal_id,
        })
        .await
        .unwrap();
    assert_eq!(progress.total_milestones, 2);
    assert_eq!(progress.verified_milestones, 0);
    assert_eq!(progress.currency, Currency::Points);
}

#[tokio::test]
async fn full_milestone_lifecycle_triggers_transaction() {
    let (
        party_repo,
        deal_repo,
        milestone_repo,
        wallet_repo,
        user_id,
        supplier_id,
        consumer_id,
        enhancer_id,
        deal_id,
    ) = setup();

    let mut consumer_wallet = PlatformWallet::new(Uuid::now_v7(), consumer_id);
    consumer_wallet.deposit(Decimal::from(500)).unwrap();
    consumer_wallet.hold_escrow(Decimal::from(300)).unwrap();
    wallet_repo
        .wallets
        .lock()
        .unwrap()
        .insert(consumer_id, consumer_wallet);

    let create = CreateMilestone::new(
        party_repo.clone(),
        deal_repo.clone(),
        milestone_repo.clone(),
    );
    let milestone = create
        .execute(CreateMilestoneCommand {
            actor_user_id: user_id,
            actor_party_id: supplier_id,
            deal_id,
            milestone_name: "Build prototype".to_string(),
            description: None,
            assigned_to_party_id: consumer_id,
            verified_by_party_id: enhancer_id,
            due_date: None,
            completion_criteria: "Working prototype".to_string(),
            payment_trigger_amount: Some(Decimal::from(100)),
            display_order: 1,
        })
        .await
        .unwrap();

    let start = StartMilestone::new(
        party_repo.clone(),
        deal_repo.clone(),
        milestone_repo.clone(),
    );
    let started = start
        .execute(MilestoneActionCommand {
            actor_user_id: user_id,
            actor_party_id: consumer_id,
            milestone_id: milestone.id,
            comment: None,
        })
        .await
        .unwrap();
    assert_eq!(started.milestone_status, "IN_PROGRESS");

    let complete = CompleteMilestone::new(
        party_repo.clone(),
        deal_repo.clone(),
        milestone_repo.clone(),
    );
    let completed = complete
        .execute(MilestoneActionCommand {
            actor_user_id: user_id,
            actor_party_id: consumer_id,
            milestone_id: milestone.id,
            comment: None,
        })
        .await
        .unwrap();
    assert_eq!(completed.milestone_status, "COMPLETED");

    let verify = VerifyMilestone::new(
        party_repo.clone(),
        deal_repo.clone(),
        milestone_repo.clone(),
        wallet_repo.clone(),
    );
    let verified = verify
        .execute(MilestoneActionCommand {
            actor_user_id: user_id,
            actor_party_id: enhancer_id,
            milestone_id: milestone.id,
            comment: None,
        })
        .await
        .unwrap();
    assert_eq!(verified.milestone.milestone_status, "VERIFIED");
    assert!(verified.triggered_transaction_id.is_some());
    assert_eq!(wallet_repo.transactions.lock().unwrap().len(), 1);

    let txn = wallet_repo
        .transactions
        .lock()
        .unwrap()
        .first()
        .cloned()
        .unwrap();
    assert_eq!(
        txn.transaction_type,
        domain::entities::TransactionType::EscrowRelease
    );
    assert_eq!(txn.amount, Decimal::from(100));
}

#[tokio::test]
async fn update_milestone_changes_fields() {
    let (
        party_repo,
        deal_repo,
        milestone_repo,
        _,
        user_id,
        supplier_id,
        consumer_id,
        enhancer_id,
        deal_id,
    ) = setup();

    let create = CreateMilestone::new(
        party_repo.clone(),
        deal_repo.clone(),
        milestone_repo.clone(),
    );
    let milestone = create
        .execute(CreateMilestoneCommand {
            actor_user_id: user_id,
            actor_party_id: supplier_id,
            deal_id,
            milestone_name: "Old name".to_string(),
            description: None,
            assigned_to_party_id: consumer_id,
            verified_by_party_id: enhancer_id,
            due_date: None,
            completion_criteria: "old".to_string(),
            payment_trigger_amount: None,
            display_order: 1,
        })
        .await
        .unwrap();

    let update = UpdateMilestone::new(
        party_repo.clone(),
        deal_repo.clone(),
        milestone_repo.clone(),
    );
    let updated = update
        .execute(UpdateMilestoneCommand {
            actor_user_id: user_id,
            actor_party_id: supplier_id,
            milestone_id: milestone.id,
            milestone_name: Some("New name".to_string()),
            description: Some("desc".to_string()),
            assigned_to_party_id: Some(consumer_id),
            verified_by_party_id: Some(enhancer_id),
            due_date: None,
            completion_criteria: Some("new".to_string()),
            payment_trigger_amount: Some(Decimal::from(50)),
            display_order: Some(2),
        })
        .await
        .unwrap();

    assert_eq!(updated.milestone_name, "New name");
    assert_eq!(updated.description, Some("desc".to_string()));
    assert_eq!(updated.display_order, 2);
    assert_eq!(updated.payment_trigger_amount, Some(Decimal::from(50)));
}

#[tokio::test]
async fn delete_milestone_removes_record() {
    let (
        party_repo,
        deal_repo,
        milestone_repo,
        _,
        user_id,
        supplier_id,
        consumer_id,
        enhancer_id,
        deal_id,
    ) = setup();

    let create = CreateMilestone::new(
        party_repo.clone(),
        deal_repo.clone(),
        milestone_repo.clone(),
    );
    let milestone = create
        .execute(CreateMilestoneCommand {
            actor_user_id: user_id,
            actor_party_id: supplier_id,
            deal_id,
            milestone_name: "To delete".to_string(),
            description: None,
            assigned_to_party_id: consumer_id,
            verified_by_party_id: enhancer_id,
            due_date: None,
            completion_criteria: "x".to_string(),
            payment_trigger_amount: None,
            display_order: 1,
        })
        .await
        .unwrap();

    let delete = DeleteMilestone::new(party_repo, deal_repo, milestone_repo.clone());
    delete
        .execute(MilestoneActionCommand {
            actor_user_id: user_id,
            actor_party_id: supplier_id,
            milestone_id: milestone.id,
            comment: None,
        })
        .await
        .unwrap();

    assert_eq!(milestone_repo.count_by_deal(deal_id).await.unwrap(), 0);
}
