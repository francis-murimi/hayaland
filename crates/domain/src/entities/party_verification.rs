use crate::errors::DomainError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

/// The type of real-world check performed for a party verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PartyVerificationType {
    Email,
    Phone,
    GovernmentId,
    BusinessRegistration,
    BankAccount,
    ProfessionalCertification,
    VideoInterview,
}

impl PartyVerificationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PartyVerificationType::Email => "EMAIL",
            PartyVerificationType::Phone => "PHONE",
            PartyVerificationType::GovernmentId => "GOVERNMENT_ID",
            PartyVerificationType::BusinessRegistration => "BUSINESS_REGISTRATION",
            PartyVerificationType::BankAccount => "BANK_ACCOUNT",
            PartyVerificationType::ProfessionalCertification => "PROFESSIONAL_CERTIFICATION",
            PartyVerificationType::VideoInterview => "VIDEO_INTERVIEW",
        }
    }

    /// Points this verification type contributes toward the party's verification level.
    pub fn points(&self) -> i32 {
        match self {
            PartyVerificationType::Email => 10,
            PartyVerificationType::Phone => 15,
            PartyVerificationType::GovernmentId => 30,
            PartyVerificationType::BusinessRegistration => 25,
            PartyVerificationType::BankAccount => 10,
            PartyVerificationType::ProfessionalCertification => 10,
            PartyVerificationType::VideoInterview => 10,
        }
    }

    /// Whether this verification type requires administrator review.
    pub fn requires_admin_review(&self) -> bool {
        !matches!(
            self,
            PartyVerificationType::Email | PartyVerificationType::Phone
        )
    }
}

impl TryFrom<&str> for PartyVerificationType {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "EMAIL" => Ok(PartyVerificationType::Email),
            "PHONE" => Ok(PartyVerificationType::Phone),
            "GOVERNMENT_ID" => Ok(PartyVerificationType::GovernmentId),
            "BUSINESS_REGISTRATION" => Ok(PartyVerificationType::BusinessRegistration),
            "BANK_ACCOUNT" => Ok(PartyVerificationType::BankAccount),
            "PROFESSIONAL_CERTIFICATION" => Ok(PartyVerificationType::ProfessionalCertification),
            "VIDEO_INTERVIEW" => Ok(PartyVerificationType::VideoInterview),
            _ => Err(DomainError::InvalidVerificationType {
                message: format!("unknown verification type: {value}"),
            }),
        }
    }
}

/// Lifecycle status of a single party verification record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PartyVerificationStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
    Revoked,
}

impl PartyVerificationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PartyVerificationStatus::Pending => "PENDING",
            PartyVerificationStatus::Approved => "APPROVED",
            PartyVerificationStatus::Rejected => "REJECTED",
            PartyVerificationStatus::Expired => "EXPIRED",
            PartyVerificationStatus::Revoked => "REVOKED",
        }
    }
}

impl TryFrom<&str> for PartyVerificationStatus {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "PENDING" => Ok(PartyVerificationStatus::Pending),
            "APPROVED" => Ok(PartyVerificationStatus::Approved),
            "REJECTED" => Ok(PartyVerificationStatus::Rejected),
            "EXPIRED" => Ok(PartyVerificationStatus::Expired),
            "REVOKED" => Ok(PartyVerificationStatus::Revoked),
            _ => Err(DomainError::InvalidVerificationStatus {
                message: format!("unknown verification status: {value}"),
            }),
        }
    }
}

/// A request for a party to be verified for a specific real-world attribute.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartyVerification {
    pub id: Uuid,
    pub party_id: Uuid,
    pub requested_by_user_id: Uuid,
    pub reviewed_by_user_id: Option<Uuid>,
    pub verification_type: PartyVerificationType,
    pub status: PartyVerificationStatus,
    pub points: i32,
    pub evidence_urls: Vec<String>,
    pub provider_reference: Option<String>,
    pub provider_payload: Option<Value>,
    pub rejection_reason: Option<String>,
    pub review_notes: Option<String>,
    pub requested_at: OffsetDateTime,
    pub reviewed_at: Option<OffsetDateTime>,
    pub expires_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl PartyVerification {
    /// Create a new pending verification request.
    pub fn new(
        id: Uuid,
        party_id: Uuid,
        requested_by_user_id: Uuid,
        verification_type: PartyVerificationType,
        evidence_urls: Vec<String>,
        review_notes: Option<String>,
    ) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id,
            party_id,
            requested_by_user_id,
            reviewed_by_user_id: None,
            verification_type,
            status: PartyVerificationStatus::Pending,
            points: verification_type.points(),
            evidence_urls,
            provider_reference: None,
            provider_payload: None,
            rejection_reason: None,
            review_notes,
            requested_at: now,
            reviewed_at: None,
            expires_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Approve the verification, recording the admin who reviewed it.
    pub fn approve(&mut self, reviewed_by_user_id: Uuid, notes: Option<String>) {
        let now = OffsetDateTime::now_utc();
        self.status = PartyVerificationStatus::Approved;
        self.reviewed_by_user_id = Some(reviewed_by_user_id);
        self.reviewed_at = Some(now);
        self.review_notes = notes.or_else(|| self.review_notes.take());
        self.updated_at = now;
    }

    /// Reject the verification with a required reason.
    pub fn reject(
        &mut self,
        reviewed_by_user_id: Uuid,
        rejection_reason: String,
        notes: Option<String>,
    ) {
        let now = OffsetDateTime::now_utc();
        self.status = PartyVerificationStatus::Rejected;
        self.reviewed_by_user_id = Some(reviewed_by_user_id);
        self.reviewed_at = Some(now);
        self.rejection_reason = Some(rejection_reason);
        self.review_notes = notes.or_else(|| self.review_notes.take());
        self.updated_at = now;
    }

    /// Revoke a previously approved verification.
    pub fn revoke(&mut self, reviewed_by_user_id: Uuid, reason: String, notes: Option<String>) {
        let now = OffsetDateTime::now_utc();
        self.status = PartyVerificationStatus::Revoked;
        self.reviewed_by_user_id = Some(reviewed_by_user_id);
        self.reviewed_at = Some(now);
        self.rejection_reason = Some(reason);
        self.review_notes = notes.or_else(|| self.review_notes.take());
        self.updated_at = now;
    }
}

/// Compute the verification level (0–5) from effective approved points.
pub fn verification_level_from_points(points: i64) -> i32 {
    match points {
        0 => 0,
        10..=24 => 1,
        25..=54 => 2,
        55..=79 => 3,
        80..=99 => 4,
        _ => 5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verification_type_as_str_round_trips() {
        let cases = [
            PartyVerificationType::Email,
            PartyVerificationType::Phone,
            PartyVerificationType::GovernmentId,
            PartyVerificationType::BusinessRegistration,
            PartyVerificationType::BankAccount,
            PartyVerificationType::ProfessionalCertification,
            PartyVerificationType::VideoInterview,
        ];
        for case in cases {
            assert_eq!(
                PartyVerificationType::try_from(case.as_str()).unwrap(),
                case
            );
        }
    }

    #[test]
    fn verification_type_points_match_spec() {
        assert_eq!(PartyVerificationType::Email.points(), 10);
        assert_eq!(PartyVerificationType::Phone.points(), 15);
        assert_eq!(PartyVerificationType::GovernmentId.points(), 30);
        assert_eq!(PartyVerificationType::BusinessRegistration.points(), 25);
        assert_eq!(PartyVerificationType::BankAccount.points(), 10);
        assert_eq!(
            PartyVerificationType::ProfessionalCertification.points(),
            10
        );
        assert_eq!(PartyVerificationType::VideoInterview.points(), 10);
    }

    #[test]
    fn automated_types_do_not_require_admin_review() {
        assert!(!PartyVerificationType::Email.requires_admin_review());
        assert!(!PartyVerificationType::Phone.requires_admin_review());
    }

    #[test]
    fn admin_review_types_require_admin_review() {
        assert!(PartyVerificationType::GovernmentId.requires_admin_review());
        assert!(PartyVerificationType::BusinessRegistration.requires_admin_review());
        assert!(PartyVerificationType::BankAccount.requires_admin_review());
        assert!(PartyVerificationType::ProfessionalCertification.requires_admin_review());
        assert!(PartyVerificationType::VideoInterview.requires_admin_review());
    }

    #[test]
    fn verification_status_as_str_round_trips() {
        let cases = [
            PartyVerificationStatus::Pending,
            PartyVerificationStatus::Approved,
            PartyVerificationStatus::Rejected,
            PartyVerificationStatus::Expired,
            PartyVerificationStatus::Revoked,
        ];
        for case in cases {
            assert_eq!(
                PartyVerificationStatus::try_from(case.as_str()).unwrap(),
                case
            );
        }
    }

    #[test]
    fn unknown_verification_type_returns_error() {
        assert!(PartyVerificationType::try_from("UNKNOWN").is_err());
    }

    #[test]
    fn new_verification_is_pending_with_correct_points() {
        let v = PartyVerification::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            PartyVerificationType::GovernmentId,
            vec!["url".to_string()],
            Some("notes".to_string()),
        );
        assert_eq!(v.status, PartyVerificationStatus::Pending);
        assert_eq!(v.points, 30);
        assert!(v.reviewed_by_user_id.is_none());
        assert!(v.reviewed_at.is_none());
    }

    #[test]
    fn approve_sets_status_and_reviewer() {
        let mut v = PartyVerification::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            PartyVerificationType::Email,
            vec![],
            None,
        );
        let admin = Uuid::now_v7();
        v.approve(admin, Some("looks good".to_string()));
        assert_eq!(v.status, PartyVerificationStatus::Approved);
        assert_eq!(v.reviewed_by_user_id, Some(admin));
        assert!(v.reviewed_at.is_some());
        assert_eq!(v.review_notes.as_deref(), Some("looks good"));
    }

    #[test]
    fn reject_sets_reason() {
        let mut v = PartyVerification::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            PartyVerificationType::GovernmentId,
            vec!["url".to_string()],
            None,
        );
        let admin = Uuid::now_v7();
        v.reject(admin, "expired document".to_string(), None);
        assert_eq!(v.status, PartyVerificationStatus::Rejected);
        assert_eq!(v.rejection_reason.as_deref(), Some("expired document"));
    }

    #[test]
    fn revoke_sets_revoked_status() {
        let mut v = PartyVerification::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            PartyVerificationType::BankAccount,
            vec!["url".to_string()],
            None,
        );
        let admin = Uuid::now_v7();
        v.revoke(
            admin,
            "fraud".to_string(),
            Some("internal note".to_string()),
        );
        assert_eq!(v.status, PartyVerificationStatus::Revoked);
        assert_eq!(v.rejection_reason.as_deref(), Some("fraud"));
    }

    #[test]
    fn level_mapping_is_correct() {
        assert_eq!(verification_level_from_points(0), 0);
        assert_eq!(verification_level_from_points(10), 1);
        assert_eq!(verification_level_from_points(25), 2);
        assert_eq!(verification_level_from_points(55), 3);
        assert_eq!(verification_level_from_points(80), 4);
        assert_eq!(verification_level_from_points(100), 5);
        assert_eq!(verification_level_from_points(110), 5);
    }
}
