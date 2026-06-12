use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct RequestPasswordResetCommand {
    pub email: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResetPasswordCommand {
    pub token: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResetPasswordResult {
    pub user_id: Uuid,
}
