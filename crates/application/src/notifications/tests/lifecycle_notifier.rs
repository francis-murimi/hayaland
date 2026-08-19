use std::sync::Arc;
use uuid::Uuid;

use crate::errors::ApplicationError;
use crate::notifications::tests::fake_repos::{
    test_deal, test_deal_aggregate, test_membership, test_party, test_template, FakeDealRepo,
    FakeEmailQueue, FakeNotificationPreferenceRepo, FakeNotificationPublisher,
    FakeNotificationRepo, FakeNotificationTemplateRepo, FakePartyRepo, FakePushSender,
    FakeSmsSender, FakeUserRepo,
};
use crate::notifications::LifecycleNotifier;
use crate::notifications::SendNotification;
use domain::entities::{NotificationChannel, NotificationType};

fn build_notifier(
    notification_repo: Arc<FakeNotificationRepo>,
    template_repo: Arc<FakeNotificationTemplateRepo>,
    deal_repo: Arc<FakeDealRepo>,
    party_repo: Arc<FakePartyRepo>,
    user_repo: Arc<FakeUserRepo>,
) -> LifecycleNotifier {
    let prefs = Arc::new(FakeNotificationPreferenceRepo::new());
    let email_queue = Arc::new(FakeEmailQueue::new());
    let publisher = Arc::new(FakeNotificationPublisher::new());
    let send = Arc::new(SendNotification::new(
        notification_repo,
        prefs,
        template_repo,
        user_repo,
        party_repo,
        deal_repo,
        email_queue,
        publisher,
        Arc::new(FakePushSender),
        Arc::new(FakeSmsSender),
        "en".to_string(),
    ));
    LifecycleNotifier::new(send)
}

#[tokio::test]
async fn notify_deal_participants_creates_party_level_notifications() {
    let deal_id = Uuid::now_v7();
    let supplier_id = Uuid::now_v7();
    let consumer_id = Uuid::now_v7();
    let enhancer_id = Uuid::now_v7();
    let actor_user_id = Uuid::now_v7();

    let deal = test_deal(deal_id, supplier_id);
    let aggregate = test_deal_aggregate(deal, &[supplier_id, consumer_id, enhancer_id]);

    let deal_repo = Arc::new(FakeDealRepo::new());
    deal_repo.with_aggregate(aggregate);

    let party_repo = Arc::new(FakePartyRepo::new());
    party_repo.with_party(test_party(supplier_id, "supplier@example.com", "Supplier"));
    party_repo.with_party(test_party(consumer_id, "consumer@example.com", "Consumer"));
    party_repo.with_party(test_party(enhancer_id, "enhancer@example.com", "Enhancer"));

    let template_repo = Arc::new(FakeNotificationTemplateRepo::new());
    template_repo.with(test_template(
        Uuid::now_v7(),
        "deal_submitted_in_app",
        NotificationType::DealSubmitted,
        NotificationChannel::InApp,
        "en",
        "",
        "Deal {{deal_name}} submitted",
    ));

    let notifications = Arc::new(FakeNotificationRepo::new());
    let user_repo = Arc::new(FakeUserRepo::new());

    let notifier = build_notifier(
        notifications.clone(),
        template_repo,
        deal_repo,
        party_repo,
        user_repo,
    );

    let ids = notifier
        .notify_deal_participants(
            actor_user_id,
            deal_id,
            NotificationType::DealSubmitted,
            serde_json::json!({"deal_name": "Test deal"}),
        )
        .await
        .unwrap();

    assert_eq!(ids.len(), 3);
    assert_eq!(notifications.notifications.lock().unwrap().len(), 3);
}

#[tokio::test]
async fn notify_party_members_creates_one_per_member() {
    let party_id = Uuid::now_v7();
    let user1 = Uuid::now_v7();
    let user2 = Uuid::now_v7();
    let actor_user_id = Uuid::now_v7();

    let party_repo = Arc::new(FakePartyRepo::new());
    party_repo.with_party(test_party(party_id, "party@example.com", "Party"));
    party_repo.with_membership(test_membership(Uuid::now_v7(), user1, party_id));
    party_repo.with_membership(test_membership(Uuid::now_v7(), user2, party_id));

    let template_repo = Arc::new(FakeNotificationTemplateRepo::new());
    template_repo.with(test_template(
        Uuid::now_v7(),
        "deal_invite_in_app",
        NotificationType::DealInvite,
        NotificationChannel::InApp,
        "en",
        "",
        "Invite",
    ));

    let notifications = Arc::new(FakeNotificationRepo::new());
    let user_repo = Arc::new(FakeUserRepo::new());

    let notifier = build_notifier(
        notifications.clone(),
        template_repo,
        Arc::new(FakeDealRepo::new()),
        party_repo,
        user_repo,
    );

    let ids = notifier
        .notify_party_members(
            actor_user_id,
            party_id,
            NotificationType::DealInvite,
            None,
            None,
            serde_json::json!({}),
        )
        .await
        .unwrap();

    assert_eq!(ids.len(), 2);
    assert_eq!(notifications.notifications.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn notify_party_members_builds_deal_action_url() {
    let party_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let related_id = Uuid::now_v7();
    let actor_user_id = Uuid::now_v7();

    let party_repo = Arc::new(FakePartyRepo::new());
    party_repo.with_party(test_party(party_id, "party@example.com", "Party"));
    party_repo.with_membership(test_membership(Uuid::now_v7(), user_id, party_id));

    let template_repo = Arc::new(FakeNotificationTemplateRepo::new());
    template_repo.with(test_template(
        Uuid::now_v7(),
        "milestone_in_app",
        NotificationType::MilestoneCompleted,
        NotificationChannel::InApp,
        "en",
        "",
        "Milestone",
    ));

    let notifications = Arc::new(FakeNotificationRepo::new());
    let notifier = build_notifier(
        notifications.clone(),
        template_repo,
        Arc::new(FakeDealRepo::new()),
        party_repo,
        Arc::new(FakeUserRepo::new()),
    );

    notifier
        .notify_party_members(
            actor_user_id,
            party_id,
            NotificationType::MilestoneCompleted,
            Some("deal"),
            Some(related_id),
            serde_json::json!({}),
        )
        .await
        .unwrap();

    let guard = notifications.notifications.lock().unwrap();
    assert_eq!(guard.len(), 1);
    assert_eq!(guard[0].action_url, Some(format!("/deals/{}", related_id)));
}

#[tokio::test]
async fn notify_party_members_builds_verification_action_url() {
    let party_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let related_id = Uuid::now_v7();
    let actor_user_id = Uuid::now_v7();

    let party_repo = Arc::new(FakePartyRepo::new());
    party_repo.with_party(test_party(party_id, "party@example.com", "Party"));
    party_repo.with_membership(test_membership(Uuid::now_v7(), user_id, party_id));

    let template_repo = Arc::new(FakeNotificationTemplateRepo::new());
    template_repo.with(test_template(
        Uuid::now_v7(),
        "verify_in_app",
        NotificationType::VerificationApproved,
        NotificationChannel::InApp,
        "en",
        "",
        "Verify",
    ));

    let notifications = Arc::new(FakeNotificationRepo::new());
    let notifier = build_notifier(
        notifications.clone(),
        template_repo,
        Arc::new(FakeDealRepo::new()),
        party_repo,
        Arc::new(FakeUserRepo::new()),
    );

    notifier
        .notify_party_members(
            actor_user_id,
            party_id,
            NotificationType::VerificationApproved,
            Some("verification"),
            Some(related_id),
            serde_json::json!({}),
        )
        .await
        .unwrap();

    let guard = notifications.notifications.lock().unwrap();
    assert_eq!(guard.len(), 1);
    assert_eq!(
        guard[0].action_url,
        Some(format!("/verifications/{}", related_id))
    );
}

#[tokio::test]
async fn notify_party_members_builds_custom_action_url() {
    let party_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let related_id = Uuid::now_v7();
    let actor_user_id = Uuid::now_v7();

    let party_repo = Arc::new(FakePartyRepo::new());
    party_repo.with_party(test_party(party_id, "party@example.com", "Party"));
    party_repo.with_membership(test_membership(Uuid::now_v7(), user_id, party_id));

    let template_repo = Arc::new(FakeNotificationTemplateRepo::new());
    template_repo.with(test_template(
        Uuid::now_v7(),
        "doc_in_app",
        NotificationType::Custom,
        NotificationChannel::InApp,
        "en",
        "",
        "Document",
    ));

    let notifications = Arc::new(FakeNotificationRepo::new());
    let notifier = build_notifier(
        notifications.clone(),
        template_repo,
        Arc::new(FakeDealRepo::new()),
        party_repo,
        Arc::new(FakeUserRepo::new()),
    );

    notifier
        .notify_party_members(
            actor_user_id,
            party_id,
            NotificationType::Custom,
            Some("documents"),
            Some(related_id),
            serde_json::json!({}),
        )
        .await
        .unwrap();

    let guard = notifications.notifications.lock().unwrap();
    assert_eq!(guard.len(), 1);
    assert_eq!(
        guard[0].action_url,
        Some(format!("/documents/{}", related_id))
    );
}

#[tokio::test]
async fn notify_user_creates_notification() {
    let user_id = Uuid::now_v7();
    let actor_user_id = Uuid::now_v7();

    let template_repo = Arc::new(FakeNotificationTemplateRepo::new());
    template_repo.with(test_template(
        Uuid::now_v7(),
        "system_in_app",
        NotificationType::SystemMaintenance,
        NotificationChannel::InApp,
        "en",
        "",
        "Maintenance",
    ));

    let notifications = Arc::new(FakeNotificationRepo::new());
    let notifier = build_notifier(
        notifications.clone(),
        template_repo,
        Arc::new(FakeDealRepo::new()),
        Arc::new(FakePartyRepo::new()),
        Arc::new(FakeUserRepo::new()),
    );

    let ids = notifier
        .notify_user(
            actor_user_id,
            user_id,
            NotificationType::SystemMaintenance,
            None,
            None,
            serde_json::json!({}),
        )
        .await
        .unwrap();

    assert_eq!(ids.len(), 1);
    let guard = notifications.notifications.lock().unwrap();
    assert_eq!(guard.len(), 1);
    assert_eq!(guard[0].user_id, Some(user_id));
    assert_eq!(guard[0].action_url, None);
}

#[tokio::test]
async fn notify_user_builds_action_url_for_related_deal() {
    let user_id = Uuid::now_v7();
    let deal_id = Uuid::now_v7();
    let actor_user_id = Uuid::now_v7();

    let template_repo = Arc::new(FakeNotificationTemplateRepo::new());
    template_repo.with(test_template(
        Uuid::now_v7(),
        "deal_in_app",
        NotificationType::DealCommitted,
        NotificationChannel::InApp,
        "en",
        "",
        "Committed",
    ));

    let notifications = Arc::new(FakeNotificationRepo::new());
    let notifier = build_notifier(
        notifications.clone(),
        template_repo,
        Arc::new(FakeDealRepo::new()),
        Arc::new(FakePartyRepo::new()),
        Arc::new(FakeUserRepo::new()),
    );

    notifier
        .notify_user(
            actor_user_id,
            user_id,
            NotificationType::DealCommitted,
            Some("deal"),
            Some(deal_id),
            serde_json::json!({}),
        )
        .await
        .unwrap();

    let guard = notifications.notifications.lock().unwrap();
    assert_eq!(guard[0].action_url, Some(format!("/deals/{}", deal_id)));
}

#[tokio::test]
async fn fire_and_forget_does_not_panic_on_ok() {
    let notifier = build_notifier(
        Arc::new(FakeNotificationRepo::new()),
        Arc::new(FakeNotificationTemplateRepo::new()),
        Arc::new(FakeDealRepo::new()),
        Arc::new(FakePartyRepo::new()),
        Arc::new(FakeUserRepo::new()),
    );

    notifier.fire_and_forget(Ok(vec![Uuid::now_v7()]), "test_ok");
}

#[tokio::test]
async fn fire_and_forget_does_not_panic_on_err() {
    let notifier = build_notifier(
        Arc::new(FakeNotificationRepo::new()),
        Arc::new(FakeNotificationTemplateRepo::new()),
        Arc::new(FakeDealRepo::new()),
        Arc::new(FakePartyRepo::new()),
        Arc::new(FakeUserRepo::new()),
    );

    notifier.fire_and_forget(Err(ApplicationError::DealNotFound), "test_err");
}
