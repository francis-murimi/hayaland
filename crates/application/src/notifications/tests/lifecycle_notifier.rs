use std::sync::Arc;
use uuid::Uuid;

use crate::notifications::tests::fake_repos::{
    test_deal, test_deal_aggregate, test_party, test_template, FakeDealRepo, FakeEmailQueue,
    FakeNotificationPreferenceRepo, FakeNotificationPublisher, FakeNotificationRepo,
    FakeNotificationTemplateRepo, FakePartyRepo, FakePushSender, FakeSmsSender, FakeUserRepo,
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
