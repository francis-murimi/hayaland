use crate::errors::DomainError;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use super::GeoPoint;

/// Urgency of a consumer need.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NeedPriority {
    Low,
    Medium,
    High,
    Urgent,
}

impl NeedPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            NeedPriority::Low => "LOW",
            NeedPriority::Medium => "MEDIUM",
            NeedPriority::High => "HIGH",
            NeedPriority::Urgent => "URGENT",
        }
    }
}

impl TryFrom<&str> for NeedPriority {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "LOW" => Ok(NeedPriority::Low),
            "MEDIUM" => Ok(NeedPriority::Medium),
            "HIGH" => Ok(NeedPriority::High),
            "URGENT" => Ok(NeedPriority::Urgent),
            _ => Err(DomainError::InvalidNeedPriority {
                message: format!("unknown need priority: {value}"),
            }),
        }
    }
}

/// A consumer-owned catalogue entry describing a desired output or requirement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Need {
    pub id: Uuid,
    pub deal_id: Option<Uuid>,
    pub catalog_item_id: Option<Uuid>,
    pub consumer_party_id: Uuid,
    pub need_category_id: Uuid,
    pub need_description: String,
    pub required_quantity: Decimal,
    pub quantity_unit: String,
    pub quality_requirements: Option<String>,
    pub required_by_date: Option<Date>,
    pub max_budget: Option<Decimal>,
    pub budget_currency: String,
    pub estimated_fulfillment_value: Option<Decimal>,
    pub acceptable_variants: Option<String>,
    pub priority: Option<NeedPriority>,
    pub location: Option<GeoPoint>,
    pub location_address: Option<serde_json::Value>,
    pub delivery_preferences: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub is_active: bool,
    pub deal_count: i32,
    pub platform_hidden: bool,
    pub platform_featured: bool,
    pub admin_notes: Option<String>,
    pub admin_reviewed_at: Option<OffsetDateTime>,
    pub admin_reviewed_by: Option<Uuid>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl Need {
    pub fn new(
        id: Uuid,
        consumer_party_id: Uuid,
        need_category_id: Uuid,
        need_description: String,
        required_quantity: Decimal,
        quantity_unit: String,
    ) -> Result<Self, DomainError> {
        Self::validate_description(&need_description)?;
        Self::validate_quantity(required_quantity)?;
        Self::validate_unit(&quantity_unit)?;
        let now = OffsetDateTime::now_utc();
        Ok(Self {
            id,
            deal_id: None,
            catalog_item_id: None,
            consumer_party_id,
            need_category_id,
            need_description,
            required_quantity,
            quantity_unit,
            quality_requirements: None,
            required_by_date: None,
            max_budget: None,
            budget_currency: "POINTS".to_string(),
            estimated_fulfillment_value: None,
            acceptable_variants: None,
            priority: None,
            location: None,
            location_address: None,
            delivery_preferences: None,
            metadata: None,
            is_active: true,
            deal_count: 0,
            platform_hidden: false,
            platform_featured: false,
            admin_notes: None,
            admin_reviewed_at: None,
            admin_reviewed_by: None,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn set_quantity(&mut self, quantity: Decimal) -> Result<(), DomainError> {
        Self::validate_quantity(quantity)?;
        self.required_quantity = quantity;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    pub fn set_description(&mut self, description: String) -> Result<(), DomainError> {
        Self::validate_description(&description)?;
        self.need_description = description;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    pub fn set_location(&mut self, location: Option<GeoPoint>) {
        self.location = location;
        self.updated_at = OffsetDateTime::now_utc();
    }

    pub fn set_active(&mut self, active: bool) {
        self.is_active = active;
        self.updated_at = OffsetDateTime::now_utc();
    }

    pub fn can_be_modified_by(&self, party_id: Uuid, is_admin: bool) -> bool {
        is_admin || self.consumer_party_id == party_id
    }

    pub fn is_visible_to(&self, party_id: Option<Uuid>, is_admin: bool) -> bool {
        if is_admin {
            return true;
        }
        if self.platform_hidden || !self.is_active {
            return party_id == Some(self.consumer_party_id);
        }
        true
    }

    pub fn can_contact_owner(&self, owner_accepts_inquiries: bool) -> bool {
        owner_accepts_inquiries && self.is_active && !self.platform_hidden
    }

    pub fn is_catalogue_entry(&self) -> bool {
        self.deal_id.is_none()
    }

    pub fn is_deal_bound(&self) -> bool {
        self.deal_id.is_some()
    }

    fn validate_description(description: &str) -> Result<(), DomainError> {
        let trimmed = description.trim();
        let len = trimmed.chars().count();
        if !(10..=1000).contains(&len) {
            return Err(DomainError::Validation(vec![
                "need description must be between 10 and 1000 characters".to_string(),
            ]));
        }
        Ok(())
    }

    fn validate_quantity(quantity: Decimal) -> Result<(), DomainError> {
        if quantity < Decimal::ZERO {
            return Err(DomainError::Validation(vec![
                "required quantity cannot be negative".to_string(),
            ]));
        }
        Ok(())
    }

    fn validate_unit(unit: &str) -> Result<(), DomainError> {
        let trimmed = unit.trim();
        if trimmed.is_empty() || trimmed.chars().count() > 50 {
            return Err(DomainError::Validation(vec![
                "quantity unit must be between 1 and 50 characters".to_string(),
            ]));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_need() -> Need {
        Need::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            "I need organic produce for my store.".to_string(),
            Decimal::from(1000),
            "lbs".to_string(),
        )
        .unwrap()
    }

    #[test]
    fn need_starts_as_catalogue_entry() {
        let n = sample_need();
        assert!(n.is_catalogue_entry());
        assert!(!n.is_deal_bound());
        assert!(n.is_active);
        assert_eq!(n.budget_currency, "POINTS");
    }

    #[test]
    fn description_rejects_too_short() {
        assert!(Need::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            "short".to_string(),
            Decimal::ONE,
            "unit".to_string(),
        )
        .is_err());
    }

    #[test]
    fn quantity_rejects_negative() {
        let mut n = sample_need();
        assert!(n.set_quantity(Decimal::NEGATIVE_ONE).is_err());
    }

    #[test]
    fn priority_round_trip() {
        assert_eq!(NeedPriority::try_from("HIGH").unwrap(), NeedPriority::High);
        assert!(NeedPriority::try_from("UNKNOWN").is_err());
    }

    #[test]
    fn hidden_need_only_visible_to_owner() {
        let mut n = sample_need();
        n.platform_hidden = true;
        assert!(!n.is_visible_to(None, false));
        assert!(n.is_visible_to(Some(n.consumer_party_id), false));
        assert!(n.is_visible_to(None, true));
    }

    #[test]
    fn only_owner_or_admin_can_modify() {
        let n = sample_need();
        let owner = n.consumer_party_id;
        let other = Uuid::now_v7();
        assert!(n.can_be_modified_by(owner, false));
        assert!(n.can_be_modified_by(other, true));
        assert!(!n.can_be_modified_by(other, false));
    }
}
