use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct VerifyEmailCommand {
    pub token: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerifyEmailResult {
    pub user_id: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResendVerificationCommand {
    pub email: String,
}
