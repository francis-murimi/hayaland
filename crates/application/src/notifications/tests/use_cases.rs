use std::sync::Arc;

use time::OffsetDateTime;
use uuid::Uuid;

use crate::errors::ApplicationError;
use crate::notifications::dto::{
    AdminSendNotificationRequest, ChannelPreferencesDto, MarkAllReadBody, NotificationActionDto,
    NotificationListQuery, NotificationPreferencesDto, NotificationTemplateRequest, QuietHoursDto,
    RecipientSelector, SendNotificationCommand, UpdateNotificationBody,
};
use crate::notifications::tests::fake_repos::{
    quiet_hours_covering_now, sample_notification, test_deal, test_deal_aggregate, test_membership,
    test_party, test_template, test_user, user_with_channels, FakeDealRepo, FakeEmailQueue,
    FakeNotificationPreferenceRepo, FakeNotificationPublisher, FakeNotificationRepo,
    FakeNotificationTemplateRepo, FakePartyRepo, FakePushSender, FakeSmsSender, FakeUserRepo,
};
use crate::notifications::{
    AdminCreateTemplate, AdminDeleteTemplate, AdminGetTemplate, AdminListTemplates,
    AdminSendNotification, AdminUpdateTemplate, DeleteNotification, GetNotification,
    GetNotificationPreferences, GetUnreadCount, ListNotifications, MarkAllNotificationsRead,
    MarkNotificationRead, SendNotification, UpdateNotificationPreferences,
};
use domain::entities::{
    ActionType, NotificationChannel, NotificationPriority, NotificationStatus, NotificationType,
};
use domain::repositories::{
    NotificationPreferenceRepository, NotificationRepository, NotificationTemplateRepository,
};

fn send_cmd(
    actor_user_id: Uuid,
    recipient: RecipientSelector,
    notification_type: NotificationType,
    priority: NotificationPriority,
) -> SendNotificationCommand {
    SendNotificationCommand {
        actor_user_id,
        actor_party_id: None,
        recipient,
        notification_type,
        priority,
        title: None,
        body: None,
        action_url: None,
        actions: vec![NotificationActionDto {
            label: "View".to_string(),
            action_type: ActionType::Navigate,
            url: Some("/".to_string()),
            method: None,
        }],
        related_entity_type: None,
        related_entity_id: None,
        metadata: serde_json::json!({"body": "Custom body from metadata."}),
        locale: "en".to_string(),
    }
}

fn send_notification_use_case(
    notification_repo: Arc<FakeNotificationRepo>,
    preference_repo: Arc<FakeNotificationPreferenceRepo>,
    template_repo: Arc<FakeNotificationTemplateRepo>,
    user_repo: Arc<FakeUserRepo>,
    party_repo: Arc<FakePartyRepo>,
    deal_repo: Arc<FakeDealRepo>,
    email_queue: Arc<FakeEmailQueue>,
    publisher: Arc<FakeNotificationPublisher>,
) -> SendNotification {
    SendNotification::new(
        notification_repo,
        preference_repo,
        template_repo,
        user_repo,
        party_repo,
        deal_repo,
        email_queue,
        publisher,
        Arc::new(FakePushSender),
        Arc::new(FakeSmsSender),
        "en".to_string(),
    )
}

fn admin_send_use_case(send: Arc<SendNotification>) -> AdminSendNotification {
    AdminSendNotification::new(send)
}

#[tokio::test]
async fn send_notification_to_user_creates_notification() {
    let user = test_user(Uuid::now_v7(), "user@example.com", "user");
    let user_repo = Arc::new(FakeUserRepo::new());
    user_repo.with(user.clone());

    let prefs = Arc::new(FakeNotificationPreferenceRepo::new());
    let notifications = Arc::new(FakeNotificationRepo::new());
    let templates = Arc::new(FakeNotificationTemplateRepo::new());
    let parties = Arc::new(FakePartyRepo::new());
    let deals = Arc::new(FakeDealRepo::new());
    let emails = Arc::new(FakeEmailQueue::new());
    let publisher = Arc::new(FakeNotificationPublisher::new());

    let use_case = send_notification_use_case(
        notifications.clone(),
        prefs.clone(),
        templates.clone(),
        user_repo,
        parties,
        deals,
        emails.clone(),
        publisher.clone(),
    );

    let ids = use_case
        .execute(send_cmd(
            user.id,
            RecipientSelector::User { user_id: user.id },
            NotificationType::SystemMaintenance,
            NotificationPriority::Normal,
        ))
        .await
        .unwrap();

    assert_eq!(ids.len(), 1);

    let guard = notifications.notifications.lock().unwrap();
    assert_eq!(guard.len(), 1);
    let n = &guard[0];
    assert_eq!(n.user_id, Some(user.id));
    assert_eq!(n.notification_type, NotificationType::SystemMaintenance);
    assert_eq!(n.status, NotificationStatus::Pending);
    assert!(n.channels.contains(&NotificationChannel::InApp));
    assert!(n.channels.contains(&NotificationChannel::Email));

    let events = publisher.events.lock().unwrap();
    assert!(events.iter().any(|e| matches!(
        e,
        crate::ports::NotificationEvent::NotificationNew { notification_id, user_id, .. }
        if *notification_id == ids[0] && *user_id == Some(user.id)
    )));
}

#[tokio::test]
async fn send_notification_to_party_members_creates_one_per_member() {
    let party = test_party(Uuid::now_v7(), "party@example.com", "Party");
    let user1 = test_user(Uuid::now_v7(), "u1@example.com", "user_one");
    let user2 = test_user(Uuid::now_v7(), "u2@example.com", "user_two");

    let party_repo = Arc::new(FakePartyRepo::new());
    party_repo.with_party(party.clone());
    party_repo.with_membership(test_membership(Uuid::now_v7(), user1.id, party.id));
    party_repo.with_membership(test_membership(Uuid::now_v7(), user2.id, party.id));

    let user_repo = Arc::new(FakeUserRepo::new());
    user_repo.with(user1.clone());
    user_repo.with(user2.clone());

    let notifications = Arc::new(FakeNotificationRepo::new());
    let use_case = send_notification_use_case(
        notifications.clone(),
        Arc::new(FakeNotificationPreferenceRepo::new()),
        Arc::new(FakeNotificationTemplateRepo::new()),
        user_repo,
        party_repo,
        Arc::new(FakeDealRepo::new()),
        Arc::new(FakeEmailQueue::new()),
        Arc::new(FakeNotificationPublisher::new()),
    );

    let ids = use_case
        .execute(send_cmd(
            user1.id,
            RecipientSelector::PartyMembers { party_id: party.id },
            NotificationType::DealInvite,
            NotificationPriority::High,
        ))
        .await
        .unwrap();

    assert_eq!(ids.len(), 2);
    let guard = notifications.notifications.lock().unwrap();
    assert_eq!(guard.len(), 2);
    assert!(guard.iter().any(|n| n.user_id == Some(user1.id)));
    assert!(guard.iter().any(|n| n.user_id == Some(user2.id)));
}

#[tokio::test]
async fn send_notification_to_deal_participants_creates_one_per_party() {
    let party_a = test_party(Uuid::now_v7(), "a@example.com", "Party A");
    let party_b = test_party(Uuid::now_v7(), "b@example.com", "Party B");
    let deal = test_deal(Uuid::now_v7(), party_a.id);
    let aggregate = test_deal_aggregate(deal.clone(), &[party_a.id, party_b.id]);

    let deal_repo = Arc::new(FakeDealRepo::new());
    deal_repo.with_aggregate(aggregate);

    let party_repo = Arc::new(FakePartyRepo::new());
    party_repo.with_party(party_a.clone());
    party_repo.with_party(party_b.clone());

    let notifications = Arc::new(FakeNotificationRepo::new());
    let use_case = send_notification_use_case(
        notifications.clone(),
        Arc::new(FakeNotificationPreferenceRepo::new()),
        Arc::new(FakeNotificationTemplateRepo::new()),
        Arc::new(FakeUserRepo::new()),
        party_repo,
        deal_repo,
        Arc::new(FakeEmailQueue::new()),
        Arc::new(FakeNotificationPublisher::new()),
    );

    let ids = use_case
        .execute(send_cmd(
            Uuid::nil(),
            RecipientSelector::DealParticipants { deal_id: deal.id },
            NotificationType::DealCommitted,
            NotificationPriority::High,
        ))
        .await
        .unwrap();

    assert_eq!(ids.len(), 2);
    let guard = notifications.notifications.lock().unwrap();
    assert_eq!(guard.len(), 2);
    assert!(guard.iter().any(|n| n.party_id == Some(party_a.id)));
    assert!(guard.iter().any(|n| n.party_id == Some(party_b.id)));
}

#[tokio::test]
async fn send_notification_with_all_channels_disabled_is_suppressed() {
    let user = test_user(Uuid::now_v7(), "user@example.com", "user");
    let prefs = user_with_channels(user.id, vec![]);
    let preference_repo = Arc::new(FakeNotificationPreferenceRepo::new());
    preference_repo.with(prefs);

    let user_repo = Arc::new(FakeUserRepo::new());
    user_repo.with(user.clone());

    let notifications = Arc::new(FakeNotificationRepo::new());
    let publisher = Arc::new(FakeNotificationPublisher::new());
    let use_case = send_notification_use_case(
        notifications.clone(),
        preference_repo,
        Arc::new(FakeNotificationTemplateRepo::new()),
        user_repo,
        Arc::new(FakePartyRepo::new()),
        Arc::new(FakeDealRepo::new()),
        Arc::new(FakeEmailQueue::new()),
        publisher.clone(),
    );

    let ids = use_case
        .execute(send_cmd(
            user.id,
            RecipientSelector::User { user_id: user.id },
            NotificationType::SystemMaintenance,
            NotificationPriority::Normal,
        ))
        .await
        .unwrap();

    assert_eq!(ids.len(), 1);
    let guard = notifications.notifications.lock().unwrap();
    assert_eq!(guard[0].status, NotificationStatus::Suppressed);
    assert_eq!(guard[0].channels, vec![NotificationChannel::InApp]);

    // No real-time event is published for a suppressed notification.
    assert!(publisher.events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn send_notification_respects_quiet_hours() {
    let user = test_user(Uuid::now_v7(), "user@example.com", "user");
    let mut prefs = user_with_channels(
        user.id,
        vec![
            NotificationChannel::InApp,
            NotificationChannel::Email,
            NotificationChannel::Push,
        ],
    );
    prefs.quiet_hours = quiet_hours_covering_now();

    let preference_repo = Arc::new(FakeNotificationPreferenceRepo::new());
    preference_repo.with(prefs);

    let user_repo = Arc::new(FakeUserRepo::new());
    user_repo.with(user.clone());

    let notifications = Arc::new(FakeNotificationRepo::new());
    let emails = Arc::new(FakeEmailQueue::new());
    let publisher = Arc::new(FakeNotificationPublisher::new());
    let use_case = send_notification_use_case(
        notifications.clone(),
        preference_repo,
        Arc::new(FakeNotificationTemplateRepo::new()),
        user_repo,
        Arc::new(FakePartyRepo::new()),
        Arc::new(FakeDealRepo::new()),
        emails.clone(),
        publisher.clone(),
    );

    use_case
        .execute(send_cmd(
            user.id,
            RecipientSelector::User { user_id: user.id },
            NotificationType::DealInvite,
            NotificationPriority::Normal,
        ))
        .await
        .unwrap();

    let guard = notifications.notifications.lock().unwrap();
    assert_eq!(guard.len(), 1);
    assert_eq!(guard[0].channels, vec![NotificationChannel::InApp]);
    assert_eq!(guard[0].status, NotificationStatus::Pending);

    // Email should not have been enqueued because quiet hours removed it.
    assert!(emails.items.lock().unwrap().is_empty());
}

#[tokio::test]
async fn send_notification_falls_back_when_no_template_exists() {
    let user = test_user(Uuid::now_v7(), "user@example.com", "user");
    let user_repo = Arc::new(FakeUserRepo::new());
    user_repo.with(user.clone());

    let notifications = Arc::new(FakeNotificationRepo::new());
    let use_case = send_notification_use_case(
        notifications.clone(),
        Arc::new(FakeNotificationPreferenceRepo::new()),
        Arc::new(FakeNotificationTemplateRepo::new()),
        user_repo,
        Arc::new(FakePartyRepo::new()),
        Arc::new(FakeDealRepo::new()),
        Arc::new(FakeEmailQueue::new()),
        Arc::new(FakeNotificationPublisher::new()),
    );

    let ids = use_case
        .execute(send_cmd(
            user.id,
            RecipientSelector::User { user_id: user.id },
            NotificationType::DealInvite,
            NotificationPriority::High,
        ))
        .await
        .unwrap();

    let guard = notifications.notifications.lock().unwrap();
    let n = &guard[0];
    assert_eq!(n.id, ids[0]);
    assert_eq!(n.title, "New deal invitation");
    assert_eq!(n.body, "Custom body from metadata.");
}

#[tokio::test]
async fn send_notification_enqueues_email_when_channel_enabled() {
    let user = test_user(Uuid::now_v7(), "user@example.com", "user");
    let user_repo = Arc::new(FakeUserRepo::new());
    user_repo.with(user.clone());

    let template_repo = Arc::new(FakeNotificationTemplateRepo::new());
    template_repo.with(test_template(
        Uuid::now_v7(),
        "deal_invite_email",
        NotificationType::DealInvite,
        NotificationChannel::Email,
        "en",
        "Invitation",
        "You have been invited.",
    ));

    let emails = Arc::new(FakeEmailQueue::new());
    let notifications = Arc::new(FakeNotificationRepo::new());
    let use_case = send_notification_use_case(
        notifications.clone(),
        Arc::new(FakeNotificationPreferenceRepo::new()),
        template_repo,
        user_repo,
        Arc::new(FakePartyRepo::new()),
        Arc::new(FakeDealRepo::new()),
        emails.clone(),
        Arc::new(FakeNotificationPublisher::new()),
    );

    use_case
        .execute(send_cmd(
            user.id,
            RecipientSelector::User { user_id: user.id },
            NotificationType::DealInvite,
            NotificationPriority::High,
        ))
        .await
        .unwrap();

    let items = emails.items.lock().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].to, "user@example.com");
    assert_eq!(items[0].subject, "Invitation");
    assert_eq!(items[0].body, "You have been invited.");
}

#[tokio::test]
async fn admin_send_notification_wraps_send_notification() {
    let user = test_user(Uuid::now_v7(), "user@example.com", "user");
    let user_repo = Arc::new(FakeUserRepo::new());
    user_repo.with(user.clone());

    let notifications = Arc::new(FakeNotificationRepo::new());
    let send = Arc::new(send_notification_use_case(
        notifications.clone(),
        Arc::new(FakeNotificationPreferenceRepo::new()),
        Arc::new(FakeNotificationTemplateRepo::new()),
        user_repo,
        Arc::new(FakePartyRepo::new()),
        Arc::new(FakeDealRepo::new()),
        Arc::new(FakeEmailQueue::new()),
        Arc::new(FakeNotificationPublisher::new()),
    ));

    let admin_use_case = admin_send_use_case(send);

    let result = admin_use_case
        .execute(
            Uuid::nil(),
            AdminSendNotificationRequest {
                target: RecipientSelector::User { user_id: user.id },
                notification_type: NotificationType::AdminBroadcast,
                priority: NotificationPriority::Normal,
                title: Some("Broadcast".to_string()),
                body: Some("Hello everyone".to_string()),
                action_url: None,
                actions: vec![],
                related_entity_type: None,
                related_entity_id: None,
                metadata: serde_json::json!({}),
                expires_at: None,
                locale: "en".to_string(),
            },
        )
        .await
        .unwrap();

    assert_eq!(result.sent_count, 1);
    assert_eq!(result.notification_ids.len(), 1);
    assert_eq!(notifications.notifications.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn list_notifications_returns_paginated_results() {
    let user_id = Uuid::now_v7();
    let n1 = sample_notification(
        Uuid::now_v7(),
        Some(user_id),
        None,
        NotificationType::DealInvite,
    );
    let mut n2 = sample_notification(
        Uuid::now_v7(),
        Some(user_id),
        None,
        NotificationType::DealCompleted,
    );
    n2.read_at = Some(OffsetDateTime::now_utc());

    let repo = Arc::new(FakeNotificationRepo::new());
    repo.create(&n1).await.unwrap();
    repo.create(&n2).await.unwrap();

    let use_case = ListNotifications::new(repo);
    let result = use_case
        .execute(user_id, None, NotificationListQuery::default())
        .await
        .unwrap();

    assert_eq!(result.total, 2);
    assert_eq!(result.unread_count, 1);
    assert_eq!(result.data.len(), 2);
    assert!(!result.data[0].is_read);
    assert!(result.data[1].is_read);
}

#[tokio::test]
async fn get_notification_returns_owned_notification() {
    let user_id = Uuid::now_v7();
    let n = sample_notification(
        Uuid::now_v7(),
        Some(user_id),
        None,
        NotificationType::DealInvite,
    );

    let repo = Arc::new(FakeNotificationRepo::new());
    repo.create(&n).await.unwrap();

    let use_case = GetNotification::new(repo);
    let result = use_case.execute(n.id, user_id, None).await.unwrap();
    assert_eq!(result.id, n.id);
    assert_eq!(result.user_id, Some(user_id));
}

#[tokio::test]
async fn get_notification_not_found() {
    let repo = Arc::new(FakeNotificationRepo::new());
    let use_case = GetNotification::new(repo);

    let err = use_case
        .execute(Uuid::now_v7(), Uuid::now_v7(), None)
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::NotificationNotFound));
}

#[tokio::test]
async fn get_notification_forbidden_for_other_user() {
    let owner = Uuid::now_v7();
    let other = Uuid::now_v7();
    let n = sample_notification(
        Uuid::now_v7(),
        Some(owner),
        None,
        NotificationType::DealInvite,
    );

    let repo = Arc::new(FakeNotificationRepo::new());
    repo.create(&n).await.unwrap();

    let use_case = GetNotification::new(repo);
    let err = use_case.execute(n.id, other, None).await.unwrap_err();

    assert!(matches!(err, ApplicationError::Forbidden));
}

#[tokio::test]
async fn mark_notification_read_marks_as_read_and_publishes_events() {
    let user_id = Uuid::now_v7();
    let n = sample_notification(
        Uuid::now_v7(),
        Some(user_id),
        None,
        NotificationType::DealInvite,
    );

    let repo = Arc::new(FakeNotificationRepo::new());
    repo.create(&n).await.unwrap();

    let publisher = Arc::new(FakeNotificationPublisher::new());
    let use_case = MarkNotificationRead::new(repo.clone(), publisher.clone());

    use_case
        .execute(
            n.id,
            user_id,
            None,
            UpdateNotificationBody {
                is_read: Some(true),
                is_actioned: None,
            },
        )
        .await
        .unwrap();

    let stored = repo.find_by_id(n.id).await.unwrap().unwrap();
    assert!(stored.read_at.is_some());

    let events = publisher.events.lock().unwrap();
    assert!(events.iter().any(|e| matches!(
        e,
        crate::ports::NotificationEvent::NotificationRead { notification_id, user_id: uid }
        if *notification_id == n.id && *uid == user_id
    )));
    assert!(events.iter().any(|e| matches!(
        e,
        crate::ports::NotificationEvent::UnreadCountChanged { user_id: uid, count, .. }
        if uid == &Some(user_id) && *count == 0
    )));
}

#[tokio::test]
async fn mark_all_notifications_read_marks_everything_read() {
    let user_id = Uuid::now_v7();
    let n1 = sample_notification(
        Uuid::now_v7(),
        Some(user_id),
        None,
        NotificationType::DealInvite,
    );
    let n2 = sample_notification(
        Uuid::now_v7(),
        Some(user_id),
        None,
        NotificationType::DealCompleted,
    );

    let repo = Arc::new(FakeNotificationRepo::new());
    repo.create(&n1).await.unwrap();
    repo.create(&n2).await.unwrap();

    let publisher = Arc::new(FakeNotificationPublisher::new());
    let use_case = MarkAllNotificationsRead::new(repo.clone(), publisher.clone());

    let count = use_case
        .execute(user_id, None, MarkAllReadBody::default())
        .await
        .unwrap();

    assert_eq!(count, 2);
    assert_eq!(
        repo.count_unread_for_recipient(Some(user_id), None)
            .await
            .unwrap(),
        0
    );

    let events = publisher.events.lock().unwrap();
    assert!(events.iter().any(|e| matches!(
        e,
        crate::ports::NotificationEvent::UnreadCountChanged { user_id: uid, count, .. }
        if uid == &Some(user_id) && *count == 0
    )));
}

#[tokio::test]
async fn delete_notification_removes_owned_notification() {
    let user_id = Uuid::now_v7();
    let n = sample_notification(
        Uuid::now_v7(),
        Some(user_id),
        None,
        NotificationType::DealInvite,
    );

    let repo = Arc::new(FakeNotificationRepo::new());
    repo.create(&n).await.unwrap();

    let use_case = DeleteNotification::new(repo.clone());
    use_case.execute(n.id, user_id, None).await.unwrap();

    assert!(repo.find_by_id(n.id).await.unwrap().is_none());
}

#[tokio::test]
async fn delete_notification_not_found() {
    let repo = Arc::new(FakeNotificationRepo::new());
    let use_case = DeleteNotification::new(repo);

    let err = use_case
        .execute(Uuid::now_v7(), Uuid::now_v7(), None)
        .await
        .unwrap_err();

    assert!(matches!(err, ApplicationError::NotificationNotFound));
}

#[tokio::test]
async fn delete_notification_forbidden_for_other_user() {
    let owner = Uuid::now_v7();
    let other = Uuid::now_v7();
    let n = sample_notification(
        Uuid::now_v7(),
        Some(owner),
        None,
        NotificationType::DealInvite,
    );

    let repo = Arc::new(FakeNotificationRepo::new());
    repo.create(&n).await.unwrap();

    let use_case = DeleteNotification::new(repo);
    let err = use_case.execute(n.id, other, None).await.unwrap_err();

    assert!(matches!(err, ApplicationError::Forbidden));
}

#[tokio::test]
async fn get_notification_preferences_returns_defaults() {
    let user_id = Uuid::now_v7();
    let repo = Arc::new(FakeNotificationPreferenceRepo::new());
    let use_case = GetNotificationPreferences::new(repo);

    let prefs = use_case.execute(user_id).await.unwrap();
    assert_eq!(prefs.user_id, user_id);
    assert!(prefs.channels.in_app);
    assert!(prefs.channels.email);
    assert!(!prefs.channels.push);
    assert!(!prefs.channels.sms);
}

#[tokio::test]
async fn update_notification_preferences_persists_changes() {
    let user_id = Uuid::now_v7();
    let repo = Arc::new(FakeNotificationPreferenceRepo::new());
    let use_case = UpdateNotificationPreferences::new(repo.clone());

    let dto = NotificationPreferencesDto {
        user_id,
        channels: ChannelPreferencesDto {
            in_app: true,
            email: false,
            push: true,
            sms: false,
        },
        per_type: serde_json::json!({
            "DEAL_INVITE": { "enabled": true, "channels": ["IN_APP"] }
        }),
        quiet_hours: QuietHoursDto {
            enabled: false,
            start: "22:00".to_string(),
            end: "07:00".to_string(),
            timezone: "UTC".to_string(),
            except_critical: true,
        },
    };

    let saved = use_case.execute(user_id, dto).await.unwrap();
    assert!(!saved.channels.email);
    assert!(saved.channels.push);

    let stored = repo.get(user_id).await.unwrap();
    assert!(!stored.channels.email);
    assert!(stored.channels.push);
    assert!(
        stored
            .per_type
            .get(&NotificationType::DealInvite)
            .unwrap()
            .enabled
    );
    assert_eq!(
        stored
            .per_type
            .get(&NotificationType::DealInvite)
            .unwrap()
            .channels,
        vec![NotificationChannel::InApp]
    );
}

#[tokio::test]
async fn update_notification_preferences_forbidden_when_user_id_mismatches() {
    let user_id = Uuid::now_v7();
    let repo = Arc::new(FakeNotificationPreferenceRepo::new());
    let use_case = UpdateNotificationPreferences::new(repo);

    let dto = NotificationPreferencesDto {
        user_id: Uuid::now_v7(),
        channels: ChannelPreferencesDto {
            in_app: true,
            email: false,
            push: false,
            sms: false,
        },
        per_type: serde_json::json!({}),
        quiet_hours: QuietHoursDto {
            enabled: false,
            start: "22:00".to_string(),
            end: "07:00".to_string(),
            timezone: "UTC".to_string(),
            except_critical: true,
        },
    };

    let err = use_case.execute(user_id, dto).await.unwrap_err();
    assert!(matches!(err, ApplicationError::Forbidden));
}

#[tokio::test]
async fn get_unread_count_returns_count() {
    let user_id = Uuid::now_v7();
    let n1 = sample_notification(
        Uuid::now_v7(),
        Some(user_id),
        None,
        NotificationType::DealInvite,
    );
    let mut n2 = sample_notification(
        Uuid::now_v7(),
        Some(user_id),
        None,
        NotificationType::DealCompleted,
    );
    n2.read_at = Some(OffsetDateTime::now_utc());

    let repo = Arc::new(FakeNotificationRepo::new());
    repo.create(&n1).await.unwrap();
    repo.create(&n2).await.unwrap();

    let use_case = GetUnreadCount::new(repo);
    let result = use_case.execute(user_id, None).await.unwrap();
    assert_eq!(result.count, 1);
}

#[tokio::test]
async fn admin_list_templates_returns_templates() {
    let repo = Arc::new(FakeNotificationTemplateRepo::new());
    repo.with(test_template(
        Uuid::now_v7(),
        "welcome_email",
        NotificationType::AdminBroadcast,
        NotificationChannel::Email,
        "en",
        "Welcome",
        "Hello!",
    ));

    let use_case = AdminListTemplates::new(repo);
    let results = use_case.execute(None, None).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "welcome_email");
}

#[tokio::test]
async fn admin_create_template_persists_template() {
    let repo = Arc::new(FakeNotificationTemplateRepo::new());
    let use_case = AdminCreateTemplate::new(repo.clone());

    let result = use_case
        .execute(NotificationTemplateRequest {
            name: "new_template".to_string(),
            notification_type: NotificationType::DealInvite,
            channel: NotificationChannel::Email,
            locale: "en".to_string(),
            subject_template: "Subject".to_string(),
            body_template: "Body".to_string(),
            variables_schema: serde_json::json!({}),
        })
        .await
        .unwrap();

    assert_eq!(result.name, "new_template");
    assert!(repo.find_by_id(result.id).await.unwrap().is_some());
}

#[tokio::test]
async fn admin_create_template_rejects_duplicate_name() {
    let repo = Arc::new(FakeNotificationTemplateRepo::new());
    repo.with(test_template(
        Uuid::now_v7(),
        "duplicate",
        NotificationType::DealInvite,
        NotificationChannel::Email,
        "en",
        "Subject",
        "Body",
    ));

    let use_case = AdminCreateTemplate::new(repo);
    let err = use_case
        .execute(NotificationTemplateRequest {
            name: "duplicate".to_string(),
            notification_type: NotificationType::DealCompleted,
            channel: NotificationChannel::InApp,
            locale: "en".to_string(),
            subject_template: "Subject".to_string(),
            body_template: "Body".to_string(),
            variables_schema: serde_json::json!({}),
        })
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        ApplicationError::DuplicateNotificationTemplate
    ));
}

#[tokio::test]
async fn admin_get_template_returns_template() {
    let id = Uuid::now_v7();
    let repo = Arc::new(FakeNotificationTemplateRepo::new());
    repo.with(test_template(
        id,
        "get_me",
        NotificationType::DealInvite,
        NotificationChannel::Email,
        "en",
        "Subject",
        "Body",
    ));

    let use_case = AdminGetTemplate::new(repo);
    let result = use_case.execute(id).await.unwrap();
    assert_eq!(result.id, id);
}

#[tokio::test]
async fn admin_get_template_not_found() {
    let repo = Arc::new(FakeNotificationTemplateRepo::new());
    let use_case = AdminGetTemplate::new(repo);

    let err = use_case.execute(Uuid::now_v7()).await.unwrap_err();
    assert!(matches!(
        err,
        ApplicationError::NotificationTemplateNotFound
    ));
}

#[tokio::test]
async fn admin_update_template_changes_fields() {
    let id = Uuid::now_v7();
    let repo = Arc::new(FakeNotificationTemplateRepo::new());
    repo.with(test_template(
        id,
        "old",
        NotificationType::DealInvite,
        NotificationChannel::Email,
        "en",
        "Old",
        "Old body",
    ));

    let use_case = AdminUpdateTemplate::new(repo.clone());
    let result = use_case
        .execute(
            id,
            NotificationTemplateRequest {
                name: "new".to_string(),
                notification_type: NotificationType::DealCompleted,
                channel: NotificationChannel::InApp,
                locale: "fr".to_string(),
                subject_template: "New".to_string(),
                body_template: "New body".to_string(),
                variables_schema: serde_json::json!({"x": "y"}),
            },
        )
        .await
        .unwrap();

    assert_eq!(result.name, "new");
    assert_eq!(result.notification_type, "DEAL_COMPLETED");
    assert_eq!(result.channel, "IN_APP");
    assert_eq!(result.locale, "fr");
}

#[tokio::test]
async fn admin_update_template_not_found() {
    let repo = Arc::new(FakeNotificationTemplateRepo::new());
    let use_case = AdminUpdateTemplate::new(repo);

    let err = use_case
        .execute(
            Uuid::now_v7(),
            NotificationTemplateRequest {
                name: "x".to_string(),
                notification_type: NotificationType::DealInvite,
                channel: NotificationChannel::Email,
                locale: "en".to_string(),
                subject_template: "S".to_string(),
                body_template: "B".to_string(),
                variables_schema: serde_json::json!({}),
            },
        )
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        ApplicationError::NotificationTemplateNotFound
    ));
}

#[tokio::test]
async fn admin_update_template_rejects_duplicate_name() {
    let id1 = Uuid::now_v7();
    let id2 = Uuid::now_v7();
    let repo = Arc::new(FakeNotificationTemplateRepo::new());
    repo.with(test_template(
        id1,
        "first",
        NotificationType::DealInvite,
        NotificationChannel::Email,
        "en",
        "S",
        "B",
    ));
    repo.with(test_template(
        id2,
        "second",
        NotificationType::DealInvite,
        NotificationChannel::Email,
        "en",
        "S",
        "B",
    ));

    let use_case = AdminUpdateTemplate::new(repo);
    let err = use_case
        .execute(
            id2,
            NotificationTemplateRequest {
                name: "first".to_string(),
                notification_type: NotificationType::DealInvite,
                channel: NotificationChannel::Email,
                locale: "en".to_string(),
                subject_template: "S".to_string(),
                body_template: "B".to_string(),
                variables_schema: serde_json::json!({}),
            },
        )
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        ApplicationError::DuplicateNotificationTemplate
    ));
}

#[tokio::test]
async fn admin_delete_template_removes_template() {
    let id = Uuid::now_v7();
    let repo = Arc::new(FakeNotificationTemplateRepo::new());
    repo.with(test_template(
        id,
        "delete_me",
        NotificationType::DealInvite,
        NotificationChannel::Email,
        "en",
        "S",
        "B",
    ));

    let use_case = AdminDeleteTemplate::new(repo.clone());
    use_case.execute(id).await.unwrap();
    assert!(repo.find_by_id(id).await.unwrap().is_none());
}
