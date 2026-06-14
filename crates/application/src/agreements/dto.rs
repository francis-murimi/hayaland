use domain::entities::{AgreementStatus, SignatureType};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Command to generate (or regenerate) an agreement for a locked deal.
#[derive(Debug, Clone, Deserialize)]
pub struct GenerateAgreementCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub deal_id: Uuid,
}

/// Command to sign the current agreement on behalf of a party.
#[derive(Debug, Clone, Deserialize)]
pub struct SignAgreementCommand {
    pub actor_user_id: Uuid,
    pub actor_party_id: Uuid,
    pub deal_id: Uuid,
    #[serde(default = "default_signature_type")]
    pub signature_type: SignatureType,
    pub ip_address: Option<String>,
}

fn default_signature_type() -> SignatureType {
    SignatureType::DigitalAttestation
}

/// Command for platform admins to update administrative agreement metadata.
#[derive(Debug, Clone, Deserialize)]
pub struct AdminUpdateAgreementCommand {
    pub admin_user_id: Uuid,
    pub deal_id: Uuid,
    pub governing_law: Option<String>,
    pub dispute_resolution: Option<String>,
    pub effective_date: Option<time::Date>,
    pub termination_date: Option<time::Date>,
    #[serde(default)]
    pub auto_renew: Option<bool>,
    pub status: Option<AgreementStatus>,
    pub reason: Option<String>,
}

/// Full agreement representation returned by use cases.
#[derive(Debug, Clone, Serialize)]
pub struct AgreementResult {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub status: AgreementStatus,
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
    pub signatures: Vec<SignatureResult>,
}

/// A recorded signature on an agreement.
#[derive(Debug, Clone, Serialize)]
pub struct SignatureResult {
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
