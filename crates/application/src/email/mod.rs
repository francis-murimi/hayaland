use crate::errors::ApplicationError;
use async_trait::async_trait;

pub mod dto;
pub mod resend_verification;
pub mod verify_email;

#[async_trait]
pub trait EmailSender: Send + Sync {
    async fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), ApplicationError>;
}

pub fn build_verification_email(
    base_url: &str,
    token: &str,
    expiry_hours: i64,
) -> (String, String) {
    let subject = "Verify your Hayaland account".to_string();
    let link = format!("{base_url}/api/v1/auth/verify-email?token={token}");
    let body = format!(
        "Welcome to Hayaland!\n\n\
        Please verify your email by clicking the link below:\n\
        {link}\n\n\
        This link expires in {expiry_hours} hours."
    );
    (subject, body)
}

pub fn generate_verification_token() -> String {
    let mut bytes = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
    base64::Engine::encode(&base64::prelude::BASE64_URL_SAFE_NO_PAD, bytes)
}
