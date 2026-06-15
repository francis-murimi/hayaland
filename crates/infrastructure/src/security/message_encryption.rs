use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use application::errors::ApplicationError;
use application::ports::EncryptionService;
use async_trait::async_trait;
use rand::RngCore;

const KEY_LEN: usize = 32;
const IV_LEN: usize = 12;
const TAG_LEN: usize = 16;

/// AES-256-GCM encryption service for message bodies.
///
/// Ciphertext format: `base64(IV || TAG || CIPHERTEXT)`.
/// The IV is randomly generated per encryption call; the authentication tag is
/// appended by `aes-gcm` during encryption and is reordered into the format above.
pub struct MessageEncryptionService {
    cipher: Aes256Gcm,
}

impl std::fmt::Debug for MessageEncryptionService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MessageEncryptionService")
            .field("key", &"<redacted>")
            .finish()
    }
}

impl MessageEncryptionService {
    /// Create a service from a raw 32-byte key.
    pub fn new(key: &[u8]) -> Result<Self, ApplicationError> {
        if key.len() != KEY_LEN {
            return Err(ApplicationError::Infrastructure(format!(
                "encryption key must be {KEY_LEN} bytes, got {}",
                key.len()
            )));
        }
        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(key);
        Ok(Self {
            cipher: Aes256Gcm::new(key),
        })
    }

    /// Load the key from a base64-encoded string.
    /// The decoded key must be exactly 32 bytes.
    pub fn from_base64(encoded: &str) -> Result<Self, ApplicationError> {
        let key = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded)
            .map_err(|e| ApplicationError::Infrastructure(format!("invalid base64 key: {e}")))?;
        Self::new(&key)
    }

    /// Load the key from the `APP_MESSAGES__ENCRYPTION_KEY` environment variable.
    /// The value is expected to be a base64-encoded 32-byte key.
    pub fn from_env() -> Result<Self, ApplicationError> {
        let encoded = std::env::var("APP_MESSAGES__ENCRYPTION_KEY").map_err(|_| {
            ApplicationError::Infrastructure("APP_MESSAGES__ENCRYPTION_KEY is not set".to_string())
        })?;
        Self::from_base64(&encoded)
    }

    fn encrypt_sync(&self, plaintext: &str) -> Result<String, ApplicationError> {
        let mut iv = [0u8; IV_LEN];
        OsRng.fill_bytes(&mut iv);
        let nonce = Nonce::from_slice(&iv);

        let mut ciphertext_with_tag = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| ApplicationError::Infrastructure(format!("encryption failed: {e}")))?;

        // aes-gcm returns ciphertext || tag; reorder to IV || tag || ciphertext.
        let tag = ciphertext_with_tag.split_off(ciphertext_with_tag.len() - TAG_LEN);

        let mut buf = Vec::with_capacity(IV_LEN + TAG_LEN + ciphertext_with_tag.len());
        buf.extend_from_slice(&iv);
        buf.extend_from_slice(&tag);
        buf.extend_from_slice(&ciphertext_with_tag);

        Ok(base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            buf,
        ))
    }

    fn decrypt_sync(&self, ciphertext: &str) -> Result<String, ApplicationError> {
        let buf = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, ciphertext)
            .map_err(|e| ApplicationError::Infrastructure(format!("invalid base64: {e}")))?;

        if buf.len() < IV_LEN + TAG_LEN {
            return Err(ApplicationError::Infrastructure(
                "ciphertext is too short".to_string(),
            ));
        }

        let iv = &buf[..IV_LEN];
        let tag = &buf[IV_LEN..IV_LEN + TAG_LEN];
        let ciphertext_only = &buf[IV_LEN + TAG_LEN..];

        // Reconstruct ciphertext || tag for aes-gcm.
        let mut payload = Vec::with_capacity(ciphertext_only.len() + TAG_LEN);
        payload.extend_from_slice(ciphertext_only);
        payload.extend_from_slice(tag);

        let nonce = Nonce::from_slice(iv);
        let plaintext = self
            .cipher
            .decrypt(nonce, payload.as_ref())
            .map_err(|e| ApplicationError::Infrastructure(format!("decryption failed: {e}")))?;

        String::from_utf8(plaintext)
            .map_err(|e| ApplicationError::Infrastructure(format!("invalid utf-8: {e}")))
    }
}

#[async_trait]
impl EncryptionService for MessageEncryptionService {
    async fn encrypt(&self, plaintext: &str) -> Result<String, ApplicationError> {
        // AES-GCM encryption is CPU-bound but very fast; keep it async-friendly by
        // running on the blocking pool.
        let plaintext = plaintext.to_string();
        let this = MessageEncryptionService {
            cipher: self.cipher.clone(),
        };
        tokio::task::spawn_blocking(move || this.encrypt_sync(&plaintext))
            .await
            .map_err(|e| ApplicationError::Infrastructure(format!("encryption task: {e}")))?
    }

    async fn decrypt(&self, ciphertext: &str) -> Result<String, ApplicationError> {
        let ciphertext = ciphertext.to_string();
        let this = MessageEncryptionService {
            cipher: self.cipher.clone(),
        };
        tokio::task::spawn_blocking(move || this.decrypt_sync(&ciphertext))
            .await
            .map_err(|e| ApplicationError::Infrastructure(format!("decryption task: {e}")))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31,
        ]
    }

    fn service() -> MessageEncryptionService {
        MessageEncryptionService::new(&test_key()).unwrap()
    }

    #[tokio::test]
    async fn round_trip_encrypts_and_decrypts() {
        let svc = service();
        let plaintext = "hello, hayaland!";

        let ciphertext = svc.encrypt(plaintext).await.unwrap();
        assert_ne!(ciphertext, plaintext);

        let decrypted = svc.decrypt(&ciphertext).await.unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[tokio::test]
    async fn each_encryption_uses_a_different_iv() {
        let svc = service();
        let plaintext = "same text";

        let a = svc.encrypt(plaintext).await.unwrap();
        let b = svc.encrypt(plaintext).await.unwrap();
        assert_ne!(a, b);
    }

    #[tokio::test]
    async fn tampered_ciphertext_fails_to_decrypt() {
        let svc = service();
        let ciphertext = svc.encrypt("secret").await.unwrap();

        let mut bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, ciphertext).unwrap();
        // Flip a bit in the ciphertext portion (after IV + tag).
        if bytes.len() > IV_LEN + TAG_LEN {
            bytes[IV_LEN + TAG_LEN] ^= 0x01;
        }
        let tampered = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes);

        let result = svc.decrypt(&tampered).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn rejects_short_key() {
        let err = MessageEncryptionService::new(&[0u8; 31]).unwrap_err();
        assert!(matches!(err, ApplicationError::Infrastructure(_)));
    }

    #[tokio::test]
    async fn rejects_long_key() {
        let err = MessageEncryptionService::new(&[0u8; 33]).unwrap_err();
        assert!(matches!(err, ApplicationError::Infrastructure(_)));
    }
}
