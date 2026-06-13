use crate::errors::DomainError;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use super::Email;

/// The structural form of a party.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PartyType {
    Individual,
    Organization,
    PartyGroup,
}

impl PartyType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PartyType::Individual => "INDIVIDUAL",
            PartyType::Organization => "ORGANIZATION",
            PartyType::PartyGroup => "PARTY_GROUP",
        }
    }
}

impl TryFrom<&str> for PartyType {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "INDIVIDUAL" => Ok(PartyType::Individual),
            "ORGANIZATION" => Ok(PartyType::Organization),
            "PARTY_GROUP" => Ok(PartyType::PartyGroup),
            _ => Err(DomainError::InvalidPartyType {
                message: format!("unknown party type: {value}"),
            }),
        }
    }
}

/// KYC/business verification state of a party.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationStatus {
    Unverified,
    Pending,
    Verified,
    Rejected,
}

impl VerificationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            VerificationStatus::Unverified => "UNVERIFIED",
            VerificationStatus::Pending => "PENDING",
            VerificationStatus::Verified => "VERIFIED",
            VerificationStatus::Rejected => "REJECTED",
        }
    }
}

impl TryFrom<&str> for VerificationStatus {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "UNVERIFIED" => Ok(VerificationStatus::Unverified),
            "PENDING" => Ok(VerificationStatus::Pending),
            "VERIFIED" => Ok(VerificationStatus::Verified),
            "REJECTED" => Ok(VerificationStatus::Rejected),
            _ => Err(DomainError::InvalidVerificationStatus {
                message: format!("unknown verification status: {value}"),
            }),
        }
    }
}

/// A validated display name for a party.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DisplayName(String);

impl DisplayName {
    pub fn new(value: &str) -> Result<Self, DomainError> {
        let trimmed = value.trim();
        let len = trimmed.chars().count();
        if !(3..=120).contains(&len) {
            return Err(DomainError::InvalidDisplayName {
                message: "display name must be between 3 and 120 characters".to_string(),
            });
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A validated phone number (E.164 format is preferred but not strictly enforced at the domain layer).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Phone(String);

impl Phone {
    pub fn new(value: &str) -> Result<Self, DomainError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(DomainError::InvalidPhone {
                message: "phone number cannot be empty".to_string(),
            });
        }
        if trimmed.chars().count() > 50 {
            return Err(DomainError::InvalidPhone {
                message: "phone number must be 50 characters or fewer".to_string(),
            });
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A geographic point with latitude and longitude.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeoPoint {
    pub latitude: f64,
    pub longitude: f64,
}

impl GeoPoint {
    pub fn new(latitude: f64, longitude: f64) -> Result<Self, DomainError> {
        if !(-90.0..=90.0).contains(&latitude) {
            return Err(DomainError::InvalidLocation {
                message: "latitude must be between -90 and 90".to_string(),
            });
        }
        if !(-180.0..=180.0).contains(&longitude) {
            return Err(DomainError::InvalidLocation {
                message: "longitude must be between -180 and 180".to_string(),
            });
        }
        Ok(Self {
            latitude,
            longitude,
        })
    }
}

/// The central business identity that participates in deals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Party {
    pub id: Uuid,
    pub party_type: PartyType,
    pub display_name: DisplayName,
    pub email: Email,
    pub phone: Option<Phone>,
    pub tax_id: Option<String>,
    pub verification_status: VerificationStatus,
    pub primary_domain_id: Option<Uuid>,
    pub location: Option<GeoPoint>,
    pub service_radius_km: Option<f64>,
    pub trust_score: f64,
    pub total_deals_completed: i32,
    pub total_deals_initiated: i32,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl Party {
    pub fn new(id: Uuid, party_type: PartyType, display_name: DisplayName, email: Email) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id,
            party_type,
            display_name,
            email,
            phone: None,
            tax_id: None,
            verification_status: VerificationStatus::Unverified,
            primary_domain_id: None,
            location: None,
            service_radius_km: None,
            trust_score: 0.0,
            total_deals_completed: 0,
            total_deals_initiated: 0,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn soft_delete(&mut self) {
        self.is_active = false;
        self.updated_at = OffsetDateTime::now_utc();
    }

    pub fn reactivate(&mut self) {
        self.is_active = true;
        self.updated_at = OffsetDateTime::now_utc();
    }

    pub fn can_be_deleted(&self) -> bool {
        self.is_active && self.total_deals_completed == 0 && self.total_deals_initiated == 0
    }
}

/// Membership role of a user within a party.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PartyMembershipRole {
    Owner,
    Admin,
    Member,
    Observer,
}

impl PartyMembershipRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            PartyMembershipRole::Owner => "OWNER",
            PartyMembershipRole::Admin => "ADMIN",
            PartyMembershipRole::Member => "MEMBER",
            PartyMembershipRole::Observer => "OBSERVER",
        }
    }
}

impl TryFrom<&str> for PartyMembershipRole {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "OWNER" => Ok(PartyMembershipRole::Owner),
            "ADMIN" => Ok(PartyMembershipRole::Admin),
            "MEMBER" => Ok(PartyMembershipRole::Member),
            "OBSERVER" => Ok(PartyMembershipRole::Observer),
            _ => Err(DomainError::InvalidPartyMembershipRole {
                message: format!("unknown party membership role: {value}"),
            }),
        }
    }
}

/// Link between a user account and a party.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserPartyMembership {
    pub id: Uuid,
    pub user_id: Uuid,
    pub party_id: Uuid,
    pub member_role: PartyMembershipRole,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
}

impl UserPartyMembership {
    pub fn new(id: Uuid, user_id: Uuid, party_id: Uuid, member_role: PartyMembershipRole) -> Self {
        Self {
            id,
            user_id,
            party_id,
            member_role,
            is_active: true,
            created_at: OffsetDateTime::now_utc(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_email() -> Email {
        Email::new("party@example.com").unwrap()
    }

    fn valid_name() -> DisplayName {
        DisplayName::new("Green Acres Farm").unwrap()
    }

    #[test]
    fn display_name_rejects_too_short() {
        assert!(DisplayName::new("ab").is_err());
    }

    #[test]
    fn display_name_rejects_too_long() {
        assert!(DisplayName::new(&"a".repeat(121)).is_err());
    }

    #[test]
    fn display_name_accepts_valid() {
        let name = DisplayName::new("Green Acres Farm Ltd").unwrap();
        assert_eq!(name.as_str(), "Green Acres Farm Ltd");
    }

    #[test]
    fn phone_rejects_empty() {
        assert!(Phone::new("").is_err());
    }

    #[test]
    fn phone_accepts_valid() {
        let phone = Phone::new("+1-555-0123").unwrap();
        assert_eq!(phone.as_str(), "+1-555-0123");
    }

    #[test]
    fn geo_point_rejects_invalid_latitude() {
        assert!(GeoPoint::new(95.0, 0.0).is_err());
    }

    #[test]
    fn geo_point_rejects_invalid_longitude() {
        assert!(GeoPoint::new(0.0, 185.0).is_err());
    }

    #[test]
    fn geo_point_accepts_valid() {
        let point = GeoPoint::new(37.0, -122.0).unwrap();
        assert_eq!(point.latitude, 37.0);
        assert_eq!(point.longitude, -122.0);
    }

    #[test]
    fn party_starts_active_and_unverified() {
        let party = Party::new(
            Uuid::now_v7(),
            PartyType::Organization,
            valid_name(),
            valid_email(),
        );
        assert!(party.is_active);
        assert_eq!(party.verification_status, VerificationStatus::Unverified);
        assert_eq!(party.trust_score, 0.0);
    }

    #[test]
    fn soft_delete_makes_party_inactive() {
        let mut party = Party::new(
            Uuid::now_v7(),
            PartyType::Individual,
            valid_name(),
            valid_email(),
        );
        party.soft_delete();
        assert!(!party.is_active);
    }

    #[test]
    fn reactivate_restores_party() {
        let mut party = Party::new(
            Uuid::now_v7(),
            PartyType::Individual,
            valid_name(),
            valid_email(),
        );
        party.soft_delete();
        party.reactivate();
        assert!(party.is_active);
    }

    #[test]
    fn party_type_from_str() {
        assert_eq!(
            PartyType::try_from("INDIVIDUAL").unwrap(),
            PartyType::Individual
        );
        assert_eq!(
            PartyType::try_from("ORGANIZATION").unwrap(),
            PartyType::Organization
        );
        assert_eq!(
            PartyType::try_from("PARTY_GROUP").unwrap(),
            PartyType::PartyGroup
        );
        assert!(PartyType::try_from("UNKNOWN").is_err());
    }

    #[test]
    fn verification_status_from_str() {
        assert_eq!(
            VerificationStatus::try_from("VERIFIED").unwrap(),
            VerificationStatus::Verified
        );
        assert!(VerificationStatus::try_from("UNKNOWN").is_err());
    }

    #[test]
    fn membership_role_from_str() {
        assert_eq!(
            PartyMembershipRole::try_from("ADMIN").unwrap(),
            PartyMembershipRole::Admin
        );
        assert!(PartyMembershipRole::try_from("UNKNOWN").is_err());
    }
}
