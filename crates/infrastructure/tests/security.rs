use application::users::create_user::PasswordHasher;
use application::users::token::TokenGenerator;
use infrastructure::security::{Argon2PasswordHasher, JwtTokenGenerator};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
    iat: usize,
}

#[tokio::test]
async fn argon2_hashes_and_verifies_password() {
    let hasher = Argon2PasswordHasher;
    let password = "a-very-strong-password";

    let hash = hasher.hash_password(password).await.unwrap();
    assert_ne!(hash, password);
    assert!(hash.starts_with("$argon2id$"));

    assert!(hasher.verify_password(password, &hash).await.unwrap());
    assert!(!hasher.verify_password("wrong", &hash).await.unwrap());
}

#[tokio::test]
async fn jwt_generates_valid_token() {
    let secret = "test-secret".to_string();
    let generator = JwtTokenGenerator::new(secret.clone(), 3600);
    let user_id = uuid::Uuid::now_v7();

    let token = generator.generate(user_id).await.unwrap();
    let decoded = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .unwrap();

    assert_eq!(decoded.claims.sub, user_id.to_string());

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    assert!(decoded.claims.exp > now);
}
