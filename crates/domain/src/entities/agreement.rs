use crate::errors::DomainError;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use uuid::Uuid;

/// Lifecycle status of an agreement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgreementStatus {
    Draft,
    PendingSignatures,
    Signed,
    Executed,
    Terminated,
}

impl AgreementStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgreementStatus::Draft => "DRAFT",
            AgreementStatus::PendingSignatures => "PENDING_SIGNATURES",
            AgreementStatus::Signed => "SIGNED",
            AgreementStatus::Executed => "EXECUTED",
            AgreementStatus::Terminated => "TERMINATED",
        }
    }
}

impl TryFrom<&str> for AgreementStatus {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "DRAFT" => Ok(AgreementStatus::Draft),
            "PENDING_SIGNATURES" => Ok(AgreementStatus::PendingSignatures),
            "SIGNED" => Ok(AgreementStatus::Signed),
            "EXECUTED" => Ok(AgreementStatus::Executed),
            "TERMINATED" => Ok(AgreementStatus::Terminated),
            _ => Err(DomainError::Validation(vec![format!(
                "unknown agreement status: {value}"
            )])),
        }
    }
}

/// The mechanism used to record a party's acceptance of an agreement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SignatureType {
    #[default]
    DigitalAttestation,
    Clickwrap,
    Esign,
}

impl SignatureType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SignatureType::DigitalAttestation => "DIGITAL_ATTESTATION",
            SignatureType::Clickwrap => "CLICKWRAP",
            SignatureType::Esign => "ESIGN",
        }
    }
}

impl TryFrom<&str> for SignatureType {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "DIGITAL_ATTESTATION" => Ok(SignatureType::DigitalAttestation),
            "CLICKWRAP" => Ok(SignatureType::Clickwrap),
            "ESIGN" => Ok(SignatureType::Esign),
            _ => Err(DomainError::Validation(vec![format!(
                "unknown signature type: {value}"
            )])),
        }
    }
}

/// A formal rendered agreement for a 3-party deal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Agreement {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub agreement_status: AgreementStatus,
    pub agreement_text: String,
    pub governing_law: Option<String>,
    pub dispute_resolution: Option<String>,
    pub effective_date: Option<time::Date>,
    pub termination_date: Option<time::Date>,
    pub auto_renew: bool,
    pub version: i32,
    pub digital_signature_url: Option<String>,
    pub created_at: OffsetDateTime,
    pub executed_at: Option<OffsetDateTime>,
}

impl Agreement {
    pub fn new(
        id: Uuid,
        deal_id: Uuid,
        agreement_text: String,
        governing_law: Option<String>,
        dispute_resolution: Option<String>,
        effective_date: Option<time::Date>,
        version: i32,
    ) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id,
            deal_id,
            agreement_status: AgreementStatus::PendingSignatures,
            agreement_text,
            governing_law,
            dispute_resolution,
            effective_date,
            termination_date: None,
            auto_renew: false,
            version,
            digital_signature_url: None,
            created_at: now,
            executed_at: None,
        }
    }

    pub fn mark_signed(&mut self) {
        self.agreement_status = AgreementStatus::Signed;
    }

    pub fn mark_executed(&mut self) {
        self.agreement_status = AgreementStatus::Executed;
        self.executed_at = Some(OffsetDateTime::now_utc());
    }

    pub fn mark_terminated(&mut self) {
        self.agreement_status = AgreementStatus::Terminated;
    }

    /// Whether the agreement can currently be signed by parties.
    pub fn can_be_signed(&self) -> bool {
        matches!(
            self.agreement_status,
            AgreementStatus::Draft | AgreementStatus::PendingSignatures
        )
    }

    /// Whether the agreement can be updated by a platform admin.
    pub fn can_be_admin_updated(&self) -> bool {
        !matches!(
            self.agreement_status,
            AgreementStatus::Executed | AgreementStatus::Terminated
        )
    }
}

/// A recorded acceptance of an agreement version by a party.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Signature {
    pub id: Uuid,
    pub agreement_id: Uuid,
    pub party_id: Uuid,
    pub signed_by_user_id: Uuid,
    pub signature_type: SignatureType,
    pub signature_data: String,
    pub ip_address: Option<String>,
    pub signed_at: OffsetDateTime,
    pub version: i32,
}

impl Signature {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Uuid,
        agreement_id: Uuid,
        party_id: Uuid,
        signed_by_user_id: Uuid,
        signature_type: SignatureType,
        signature_data: String,
        ip_address: Option<String>,
        version: i32,
    ) -> Self {
        Self {
            id,
            agreement_id,
            party_id,
            signed_by_user_id,
            signature_type,
            signature_data,
            ip_address,
            signed_at: OffsetDateTime::now_utc(),
            version,
        }
    }

    /// Build the human-readable attestation string used for non-repudiation.
    pub fn attestation_string(
        party_display_name: &str,
        role: &str,
        agreement_id: Uuid,
        version: i32,
        deal_reference: &str,
        signed_at: OffsetDateTime,
    ) -> String {
        format!(
            "I, acting on behalf of {party_display_name} as {role}, \
             have read and agree to Agreement {agreement_id} version {version} \
             for Deal {deal_reference} on {signed_at}."
        )
    }

    /// Compute a SHA-256 attestation hash over the agreement text and signing context.
    pub fn compute_signature_data(
        agreement_text: &str,
        party_id: Uuid,
        signed_by_user_id: Uuid,
        signed_at: OffsetDateTime,
        version: i32,
        attestation: &str,
    ) -> String {
        let payload = format!(
            "{agreement_text}\n{party_id}\n{signed_by_user_id}\n{signed_at}\n{version}\n{attestation}"
        );
        let hash = Sha256::digest(payload.as_bytes());
        format!("sha256:{}", BASE64.encode(hash))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_agreement() -> Agreement {
        Agreement::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            "# Agreement".to_string(),
            Some("CA".to_string()),
            Some("Arbitration".to_string()),
            None,
            1,
        )
    }

    #[test]
    fn new_agreement_is_pending_signatures() {
        let agreement = sample_agreement();
        assert_eq!(
            agreement.agreement_status,
            AgreementStatus::PendingSignatures
        );
        assert_eq!(agreement.version, 1);
    }

    #[test]
    fn agreement_status_from_str() {
        assert_eq!(
            AgreementStatus::try_from("PENDING_SIGNATURES").unwrap(),
            AgreementStatus::PendingSignatures
        );
        assert_eq!(
            AgreementStatus::try_from("SIGNED").unwrap(),
            AgreementStatus::Signed
        );
        assert!(AgreementStatus::try_from("UNKNOWN").is_err());
    }

    #[test]
    fn signature_type_from_str() {
        assert_eq!(
            SignatureType::try_from("DIGITAL_ATTESTATION").unwrap(),
            SignatureType::DigitalAttestation
        );
        assert!(SignatureType::try_from("UNKNOWN").is_err());
    }

    #[test]
    fn mark_signed_and_executed() {
        let mut agreement = sample_agreement();
        agreement.mark_signed();
        assert_eq!(agreement.agreement_status, AgreementStatus::Signed);
        agreement.mark_executed();
        assert_eq!(agreement.agreement_status, AgreementStatus::Executed);
        assert!(agreement.executed_at.is_some());
    }

    #[test]
    fn can_be_signed_only_in_pending_or_draft() {
        let mut agreement = sample_agreement();
        assert!(agreement.can_be_signed());
        agreement.mark_signed();
        assert!(!agreement.can_be_signed());
    }

    #[test]
    fn signature_data_is_deterministic() {
        let agreement = sample_agreement();
        let signed_at = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let data1 = Signature::compute_signature_data(
            &agreement.agreement_text,
            agreement.deal_id,
            Uuid::now_v7(),
            signed_at,
            agreement.version,
            "I agree.",
        );
        let data2 = Signature::compute_signature_data(
            &agreement.agreement_text,
            agreement.deal_id,
            Uuid::now_v7(),
            signed_at,
            agreement.version,
            "I agree.",
        );
        assert_ne!(data1, data2); // user id differs
        assert!(data1.starts_with("sha256:"));
        assert!(data2.starts_with("sha256:"));
    }

    #[test]
    fn signature_data_same_inputs_same_output() {
        let agreement = sample_agreement();
        let party_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();
        let signed_at = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let data1 = Signature::compute_signature_data(
            &agreement.agreement_text,
            party_id,
            user_id,
            signed_at,
            agreement.version,
            "I agree.",
        );
        let data2 = Signature::compute_signature_data(
            &agreement.agreement_text,
            party_id,
            user_id,
            signed_at,
            agreement.version,
            "I agree.",
        );
        assert_eq!(data1, data2);
    }

    #[test]
    fn terminated_agreement_cannot_be_signed_or_admin_updated() {
        let mut agreement = sample_agreement();
        agreement.mark_terminated();
        assert_eq!(agreement.agreement_status, AgreementStatus::Terminated);
        assert!(!agreement.can_be_signed());
        assert!(!agreement.can_be_admin_updated());
    }

    #[test]
    fn executed_agreement_cannot_be_admin_updated() {
        let mut agreement = sample_agreement();
        agreement.mark_executed();
        assert!(!agreement.can_be_admin_updated());
    }
}
