use domain::entities::{
    notification_preference::{NotificationPreference, QuietHours},
    NotificationChannel, NotificationPriority, NotificationType,
};
use time::OffsetDateTime;

/// Determine the delivery channels for a notification based on priority and
/// the recipient's preferences/quiet hours.
pub fn route_channels(
    notification_type: NotificationType,
    priority: NotificationPriority,
    preferences: &NotificationPreference,
    now: OffsetDateTime,
) -> Vec<NotificationChannel> {
    // Per-type disabled?
    if !preferences.type_enabled(notification_type) {
        return vec![];
    }

    // Start from per-type channel list, or fall back to globally enabled channels.
    let mut channels: Vec<NotificationChannel> = preferences.channels_for_type(notification_type);

    // Filter by global channel enablement.
    channels.retain(|c| preferences.channel_enabled(*c));

    // Quiet hours suppression (except critical if configured).
    let in_quiet_hours = preferences.is_quiet_hours(now);
    let bypass_quiet_hours =
        priority == NotificationPriority::Critical && preferences.quiet_hours.except_critical;

    if in_quiet_hours && !bypass_quiet_hours {
        // In-app is never suppressed by quiet hours.
        channels.retain(|c| *c == NotificationChannel::InApp);
    }

    // SMS only allowed for high/critical unless explicitly enabled.
    if !matches!(
        priority,
        NotificationPriority::High | NotificationPriority::Critical
    ) {
        channels.retain(|c| *c != NotificationChannel::Sms);
    }

    // Webhook is reserved/system-only in MVP.
    channels.retain(|c| *c != NotificationChannel::Webhook);

    channels
}

/// Check whether the current time is within quiet hours.
pub fn is_quiet_hours(now: OffsetDateTime, quiet_hours: &QuietHours) -> bool {
    if !quiet_hours.enabled {
        return false;
    }
    let start = parse_time(&quiet_hours.start);
    let end = parse_time(&quiet_hours.end);
    let current = (now.hour(), now.minute());
    match (start, end) {
        (Some((sh, sm)), Some((eh, em))) => {
            let after_start = current >= (sh, sm);
            let before_end = current < (eh, em);
            if (sh, sm) < (eh, em) {
                after_start && before_end
            } else {
                after_start || before_end
            }
        }
        _ => false,
    }
}

fn parse_time(value: &str) -> Option<(u8, u8)> {
    let mut parts = value.split(':');
    let hour = parts.next()?.parse::<u8>().ok()?;
    let minute = parts.next()?.parse::<u8>().ok()?;
    Some((hour, minute))
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::entities::notification_preference::ChannelPreferences;
    use uuid::Uuid;

    fn prefs_with_channels(channels: Vec<NotificationChannel>) -> NotificationPreference {
        let mut prefs = NotificationPreference::new(Uuid::now_v7());
        prefs.channels = ChannelPreferences {
            in_app: channels.contains(&NotificationChannel::InApp),
            email: channels.contains(&NotificationChannel::Email),
            push: channels.contains(&NotificationChannel::Push),
            sms: channels.contains(&NotificationChannel::Sms),
        };
        prefs
    }

    #[test]
    fn critical_bypasses_quiet_hours() {
        let mut prefs = prefs_with_channels(vec![
            NotificationChannel::InApp,
            NotificationChannel::Email,
            NotificationChannel::Push,
        ]);
        prefs.quiet_hours.enabled = true;
        prefs.quiet_hours.start = "22:00".to_string();
        prefs.quiet_hours.end = "07:00".to_string();
        prefs.quiet_hours.except_critical = true;

        let t = OffsetDateTime::new_utc(
            time::Date::from_calendar_date(2026, time::Month::January, 1).unwrap(),
            time::Time::from_hms(23, 0, 0).unwrap(),
        );

        let channels = route_channels(
            NotificationType::SecurityAlert,
            NotificationPriority::Critical,
            &prefs,
            t,
        );
        assert!(channels.contains(&NotificationChannel::Email));
        assert!(channels.contains(&NotificationChannel::Push));
    }

    #[test]
    fn normal_respects_quiet_hours() {
        let mut prefs =
            prefs_with_channels(vec![NotificationChannel::InApp, NotificationChannel::Email]);
        prefs.quiet_hours.enabled = true;
        prefs.quiet_hours.start = "22:00".to_string();
        prefs.quiet_hours.end = "07:00".to_string();

        let t = OffsetDateTime::new_utc(
            time::Date::from_calendar_date(2026, time::Month::January, 1).unwrap(),
            time::Time::from_hms(23, 0, 0).unwrap(),
        );

        let channels = route_channels(
            NotificationType::DealInvite,
            NotificationPriority::Normal,
            &prefs,
            t,
        );
        assert!(channels.contains(&NotificationChannel::InApp));
        assert!(!channels.contains(&NotificationChannel::Email));
    }

    #[test]
    fn disabled_type_returns_empty() {
        let mut prefs = prefs_with_channels(vec![NotificationChannel::InApp]);
        prefs.per_type.insert(
            NotificationType::TrustScoreUpdated,
            domain::entities::notification_preference::TypePreference {
                enabled: false,
                channels: vec![NotificationChannel::InApp],
            },
        );

        let channels = route_channels(
            NotificationType::TrustScoreUpdated,
            NotificationPriority::Low,
            &prefs,
            OffsetDateTime::now_utc(),
        );
        assert!(channels.is_empty());
    }
}
