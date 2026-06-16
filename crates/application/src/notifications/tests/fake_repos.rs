#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::email::queue::{EmailQueue, EmailQueueItem};
use crate::ports::{
    NotificationEvent, NotificationRealtimePublisher, PushNotificationSender, PushResult, SmsSender,
};
use domain::entities::{
    notification_preference::{ChannelPreferences, NotificationPreference, TypePreference},
    Deal, DealParticipation, DealRole, DealStatus, Email, Notification, NotificationAction,
    NotificationChannel, NotificationPriority, NotificationStatus, NotificationTemplate,
    NotificationType, Party, PartyType, PasswordHash, User, UserPartyMembership, Username,
};
use domain::errors::DomainError;
use domain::repositories::{
    DealAggregate, DealListResult, DealRepository, DealSearchCriteria, DeliveryResult,
    NotificationFilters, NotificationListResult, NotificationPreferenceRepository,
    NotificationRepository, NotificationTemplateRepository, Pagination, PartyRepository,
    PartySearchCriteria, UserRepository,
};

// ---------------------------------------------------------------------------
// Notification repository
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
pub struct FakeNotificationRepo {
    pub notifications: Arc<Mutex<Vec<Notification>>>,
}

impl FakeNotificationRepo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(notification: Notification) -> Self {
        let s = Self::new();
        s.notifications.lock().unwrap().push(notification);
        s
    }

    fn matches_recipient(n: &Notification, user_id: Option<Uuid>, party_id: Option<Uuid>) -> bool {
        user_id.is_some_and(|uid| n.user_id == Some(uid))
            || party_id.is_some_and(|pid| n.party_id == Some(pid))
    }
}

#[async_trait]
impl NotificationRepository for FakeNotificationRepo {
    async fn create(&self, notification: &Notification) -> Result<(), DomainError> {
        self.notifications
            .lock()
            .unwrap()
            .push(notification.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Notification>, DomainError> {
        Ok(self
            .notifications
            .lock()
            .unwrap()
            .iter()
            .find(|n| n.id == id)
            .cloned())
    }

    async fn list_for_recipient(
        &self,
        user_id: Option<Uuid>,
        party_id: Option<Uuid>,
        filters: NotificationFilters,
        pagination: Pagination,
    ) -> Result<NotificationListResult, DomainError> {
        let all = self.notifications.lock().unwrap().clone();
        let filtered: Vec<_> = all
            .into_iter()
            .filter(|n| Self::matches_recipient(n, user_id, party_id))
            .filter(|n| {
                filters
                    .notification_type
                    .map_or(true, |t| n.notification_type == t)
                    && filters.is_read.map_or(true, |r| (n.read_at.is_some()) == r)
                    && filters
                        .is_actioned
                        .map_or(true, |a| (n.actioned_at.is_some()) == a)
                    && filters.priority.map_or(true, |p| n.priority == p)
            })
            .collect();

        let total = filtered.len() as i64;
        let unread_count = filtered.iter().filter(|n| n.read_at.is_none()).count() as i64;

        let items: Vec<_> = filtered
            .into_iter()
            .skip(pagination.offset.max(0) as usize)
            .take(pagination.limit.max(1) as usize)
            .collect();

        Ok(NotificationListResult {
            items,
            total,
            unread_count,
        })
    }

    async fn count_unread_for_recipient(
        &self,
        user_id: Option<Uuid>,
        party_id: Option<Uuid>,
    ) -> Result<i64, DomainError> {
        let count = self
            .notifications
            .lock()
            .unwrap()
            .iter()
            .filter(|n| n.read_at.is_none() && Self::matches_recipient(n, user_id, party_id))
            .count() as i64;
        Ok(count)
    }

    async fn mark_read(
        &self,
        id: Uuid,
        user_id: Uuid,
        party_id: Option<Uuid>,
        read_at: OffsetDateTime,
    ) -> Result<bool, DomainError> {
        let mut guard = self.notifications.lock().unwrap();
        let Some(n) = guard.iter_mut().find(|n| n.id == id) else {
            return Ok(false);
        };
        let owns =
            n.user_id == Some(user_id) || n.party_id.is_some_and(|pid| party_id == Some(pid));
        if !owns {
            return Ok(false);
        }
        if n.read_at.is_some() {
            return Ok(false);
        }
        n.read_at = Some(read_at);
        n.updated_at = read_at;
        if n.status == NotificationStatus::Sent {
            n.status = NotificationStatus::Delivered;
        }
        Ok(true)
    }

    async fn mark_all_read(
        &self,
        user_id: Option<Uuid>,
        party_id: Option<Uuid>,
        before: Option<OffsetDateTime>,
        notification_type: Option<NotificationType>,
    ) -> Result<u64, DomainError> {
        let mut guard = self.notifications.lock().unwrap();
        let mut count = 0u64;
        for n in guard.iter_mut() {
            if n.read_at.is_some() {
                continue;
            }
            if !Self::matches_recipient(n, user_id, party_id) {
                continue;
            }
            if before.is_some_and(|b| n.created_at >= b) {
                continue;
            }
            if notification_type.is_some_and(|t| n.notification_type != t) {
                continue;
            }
            let now = OffsetDateTime::now_utc();
            n.read_at = Some(now);
            n.updated_at = now;
            if n.status == NotificationStatus::Sent {
                n.status = NotificationStatus::Delivered;
            }
            count += 1;
        }
        Ok(count)
    }

    async fn mark_actioned(
        &self,
        id: Uuid,
        user_id: Uuid,
        party_id: Option<Uuid>,
        actioned_at: OffsetDateTime,
    ) -> Result<bool, DomainError> {
        let mut guard = self.notifications.lock().unwrap();
        let Some(n) = guard.iter_mut().find(|n| n.id == id) else {
            return Ok(false);
        };
        let owns =
            n.user_id == Some(user_id) || n.party_id.is_some_and(|pid| party_id == Some(pid));
        if !owns {
            return Ok(false);
        }
        n.actioned_at = Some(actioned_at);
        n.updated_at = actioned_at;
        Ok(true)
    }

    async fn delete(
        &self,
        id: Uuid,
        user_id: Uuid,
        party_id: Option<Uuid>,
    ) -> Result<bool, DomainError> {
        let mut guard = self.notifications.lock().unwrap();
        let pos = guard.iter().position(|n| {
            n.id == id
                && (n.user_id == Some(user_id)
                    || n.party_id.is_some_and(|pid| party_id == Some(pid)))
        });
        if let Some(idx) = pos {
            guard.remove(idx);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn update_status(&self, id: Uuid, status: NotificationStatus) -> Result<(), DomainError> {
        let mut guard = self.notifications.lock().unwrap();
        if let Some(n) = guard.iter_mut().find(|n| n.id == id) {
            n.status = status;
        }
        Ok(())
    }

    async fn record_delivery(
        &self,
        _notification_id: Uuid,
        _channel: NotificationChannel,
        _result: DeliveryResult,
    ) -> Result<(), DomainError> {
        Ok(())
    }

    async fn list_pending(
        &self,
        batch_size: usize,
        _older_than: Option<OffsetDateTime>,
    ) -> Result<Vec<Notification>, DomainError> {
        let guard = self.notifications.lock().unwrap();
        Ok(guard
            .iter()
            .filter(|n| n.status == NotificationStatus::Pending)
            .take(batch_size)
            .cloned()
            .collect())
    }
}

// ---------------------------------------------------------------------------
// Notification preference repository
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
pub struct FakeNotificationPreferenceRepo {
    pub prefs: Arc<Mutex<HashMap<Uuid, NotificationPreference>>>,
}

impl FakeNotificationPreferenceRepo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(&self, preference: NotificationPreference) {
        self.prefs
            .lock()
            .unwrap()
            .insert(preference.user_id, preference);
    }
}

#[async_trait]
impl NotificationPreferenceRepository for FakeNotificationPreferenceRepo {
    async fn get(&self, user_id: Uuid) -> Result<NotificationPreference, DomainError> {
        Ok(self
            .prefs
            .lock()
            .unwrap()
            .get(&user_id)
            .cloned()
            .unwrap_or_else(|| NotificationPreference::new(user_id)))
    }

    async fn save(&self, preference: &NotificationPreference) -> Result<(), DomainError> {
        self.prefs
            .lock()
            .unwrap()
            .insert(preference.user_id, preference.clone());
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Notification template repository
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
pub struct FakeNotificationTemplateRepo {
    pub templates: Arc<Mutex<Vec<NotificationTemplate>>>,
}

impl FakeNotificationTemplateRepo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(&self, template: NotificationTemplate) {
        self.templates.lock().unwrap().push(template);
    }

    fn name_exists(&self, name: &str, exclude_id: Option<Uuid>) -> bool {
        self.templates
            .lock()
            .unwrap()
            .iter()
            .any(|t| t.name == name && Some(t.id) != exclude_id)
    }
}

#[async_trait]
impl NotificationTemplateRepository for FakeNotificationTemplateRepo {
    async fn create(&self, template: &NotificationTemplate) -> Result<(), DomainError> {
        if self.name_exists(&template.name, None) {
            return Err(DomainError::DuplicateNotificationTemplate);
        }
        self.templates.lock().unwrap().push(template.clone());
        Ok(())
    }

    async fn update(&self, template: &NotificationTemplate) -> Result<(), DomainError> {
        if self.name_exists(&template.name, Some(template.id)) {
            return Err(DomainError::DuplicateNotificationTemplate);
        }
        let mut guard = self.templates.lock().unwrap();
        if let Some(idx) = guard.iter().position(|t| t.id == template.id) {
            guard[idx] = template.clone();
        }
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<NotificationTemplate>, DomainError> {
        Ok(self
            .templates
            .lock()
            .unwrap()
            .iter()
            .find(|t| t.id == id)
            .cloned())
    }

    async fn find_active(
        &self,
        notification_type: NotificationType,
        channel: NotificationChannel,
        locale: &str,
    ) -> Result<Option<NotificationTemplate>, DomainError> {
        let guard = self.templates.lock().unwrap();
        let find = |loc: &str| {
            guard.iter().find(|t| {
                t.notification_type == notification_type
                    && t.channel == channel
                    && t.locale == loc
                    && t.is_active
            })
        };
        Ok(find(locale)
            .or_else(|| if locale != "en" { find("en") } else { None })
            .cloned())
    }

    async fn list(&self, pagination: Pagination) -> Result<Vec<NotificationTemplate>, DomainError> {
        let guard = self.templates.lock().unwrap();
        Ok(guard
            .iter()
            .skip(pagination.offset.max(0) as usize)
            .take(pagination.limit.max(1) as usize)
            .cloned()
            .collect())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let mut guard = self.templates.lock().unwrap();
        if let Some(idx) = guard.iter().position(|t| t.id == id) {
            guard.remove(idx);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// User repository
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
pub struct FakeUserRepo {
    pub users: Arc<Mutex<Vec<User>>>,
}

impl FakeUserRepo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(&self, user: User) {
        self.users.lock().unwrap().push(user);
    }
}

#[async_trait]
impl UserRepository for FakeUserRepo {
    async fn create(&self, user: &User) -> Result<(), DomainError> {
        self.users.lock().unwrap().push(user.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, DomainError> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.id == id)
            .cloned())
    }

    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, DomainError> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.email.as_str() == email.as_str())
            .cloned())
    }

    async fn find_by_username(&self, username: &Username) -> Result<Option<User>, DomainError> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.username.as_str() == username.as_str())
            .cloned())
    }

    async fn list(
        &self,
        limit: i64,
        offset: i64,
        active_only: Option<bool>,
    ) -> Result<Vec<User>, DomainError> {
        let guard = self.users.lock().unwrap();
        Ok(guard
            .iter()
            .filter(|u| active_only.map_or(true, |active| u.is_active == active))
            .skip(offset.max(0) as usize)
            .take(limit.max(0) as usize)
            .cloned()
            .collect())
    }

    async fn update(&self, user: &User) -> Result<(), DomainError> {
        let mut guard = self.users.lock().unwrap();
        if let Some(idx) = guard.iter().position(|u| u.id == user.id) {
            guard[idx] = user.clone();
        }
        Ok(())
    }

    async fn count(&self) -> Result<i64, DomainError> {
        Ok(self.users.lock().unwrap().len() as i64)
    }
}

// ---------------------------------------------------------------------------
// Party repository
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
pub struct FakePartyRepo {
    pub parties: Arc<Mutex<Vec<Party>>>,
    pub memberships: Arc<Mutex<Vec<UserPartyMembership>>>,
}

impl FakePartyRepo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_party(&self, party: Party) {
        self.parties.lock().unwrap().push(party);
    }

    pub fn with_membership(&self, membership: UserPartyMembership) {
        self.memberships.lock().unwrap().push(membership);
    }
}

#[async_trait]
impl PartyRepository for FakePartyRepo {
    async fn create(&self, party: &Party) -> Result<(), DomainError> {
        self.parties.lock().unwrap().push(party.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Party>, DomainError> {
        Ok(self
            .parties
            .lock()
            .unwrap()
            .iter()
            .find(|p| p.id == id)
            .cloned())
    }

    async fn find_by_email(&self, email: &Email) -> Result<Option<Party>, DomainError> {
        Ok(self
            .parties
            .lock()
            .unwrap()
            .iter()
            .find(|p| p.email.as_str() == email.as_str())
            .cloned())
    }

    async fn update(&self, party: &Party) -> Result<(), DomainError> {
        let mut guard = self.parties.lock().unwrap();
        if let Some(idx) = guard.iter().position(|p| p.id == party.id) {
            guard[idx] = party.clone();
        }
        Ok(())
    }

    async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError> {
        let mut guard = self.parties.lock().unwrap();
        if let Some(p) = guard.iter_mut().find(|p| p.id == id) {
            p.is_active = false;
        }
        Ok(())
    }

    async fn list(&self, criteria: &PartySearchCriteria) -> Result<Vec<Party>, DomainError> {
        let guard = self.parties.lock().unwrap();
        Ok(guard
            .iter()
            .filter(|p| {
                criteria
                    .active_only
                    .map_or(true, |active| p.is_active == active)
            })
            .skip(criteria.offset.max(0) as usize)
            .take(criteria.limit.max(0) as usize)
            .cloned()
            .collect())
    }

    async fn count(&self, criteria: &PartySearchCriteria) -> Result<i64, DomainError> {
        let guard = self.parties.lock().unwrap();
        Ok(guard
            .iter()
            .filter(|p| {
                criteria
                    .active_only
                    .map_or(true, |active| p.is_active == active)
            })
            .count() as i64)
    }

    async fn add_role(
        &self,
        _party_id: Uuid,
        _role: DealRole,
        _profile: domain::entities::RoleProfile,
    ) -> Result<(), DomainError> {
        Ok(())
    }

    async fn remove_role(&self, _party_id: Uuid, _role: DealRole) -> Result<(), DomainError> {
        Ok(())
    }

    async fn list_roles(
        &self,
        _party_id: Uuid,
    ) -> Result<Vec<(DealRole, domain::entities::RoleProfile)>, DomainError> {
        Ok(vec![])
    }

    async fn has_role(&self, _party_id: Uuid, _role: DealRole) -> Result<bool, DomainError> {
        Ok(false)
    }

    async fn count_active_deals_for_role(
        &self,
        _party_id: Uuid,
        _role: DealRole,
    ) -> Result<i64, DomainError> {
        Ok(0)
    }

    async fn count_active_deals(&self, _party_id: Uuid) -> Result<i64, DomainError> {
        Ok(0)
    }

    async fn add_membership(&self, membership: &UserPartyMembership) -> Result<(), DomainError> {
        self.memberships.lock().unwrap().push(membership.clone());
        Ok(())
    }

    async fn list_memberships_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<(UserPartyMembership, Party)>, DomainError> {
        let parties = self.parties.lock().unwrap();
        Ok(self
            .memberships
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.user_id == user_id)
            .cloned()
            .map(|m| {
                let party = parties
                    .iter()
                    .find(|p| p.id == m.party_id)
                    .cloned()
                    .unwrap();
                (m, party)
            })
            .collect())
    }

    async fn find_membership(
        &self,
        user_id: Uuid,
        party_id: Uuid,
    ) -> Result<Option<UserPartyMembership>, DomainError> {
        Ok(self
            .memberships
            .lock()
            .unwrap()
            .iter()
            .find(|m| m.user_id == user_id && m.party_id == party_id)
            .cloned())
    }

    async fn touch(&self, id: Uuid, updated_at: OffsetDateTime) -> Result<(), DomainError> {
        let mut guard = self.parties.lock().unwrap();
        if let Some(p) = guard.iter_mut().find(|p| p.id == id) {
            p.updated_at = updated_at;
        }
        Ok(())
    }

    async fn is_user_member_of_party(
        &self,
        user_id: Uuid,
        party_id: Uuid,
    ) -> Result<bool, DomainError> {
        Ok(self
            .memberships
            .lock()
            .unwrap()
            .iter()
            .any(|m| m.user_id == user_id && m.party_id == party_id && m.is_active))
    }

    async fn list_members_for_party(
        &self,
        party_id: Uuid,
    ) -> Result<Vec<UserPartyMembership>, DomainError> {
        Ok(self
            .memberships
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.party_id == party_id && m.is_active)
            .cloned()
            .collect())
    }
}

// ---------------------------------------------------------------------------
// Deal repository
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
pub struct FakeDealRepo {
    pub aggregates: Arc<Mutex<Vec<(Deal, Vec<DealParticipation>)>>>,
}

impl FakeDealRepo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_aggregate(&self, aggregate: DealAggregate) {
        self.aggregates
            .lock()
            .unwrap()
            .push((aggregate.deal, aggregate.participations));
    }
}

#[async_trait]
impl DealRepository for FakeDealRepo {
    async fn create(&self, aggregate: &DealAggregate) -> Result<(), DomainError> {
        self.aggregates
            .lock()
            .unwrap()
            .push((aggregate.deal.clone(), aggregate.participations.clone()));
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Deal>, DomainError> {
        Ok(self
            .aggregates
            .lock()
            .unwrap()
            .iter()
            .find(|(d, _)| d.id == id)
            .map(|(d, _)| d.clone()))
    }

    async fn find_aggregate_by_id(&self, id: Uuid) -> Result<Option<DealAggregate>, DomainError> {
        Ok(self
            .aggregates
            .lock()
            .unwrap()
            .iter()
            .find(|(d, _)| d.id == id)
            .map(|(d, p)| DealAggregate {
                deal: d.clone(),
                participations: p.clone(),
            }))
    }

    async fn find_participations_by_deal(
        &self,
        deal_id: Uuid,
    ) -> Result<Vec<DealParticipation>, DomainError> {
        Ok(self
            .aggregates
            .lock()
            .unwrap()
            .iter()
            .find(|(d, _)| d.id == deal_id)
            .map(|(_, p)| p.clone())
            .unwrap_or_default())
    }

    async fn update(&self, deal: &Deal) -> Result<(), DomainError> {
        let mut guard = self.aggregates.lock().unwrap();
        if let Some((d, _)) = guard.iter_mut().find(|(d, _)| d.id == deal.id) {
            *d = deal.clone();
        }
        Ok(())
    }

    async fn update_participation(
        &self,
        participation: &DealParticipation,
    ) -> Result<(), DomainError> {
        let mut guard = self.aggregates.lock().unwrap();
        for (_, participations) in guard.iter_mut() {
            if let Some(p) = participations.iter_mut().find(|p| p.id == participation.id) {
                *p = participation.clone();
                return Ok(());
            }
        }
        Ok(())
    }

    async fn list(&self, _criteria: &DealSearchCriteria) -> Result<DealListResult, DomainError> {
        Ok(DealListResult {
            deals: vec![],
            total: 0,
            limit: 0,
            offset: 0,
        })
    }

    async fn count_active_deals_for_party(&self, _party_id: Uuid) -> Result<i64, DomainError> {
        Ok(0)
    }

    async fn count_active_deals_for_party_role(
        &self,
        _party_id: Uuid,
        _role: DealRole,
    ) -> Result<i64, DomainError> {
        Ok(0)
    }

    async fn record_history(
        &self,
        _deal_id: Uuid,
        _event_type: &str,
        _actor_party_id: Option<Uuid>,
        _details: Option<serde_json::Value>,
    ) -> Result<(), DomainError> {
        Ok(())
    }

    async fn is_party_participant(
        &self,
        _deal_id: Uuid,
        _party_id: Uuid,
    ) -> Result<bool, DomainError> {
        Ok(false)
    }

    async fn next_deal_reference(&self) -> Result<String, DomainError> {
        Ok("DL-TEST-0001".to_string())
    }

    async fn find_deals_by_status(
        &self,
        _status: DealStatus,
        _entered_before: OffsetDateTime,
        _limit: i64,
    ) -> Result<Vec<Deal>, DomainError> {
        Ok(vec![])
    }

    async fn update_value_totals(
        &self,
        _deal_id: Uuid,
        _total_value: rust_decimal::Decimal,
        _platform_fee_percentage: rust_decimal::Decimal,
        _platform_fee_amount: rust_decimal::Decimal,
    ) -> Result<(), DomainError> {
        Ok(())
    }

    async fn create_term(&self, _term: &domain::entities::Term) -> Result<(), DomainError> {
        Ok(())
    }

    async fn update_term(&self, _term: &domain::entities::Term) -> Result<(), DomainError> {
        Ok(())
    }

    async fn find_term_by_id(
        &self,
        _id: Uuid,
    ) -> Result<Option<domain::entities::Term>, DomainError> {
        Ok(None)
    }

    async fn find_terms_by_deal(
        &self,
        _deal_id: Uuid,
    ) -> Result<Vec<domain::entities::Term>, DomainError> {
        Ok(vec![])
    }

    async fn set_value_distribution(
        &self,
        _distribution: &domain::entities::ValueDistribution,
    ) -> Result<(), DomainError> {
        Ok(())
    }

    async fn find_value_distribution_by_deal(
        &self,
        _deal_id: Uuid,
    ) -> Result<Option<domain::entities::ValueDistribution>, DomainError> {
        Ok(None)
    }
}

// ---------------------------------------------------------------------------
// Email queue & notification publishers
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
pub struct FakeEmailQueue {
    pub items: Arc<Mutex<Vec<EmailQueueItem>>>,
}

impl FakeEmailQueue {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl EmailQueue for FakeEmailQueue {
    async fn enqueue(&self, item: EmailQueueItem) -> Result<(), crate::errors::ApplicationError> {
        self.items.lock().unwrap().push(item);
        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct FakeNotificationPublisher {
    pub events: Arc<Mutex<Vec<NotificationEvent>>>,
}

impl FakeNotificationPublisher {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl NotificationRealtimePublisher for FakeNotificationPublisher {
    async fn publish(
        &self,
        event: NotificationEvent,
    ) -> Result<(), crate::errors::ApplicationError> {
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct FakePushSender;

#[async_trait]
impl PushNotificationSender for FakePushSender {
    async fn send(
        &self,
        _device_tokens: &[String],
        _title: &str,
        _body: &str,
        _data: serde_json::Value,
    ) -> Result<Vec<PushResult>, crate::errors::ApplicationError> {
        Ok(vec![])
    }
}

#[derive(Debug, Default, Clone)]
pub struct FakeSmsSender;

#[async_trait]
impl SmsSender for FakeSmsSender {
    async fn send(&self, _phone: &str, _body: &str) -> Result<(), crate::errors::ApplicationError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Entity builders
// ---------------------------------------------------------------------------

pub fn test_email(value: &str) -> Email {
    Email::new(value).expect("valid test email")
}

pub fn test_username(value: &str) -> Username {
    Username::new(value).expect("valid test username")
}

pub fn test_password_hash() -> PasswordHash {
    PasswordHash::new("$argon2id$v=19$m=65536,t=3,p=4$c2FsdA$cGFzc3dvcmQ".to_string())
        .expect("valid test hash")
}

pub fn test_user(id: Uuid, email: &str, username: &str) -> User {
    User::new(
        id,
        test_email(email),
        test_username(username),
        test_password_hash(),
    )
}

pub fn test_party(id: Uuid, email: &str, display_name: &str) -> Party {
    Party::new(
        id,
        PartyType::Organization,
        domain::entities::DisplayName::new(display_name).expect("valid display name"),
        test_email(email),
    )
}

pub fn test_membership(id: Uuid, user_id: Uuid, party_id: Uuid) -> UserPartyMembership {
    UserPartyMembership::new(
        id,
        user_id,
        party_id,
        domain::entities::PartyMembershipRole::Member,
    )
}

pub fn test_deal(id: Uuid, initiator_party_id: Uuid) -> Deal {
    Deal::new(
        id,
        format!("DL-{}", id),
        domain::entities::DealTitle::new("Test Deal").expect("valid deal title"),
        Uuid::now_v7(),
        initiator_party_id,
        DealRole::Supplier,
    )
}

pub fn test_deal_aggregate(deal: Deal, party_ids: &[Uuid]) -> DealAggregate {
    let participations = party_ids
        .iter()
        .enumerate()
        .map(|(i, &party_id)| {
            DealParticipation::new(
                Uuid::now_v7(),
                deal.id,
                party_id,
                if i == 0 {
                    DealRole::Supplier
                } else {
                    DealRole::Consumer
                },
                i == 0,
            )
        })
        .collect();
    DealAggregate {
        deal,
        participations,
    }
}

pub fn user_with_channels(
    user_id: Uuid,
    channels: Vec<NotificationChannel>,
) -> NotificationPreference {
    let mut prefs = NotificationPreference::new(user_id);
    prefs.channels = ChannelPreferences {
        in_app: channels.contains(&NotificationChannel::InApp),
        email: channels.contains(&NotificationChannel::Email),
        push: channels.contains(&NotificationChannel::Push),
        sms: channels.contains(&NotificationChannel::Sms),
    };
    prefs
}

pub fn user_with_disabled_type(
    user_id: Uuid,
    notification_type: NotificationType,
) -> NotificationPreference {
    let mut prefs = NotificationPreference::new(user_id);
    prefs.per_type.insert(
        notification_type,
        TypePreference {
            enabled: false,
            channels: vec![NotificationChannel::InApp],
        },
    );
    prefs
}

pub fn quiet_hours_covering_now() -> domain::entities::notification_preference::QuietHours {
    let now = OffsetDateTime::now_utc();
    let start = now - time::Duration::hours(2);
    let end = now + time::Duration::hours(2);

    let fmt = |t: OffsetDateTime| format!("{:02}:{:02}", t.hour(), t.minute());

    domain::entities::notification_preference::QuietHours {
        enabled: true,
        start: fmt(start),
        end: fmt(end),
        timezone: "UTC".to_string(),
        except_critical: false,
    }
}

pub fn test_template(
    id: Uuid,
    name: &str,
    notification_type: NotificationType,
    channel: NotificationChannel,
    locale: &str,
    subject: &str,
    body: &str,
) -> NotificationTemplate {
    NotificationTemplate::new(
        id,
        name.to_string(),
        notification_type,
        channel,
        locale.to_string(),
        subject.to_string(),
        body.to_string(),
        serde_json::Value::Null,
    )
    .expect("valid test template")
}

pub fn sample_notification(
    id: Uuid,
    user_id: Option<Uuid>,
    party_id: Option<Uuid>,
    notification_type: NotificationType,
) -> Notification {
    Notification::new(
        id,
        user_id,
        party_id,
        notification_type,
        "Sample title".to_string(),
        "Sample body".to_string(),
        NotificationPriority::Normal,
        None,
        vec![NotificationAction {
            label: "View".to_string(),
            action_type: domain::entities::ActionType::Navigate,
            url: Some("/deals/1".to_string()),
            method: None,
        }],
        None,
        None,
        serde_json::Value::Null,
        None,
    )
    .expect("valid sample notification")
}
