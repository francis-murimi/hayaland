use crate::errors::DomainError;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// The function a party plays within a specific deal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DealRole {
    Supplier,
    Consumer,
    Enhancer,
}

impl DealRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            DealRole::Supplier => "SUPPLIER",
            DealRole::Consumer => "CONSUMER",
            DealRole::Enhancer => "ENHANCER",
        }
    }

    pub fn all() -> &'static [DealRole] {
        &[DealRole::Supplier, DealRole::Consumer, DealRole::Enhancer]
    }
}

impl TryFrom<&str> for DealRole {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "SUPPLIER" => Ok(DealRole::Supplier),
            "CONSUMER" => Ok(DealRole::Consumer),
            "ENHANCER" => Ok(DealRole::Enhancer),
            _ => Err(DomainError::InvalidDealRole {
                message: format!("unknown deal role: {value}"),
            }),
        }
    }
}

/// Supplier-specific profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SupplierProfile {
    pub resource_type_ids: Vec<Uuid>,
    pub typical_capacity: Option<String>,
    pub availability_schedule: Option<serde_json::Value>,
    pub preferred_compensation: Vec<String>,
    pub insurance_verified: bool,
}

/// Consumer-specific profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ConsumerProfile {
    pub need_category_ids: Vec<Uuid>,
    pub typical_volume: Option<String>,
    pub preferred_quality_standard: Option<String>,
    pub budget_range_min: Option<f64>,
    pub budget_range_max: Option<f64>,
    pub preferred_payment_terms: Vec<String>,
}

/// Enhancer-specific profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EnhancerProfile {
    pub enhancement_type_ids: Vec<Uuid>,
    pub skills: Vec<String>,
    pub certifications: Option<serde_json::Value>,
    pub hourly_rate: Option<f64>,
    pub fixed_rate: Option<f64>,
    pub equipment_owned: Vec<String>,
    pub availability: Option<serde_json::Value>,
    pub typical_engagement_duration: Option<String>,
}

/// A role assigned to a party, including role-specific profile data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartyRole {
    pub id: Uuid,
    pub party_id: Uuid,
    pub role_type: DealRole,
    pub profile: RoleProfile,
    pub is_active: bool,
    pub assigned_at: OffsetDateTime,
}

/// Union of role-specific profiles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RoleProfile {
    Supplier(SupplierProfile),
    Consumer(ConsumerProfile),
    Enhancer(EnhancerProfile),
}

impl PartyRole {
    pub fn new(id: Uuid, party_id: Uuid, role_type: DealRole, profile: RoleProfile) -> Self {
        Self {
            id,
            party_id,
            role_type,
            profile,
            is_active: true,
            assigned_at: OffsetDateTime::now_utc(),
        }
    }

    pub fn matches_role(&self, role: DealRole) -> bool {
        self.role_type == role && self.is_active
    }
}

impl RoleProfile {
    pub fn for_role(role: DealRole) -> Self {
        match role {
            DealRole::Supplier => RoleProfile::Supplier(SupplierProfile::default()),
            DealRole::Consumer => RoleProfile::Consumer(ConsumerProfile::default()),
            DealRole::Enhancer => RoleProfile::Enhancer(EnhancerProfile::default()),
        }
    }

    pub fn role_type(&self) -> DealRole {
        match self {
            RoleProfile::Supplier(_) => DealRole::Supplier,
            RoleProfile::Consumer(_) => DealRole::Consumer,
            RoleProfile::Enhancer(_) => DealRole::Enhancer,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deal_role_from_str() {
        assert_eq!(DealRole::try_from("SUPPLIER").unwrap(), DealRole::Supplier);
        assert_eq!(DealRole::try_from("CONSUMER").unwrap(), DealRole::Consumer);
        assert_eq!(DealRole::try_from("ENHANCER").unwrap(), DealRole::Enhancer);
        assert!(DealRole::try_from("UNKNOWN").is_err());
    }

    #[test]
    fn role_profile_matches_role() {
        let profile = RoleProfile::Supplier(SupplierProfile::default());
        assert_eq!(profile.role_type(), DealRole::Supplier);
    }

    #[test]
    fn party_role_matches() {
        let role = PartyRole::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            DealRole::Supplier,
            RoleProfile::Supplier(SupplierProfile::default()),
        );
        assert!(role.matches_role(DealRole::Supplier));
        assert!(!role.matches_role(DealRole::Consumer));
    }
}
