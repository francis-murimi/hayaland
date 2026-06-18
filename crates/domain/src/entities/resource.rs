use crate::errors::DomainError;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use super::GeoPoint;

/// Condition of a physical resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResourceCondition {
    New,
    Good,
    Fair,
    Poor,
    Variable,
}

impl ResourceCondition {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceCondition::New => "NEW",
            ResourceCondition::Good => "GOOD",
            ResourceCondition::Fair => "FAIR",
            ResourceCondition::Poor => "POOR",
            ResourceCondition::Variable => "VARIABLE",
        }
    }
}

impl TryFrom<&str> for ResourceCondition {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "NEW" => Ok(ResourceCondition::New),
            "GOOD" => Ok(ResourceCondition::Good),
            "FAIR" => Ok(ResourceCondition::Fair),
            "POOR" => Ok(ResourceCondition::Poor),
            "VARIABLE" => Ok(ResourceCondition::Variable),
            _ => Err(DomainError::InvalidResourceCondition {
                message: format!("unknown resource condition: {value}"),
            }),
        }
    }
}

/// A supplier-owned catalogue entry describing an underutilized resource.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resource {
    pub id: Uuid,
    pub deal_id: Option<Uuid>,
    pub catalog_item_id: Option<Uuid>,
    pub supplier_party_id: Uuid,
    pub resource_type_id: Uuid,
    pub resource_name: String,
    pub description: Option<String>,
    pub quantity: Decimal,
    pub quantity_unit: String,
    pub condition: Option<ResourceCondition>,
    pub location: Option<GeoPoint>,
    pub location_address: Option<serde_json::Value>,
    pub availability_start: Option<Date>,
    pub availability_end: Option<Date>,
    pub document_urls: Vec<String>,
    pub opportunity_cost: Option<Decimal>,
    pub verified_by_platform: bool,
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

impl Resource {
    pub fn new(
        id: Uuid,
        supplier_party_id: Uuid,
        resource_type_id: Uuid,
        resource_name: String,
        quantity: Decimal,
        quantity_unit: String,
    ) -> Result<Self, DomainError> {
        Self::validate_name(&resource_name)?;
        Self::validate_quantity(quantity)?;
        Self::validate_unit(&quantity_unit)?;
        let now = OffsetDateTime::now_utc();
        Ok(Self {
            id,
            deal_id: None,
            catalog_item_id: None,
            supplier_party_id,
            resource_type_id,
            resource_name,
            description: None,
            quantity,
            quantity_unit,
            condition: None,
            location: None,
            location_address: None,
            availability_start: None,
            availability_end: None,
            document_urls: Vec::new(),
            opportunity_cost: None,
            verified_by_platform: false,
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
        self.quantity = quantity;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    pub fn set_name(&mut self, name: String) -> Result<(), DomainError> {
        Self::validate_name(&name)?;
        self.resource_name = name;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    pub fn set_condition(&mut self, condition: Option<ResourceCondition>) {
        self.condition = condition;
        self.updated_at = OffsetDateTime::now_utc();
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
        is_admin || self.supplier_party_id == party_id
    }

    pub fn is_visible_to(&self, party_id: Option<Uuid>, is_admin: bool) -> bool {
        if is_admin {
            return true;
        }
        if self.platform_hidden || !self.is_active {
            return party_id == Some(self.supplier_party_id);
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

    fn validate_name(name: &str) -> Result<(), DomainError> {
        let trimmed = name.trim();
        let len = trimmed.chars().count();
        if !(3..=200).contains(&len) {
            return Err(DomainError::Validation(vec![
                "resource name must be between 3 and 200 characters".to_string(),
            ]));
        }
        Ok(())
    }

    fn validate_quantity(quantity: Decimal) -> Result<(), DomainError> {
        if quantity < Decimal::ZERO {
            return Err(DomainError::Validation(vec![
                "quantity cannot be negative".to_string()
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

    fn sample_resource() -> Resource {
        Resource::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            "Irrigated Farmland".to_string(),
            Decimal::from(10),
            "acres".to_string(),
        )
        .unwrap()
    }

    #[test]
    fn resource_starts_as_catalogue_entry() {
        let r = sample_resource();
        assert!(r.is_catalogue_entry());
        assert!(!r.is_deal_bound());
        assert!(r.is_active);
        assert!(!r.platform_hidden);
    }

    #[test]
    fn resource_name_rejects_too_short() {
        assert!(Resource::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            "ab".to_string(),
            Decimal::ONE,
            "unit".to_string(),
        )
        .is_err());
    }

    #[test]
    fn resource_name_rejects_too_long() {
        assert!(Resource::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            "a".repeat(201),
            Decimal::ONE,
            "unit".to_string(),
        )
        .is_err());
    }

    #[test]
    fn quantity_rejects_negative() {
        let mut r = sample_resource();
        assert!(r.set_quantity(Decimal::NEGATIVE_ONE).is_err());
    }

    #[test]
    fn quantity_accepts_zero() {
        let mut r = sample_resource();
        assert!(r.set_quantity(Decimal::ZERO).is_ok());
        assert_eq!(r.quantity, Decimal::ZERO);
    }

    #[test]
    fn condition_round_trip() {
        assert_eq!(
            ResourceCondition::try_from("GOOD").unwrap(),
            ResourceCondition::Good
        );
        assert!(ResourceCondition::try_from("UNKNOWN").is_err());
    }

    #[test]
    fn visibility_for_anonymous() {
        let r = sample_resource();
        assert!(r.is_visible_to(None, false));
    }

    #[test]
    fn hidden_item_only_visible_to_owner() {
        let mut r = sample_resource();
        r.platform_hidden = true;
        assert!(!r.is_visible_to(None, false));
        assert!(r.is_visible_to(Some(r.supplier_party_id), false));
        assert!(r.is_visible_to(None, true));
    }

    #[test]
    fn inactive_item_only_visible_to_owner() {
        let mut r = sample_resource();
        r.is_active = false;
        assert!(!r.is_visible_to(None, false));
        assert!(r.is_visible_to(Some(r.supplier_party_id), false));
    }

    #[test]
    fn only_owner_or_admin_can_modify() {
        let r = sample_resource();
        let owner = r.supplier_party_id;
        let other = Uuid::now_v7();
        assert!(r.can_be_modified_by(owner, false));
        assert!(r.can_be_modified_by(other, true));
        assert!(!r.can_be_modified_by(other, false));
    }

    #[test]
    fn contact_requires_active_not_hidden_and_owner_accepts() {
        let r = sample_resource();
        assert!(r.can_contact_owner(true));

        let mut hidden = r.clone();
        hidden.platform_hidden = true;
        assert!(!hidden.can_contact_owner(true));

        let mut inactive = r.clone();
        inactive.is_active = false;
        assert!(!inactive.can_contact_owner(true));

        assert!(!r.can_contact_owner(false));
    }
}
