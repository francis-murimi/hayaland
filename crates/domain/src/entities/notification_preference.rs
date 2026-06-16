use crate::entities::{NotificationChannel, NotificationType};
use std::collections::HashMap;
use time::OffsetDateTime;
use uuid::Uuid;

/// Per-user preferences controlling how notifications are delivered.
#[derive(Debug, Clone)]
pub struct NotificationPreference {
    pub user_id: Uuid,
    pub channels: ChannelPreferences,
    pub per_type: HashMap<NotificationType, TypePreference>,
    pub quiet_hours: QuietHours,
    pub updated_at: OffsetDateTime,
}

impl NotificationPreference {
    pub fn new(user_id: Uuid) -> Self {
        Self {
            user_id,
            channels: ChannelPreferences::default(),
            per_type: Self::default_per_type(),
            quiet_hours: QuietHours::default(),
            updated_at: OffsetDateTime::now_utc(),
        }
    }

    pub fn channel_enabled(&self, channel: NotificationChannel) -> bool {
        match channel {
            NotificationChannel::InApp => self.channels.in_app,
            NotificationChannel::Email => self.channels.email,
            NotificationChannel::Push => self.channels.push,
            NotificationChannel::Sms => self.channels.sms,
            NotificationChannel::Webhook => false,
        }
    }

    pub fn type_enabled(&self, notification_type: NotificationType) -> bool {
        self.per_type
            .get(&notification_type)
            .map(|p| p.enabled)
            .unwrap_or(true)
    }

    pub fn channels_for_type(
        &self,
        notification_type: NotificationType,
    ) -> Vec<NotificationChannel> {
        self.per_type
            .get(&notification_type)
            .map(|p| p.channels.clone())
            .unwrap_or_else(|| {
                NotificationChannel::all()
                    .into_iter()
                    .filter(|c| self.channel_enabled(*c))
                    .collect()
            })
    }

    pub fn is_quiet_hours(&self, now: time::OffsetDateTime) -> bool {
        if !self.quiet_hours.enabled {
            return false;
        }
        let start = parse_time(&self.quiet_hours.start);
        let end = parse_time(&self.quiet_hours.end);
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

    fn default_per_type() -> HashMap<NotificationType, TypePreference> {
        let mut map = HashMap::new();
        map.insert(
            NotificationType::SecurityAlert,
            TypePreference {
                enabled: true,
                channels: vec![
                    NotificationChannel::InApp,
                    NotificationChannel::Email,
                    NotificationChannel::Push,
                    NotificationChannel::Sms,
                ],
            },
        );
        map.insert(
            NotificationType::DisputeOpened,
            TypePreference {
                enabled: true,
                channels: vec![
                    NotificationChannel::InApp,
                    NotificationChannel::Email,
                    NotificationChannel::Push,
                ],
            },
        );
        map.insert(
            NotificationType::DealDisputed,
            TypePreference {
                enabled: true,
                channels: vec![
                    NotificationChannel::InApp,
                    NotificationChannel::Email,
                    NotificationChannel::Push,
                ],
            },
        );
        map
    }
}

fn parse_time(value: &str) -> Option<(u8, u8)> {
    let mut parts = value.split(':');
    let hour = parts.next()?.parse::<u8>().ok()?;
    let minute = parts.next()?.parse::<u8>().ok()?;
    Some((hour, minute))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChannelPreferences {
    pub in_app: bool,
    pub email: bool,
    pub push: bool,
    pub sms: bool,
}

impl Default for ChannelPreferences {
    fn default() -> Self {
        Self {
            in_app: true,
            email: true,
            push: false,
            sms: false,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypePreference {
    pub enabled: bool,
    pub channels: Vec<NotificationChannel>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QuietHours {
    pub enabled: bool,
    pub start: String,
    pub end: String,
    pub timezone: String,
    pub except_critical: bool,
}

impl Default for QuietHours {
    fn default() -> Self {
        Self {
            enabled: false,
            start: "22:00".to_string(),
            end: "07:00".to_string(),
            timezone: "UTC".to_string(),
            except_critical: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_preferences_enable_in_app_and_email() {
        let prefs = NotificationPreference::new(Uuid::now_v7());
        assert!(prefs.channel_enabled(NotificationChannel::InApp));
        assert!(prefs.channel_enabled(NotificationChannel::Email));
        assert!(!prefs.channel_enabled(NotificationChannel::Push));
        assert!(!prefs.channel_enabled(NotificationChannel::Sms));
    }

    #[test]
    fn quiet_hours_detected() {
        let mut prefs = NotificationPreference::new(Uuid::now_v7());
        prefs.quiet_hours.enabled = true;
        prefs.quiet_hours.start = "22:00".to_string();
        prefs.quiet_hours.end = "07:00".to_string();

        let t = OffsetDateTime::new_utc(
            time::Date::from_calendar_date(2026, time::Month::January, 1).unwrap(),
            time::Time::from_hms(23, 0, 0).unwrap(),
        );
        assert!(prefs.is_quiet_hours(t));

        let t = OffsetDateTime::new_utc(
            time::Date::from_calendar_date(2026, time::Month::January, 1).unwrap(),
            time::Time::from_hms(10, 0, 0).unwrap(),
        );
        assert!(!prefs.is_quiet_hours(t));
    }

    #[test]
    fn quiet_hours_wrapped_range() {
        let mut prefs = NotificationPreference::new(Uuid::now_v7());
        prefs.quiet_hours.enabled = true;
        prefs.quiet_hours.start = "23:00".to_string();
        prefs.quiet_hours.end = "05:00".to_string();

        assert!(prefs.is_quiet_hours(OffsetDateTime::new_utc(
            time::Date::from_calendar_date(2026, time::Month::January, 1).unwrap(),
            time::Time::from_hms(0, 30, 0).unwrap(),
        )));
        assert!(!prefs.is_quiet_hours(OffsetDateTime::new_utc(
            time::Date::from_calendar_date(2026, time::Month::January, 1).unwrap(),
            time::Time::from_hms(12, 0, 0).unwrap(),
        )));
    }

    #[test]
    fn quiet_hours_disabled_returns_false() {
        let mut prefs = NotificationPreference::new(Uuid::now_v7());
        prefs.quiet_hours.enabled = false;
        prefs.quiet_hours.start = "22:00".to_string();
        prefs.quiet_hours.end = "07:00".to_string();

        assert!(!prefs.is_quiet_hours(OffsetDateTime::new_utc(
            time::Date::from_calendar_date(2026, time::Month::January, 1).unwrap(),
            time::Time::from_hms(23, 0, 0).unwrap(),
        )));
    }

    #[test]
    fn quiet_hours_invalid_time_returns_false() {
        let mut prefs = NotificationPreference::new(Uuid::now_v7());
        prefs.quiet_hours.enabled = true;
        prefs.quiet_hours.start = "not-a-time".to_string();
        prefs.quiet_hours.end = "07:00".to_string();

        assert!(!prefs.is_quiet_hours(OffsetDateTime::new_utc(
            time::Date::from_calendar_date(2026, time::Month::January, 1).unwrap(),
            time::Time::from_hms(23, 0, 0).unwrap(),
        )));
    }

    #[test]
    fn type_enabled_defaults_to_true() {
        let prefs = NotificationPreference::new(Uuid::now_v7());
        assert!(prefs.type_enabled(NotificationType::DealInvite));
    }

    #[test]
    fn channels_for_type_uses_global_when_no_override() {
        let prefs = NotificationPreference::new(Uuid::now_v7());
        let channels = prefs.channels_for_type(NotificationType::DealInvite);
        assert!(channels.contains(&NotificationChannel::InApp));
        assert!(channels.contains(&NotificationChannel::Email));
        assert!(!channels.contains(&NotificationChannel::Sms));
    }

    #[test]
    fn channels_for_type_uses_override() {
        let mut prefs = NotificationPreference::new(Uuid::now_v7());
        prefs.per_type.insert(
            NotificationType::DealInvite,
            TypePreference {
                enabled: true,
                channels: vec![NotificationChannel::Push],
            },
        );
        let channels = prefs.channels_for_type(NotificationType::DealInvite);
        assert_eq!(channels, vec![NotificationChannel::Push]);
    }

    #[test]
    fn webhook_channel_is_always_disabled() {
        let prefs = NotificationPreference::new(Uuid::now_v7());
        assert!(!prefs.channel_enabled(NotificationChannel::Webhook));
    }
}
