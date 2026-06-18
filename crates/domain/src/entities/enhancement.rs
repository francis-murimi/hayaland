use crate::errors::DomainError;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// An enhancer-owned catalogue entry describing an enabling service or input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Enhancement {
    pub id: Uuid,
    pub deal_id: Option<Uuid>,
    pub catalog_item_id: Option<Uuid>,
    pub enhancer_party_id: Uuid,
    pub enhancement_type_id: Uuid,
    pub enhancement_name: String,
    pub description: Option<String>,
    pub input_quantity: Option<Decimal>,
    pub quantity_unit: Option<String>,
    pub estimated_input_cost: Option<Decimal>,
    pub service_duration_hours: Option<Decimal>,
    pub estimated_completion_days: Option<i32>,
    pub deliverables: Option<String>,
    pub prerequisites: Option<String>,
    pub skills: Vec<String>,
    pub certifications: Option<serde_json::Value>,
    pub equipment: Vec<String>,
    pub pricing: Option<serde_json::Value>,
    pub availability: Option<serde_json::Value>,
    pub service_area: Option<serde_json::Value>,
    pub is_complete: bool,
    pub completed_at: Option<OffsetDateTime>,
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

impl Enhancement {
    pub fn new(
        id: Uuid,
        enhancer_party_id: Uuid,
        enhancement_type_id: Uuid,
        enhancement_name: String,
    ) -> Result<Self, DomainError> {
        Self::validate_name(&enhancement_name)?;
        let now = OffsetDateTime::now_utc();
        Ok(Self {
            id,
            deal_id: None,
            catalog_item_id: None,
            enhancer_party_id,
            enhancement_type_id,
            enhancement_name,
            description: None,
            input_quantity: None,
            quantity_unit: None,
            estimated_input_cost: None,
            service_duration_hours: None,
            estimated_completion_days: None,
            deliverables: None,
            prerequisites: None,
            skills: Vec::new(),
            certifications: None,
            equipment: Vec::new(),
            pricing: None,
            availability: None,
            service_area: None,
            is_complete: false,
            completed_at: None,
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

    pub fn set_name(&mut self, name: String) -> Result<(), DomainError> {
        Self::validate_name(&name)?;
        self.enhancement_name = name;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    pub fn set_input_quantity(&mut self, quantity: Option<Decimal>) -> Result<(), DomainError> {
        if let Some(q) = quantity {
            if q < Decimal::ZERO {
                return Err(DomainError::Validation(vec![
                    "input quantity cannot be negative".to_string(),
                ]));
            }
        }
        self.input_quantity = quantity;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    pub fn set_service_duration_hours(
        &mut self,
        hours: Option<Decimal>,
    ) -> Result<(), DomainError> {
        if let Some(h) = hours {
            if h < Decimal::ZERO {
                return Err(DomainError::Validation(vec![
                    "service duration hours cannot be negative".to_string(),
                ]));
            }
        }
        self.service_duration_hours = hours;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    pub fn set_completion_days(&mut self, days: Option<i32>) -> Result<(), DomainError> {
        if let Some(d) = days {
            if d < 1 {
                return Err(DomainError::Validation(vec![
                    "estimated completion days must be at least 1".to_string(),
                ]));
            }
        }
        self.estimated_completion_days = days;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    pub fn mark_complete(&mut self) {
        self.is_complete = true;
        self.completed_at = Some(OffsetDateTime::now_utc());
        self.updated_at = OffsetDateTime::now_utc();
    }

    pub fn set_active(&mut self, active: bool) {
        self.is_active = active;
        self.updated_at = OffsetDateTime::now_utc();
    }

    pub fn can_be_modified_by(&self, party_id: Uuid, is_admin: bool) -> bool {
        is_admin || self.enhancer_party_id == party_id
    }

    pub fn is_visible_to(&self, party_id: Option<Uuid>, is_admin: bool) -> bool {
        if is_admin {
            return true;
        }
        if self.platform_hidden || !self.is_active {
            return party_id == Some(self.enhancer_party_id);
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
                "enhancement name must be between 3 and 200 characters".to_string(),
            ]));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_enhancement() -> Enhancement {
        Enhancement::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            "Full Season Agricultural Support".to_string(),
        )
        .unwrap()
    }

    #[test]
    fn enhancement_starts_as_catalogue_entry() {
        let e = sample_enhancement();
        assert!(e.is_catalogue_entry());
        assert!(!e.is_deal_bound());
        assert!(e.is_active);
        assert!(!e.is_complete);
    }

    #[test]
    fn name_rejects_too_short() {
        assert!(Enhancement::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            "ab".to_string(),
        )
        .is_err());
    }

    #[test]
    fn input_quantity_rejects_negative() {
        let mut e = sample_enhancement();
        assert!(e.set_input_quantity(Some(Decimal::NEGATIVE_ONE)).is_err());
    }

    #[test]
    fn completion_days_rejects_zero() {
        let mut e = sample_enhancement();
        assert!(e.set_completion_days(Some(0)).is_err());
    }

    #[test]
    fn mark_complete_sets_flags() {
        let mut e = sample_enhancement();
        e.mark_complete();
        assert!(e.is_complete);
        assert!(e.completed_at.is_some());
    }

    #[test]
    fn hidden_enhancement_only_visible_to_owner() {
        let mut e = sample_enhancement();
        e.platform_hidden = true;
        assert!(!e.is_visible_to(None, false));
        assert!(e.is_visible_to(Some(e.enhancer_party_id), false));
        assert!(e.is_visible_to(None, true));
    }

    #[test]
    fn only_owner_or_admin_can_modify() {
        let e = sample_enhancement();
        let owner = e.enhancer_party_id;
        let other = Uuid::now_v7();
        assert!(e.can_be_modified_by(owner, false));
        assert!(e.can_be_modified_by(other, true));
        assert!(!e.can_be_modified_by(other, false));
    }
}
