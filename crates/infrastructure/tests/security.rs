use application::users::create_user::PasswordHasher;
use application::users::token::{AuthContext, TokenGenerator};
use infrastructure::security::{Argon2PasswordHasher, JwtTokenService};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    roles: Vec<String>,
    scope: Vec<String>,
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
    let generator = JwtTokenService::new(secret.clone(), 3600);
    let user_id = uuid::Uuid::now_v7();

    let ctx = AuthContext {
        user_id,
        roles: vec!["user".to_string()],
        scopes: vec!["users:read".to_string()],
    };

    let token = generator.generate(&ctx).await.unwrap();
    let decoded = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .unwrap();

    assert_eq!(decoded.claims.sub, user_id.to_string());
    assert_eq!(decoded.claims.roles, ctx.roles);
    assert_eq!(decoded.claims.scope, ctx.scopes);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    assert!(decoded.claims.exp > now);
}
