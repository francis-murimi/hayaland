use domain::entities::{
    notification_preference::{
        ChannelPreferences, NotificationPreference, QuietHours, TypePreference,
    },
    Email, NotificationChannel, NotificationType, PasswordHash, User, Username,
};
use domain::repositories::{NotificationPreferenceRepository, UserRepository};
use infrastructure::repositories::{
    PostgresNotificationPreferenceRepository, PostgresUserRepository,
};
use sqlx::PgPool;
use uuid::Uuid;

fn sample_user(email: &str, username: &str) -> User {
    User::new(
        Uuid::now_v7(),
        Email::new(email).unwrap(),
        Username::new(username).unwrap(),
        PasswordHash::new(format!("hash-{username}")).unwrap(),
    )
}

#[sqlx::test(migrations = "../../migrations")]
async fn get_returns_default_preferences_when_none_exist(pool: PgPool) {
    let user_repo = PostgresUserRepository::new(pool.clone());
    let repo = PostgresNotificationPreferenceRepository::new(pool);
    let user = sample_user("pref-default@example.com", "pref_default");
    let user_id = user.id;

    user_repo.create(&user).await.unwrap();

    let prefs = repo.get(user_id).await.unwrap();
    assert_eq!(prefs.user_id, user_id);
    assert!(prefs.channel_enabled(NotificationChannel::InApp));
    assert!(prefs.channel_enabled(NotificationChannel::Email));
    assert!(!prefs.channel_enabled(NotificationChannel::Push));
    assert!(!prefs.channel_enabled(NotificationChannel::Sms));
    assert!(!prefs.quiet_hours.enabled);
    assert!(prefs.type_enabled(NotificationType::DealInvite));
    assert!(prefs.type_enabled(NotificationType::SecurityAlert));
}

#[sqlx::test(migrations = "../../migrations")]
async fn save_and_get_round_trip(pool: PgPool) {
    let user_repo = PostgresUserRepository::new(pool.clone());
    let repo = PostgresNotificationPreferenceRepository::new(pool);
    let user = sample_user("pref-roundtrip@example.com", "pref_roundtrip");
    let user_id = user.id;

    user_repo.create(&user).await.unwrap();

    let mut prefs = NotificationPreference::new(user_id);
    prefs.channels = ChannelPreferences {
        in_app: true,
        email: false,
        push: true,
        sms: true,
    };
    prefs.per_type.insert(
        NotificationType::DealInvite,
        TypePreference {
            enabled: true,
            channels: vec![NotificationChannel::InApp, NotificationChannel::Email],
        },
    );
    prefs.per_type.insert(
        NotificationType::MessageReceived,
        TypePreference {
            enabled: false,
            channels: vec![NotificationChannel::Push],
        },
    );
    prefs.quiet_hours = QuietHours {
        enabled: true,
        start: "21:30".to_string(),
        end: "06:30".to_string(),
        timezone: "Africa/Nairobi".to_string(),
        except_critical: false,
    };

    repo.save(&prefs).await.unwrap();

    let loaded = repo.get(user_id).await.unwrap();
    assert_eq!(loaded.user_id, user_id);
    assert!(loaded.channel_enabled(NotificationChannel::InApp));
    assert!(!loaded.channel_enabled(NotificationChannel::Email));
    assert!(loaded.channel_enabled(NotificationChannel::Push));
    assert!(loaded.channel_enabled(NotificationChannel::Sms));

    let deal_pref = loaded.per_type.get(&NotificationType::DealInvite).unwrap();
    assert!(deal_pref.enabled);
    assert_eq!(
        deal_pref.channels,
        vec![NotificationChannel::InApp, NotificationChannel::Email]
    );

    let message_pref = loaded
        .per_type
        .get(&NotificationType::MessageReceived)
        .unwrap();
    assert!(!message_pref.enabled);
    assert_eq!(message_pref.channels, vec![NotificationChannel::Push]);

    assert!(loaded.quiet_hours.enabled);
    assert_eq!(loaded.quiet_hours.start, "21:30");
    assert_eq!(loaded.quiet_hours.end, "06:30");
    assert_eq!(loaded.quiet_hours.timezone, "Africa/Nairobi");
    assert!(!loaded.quiet_hours.except_critical);
}
