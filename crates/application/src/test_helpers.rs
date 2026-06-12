#[cfg(test)]
use crate::email::EmailSender;
#[cfg(test)]
use crate::errors::ApplicationError;
#[cfg(test)]
use crate::users::create_user::PasswordHasher;
#[cfg(test)]
use crate::users::token::{AuthContext, TokenGenerator, TokenVerifier};
#[cfg(test)]
use async_trait::async_trait;
#[cfg(test)]
use domain::entities::{Email, EmailVerification, PasswordHash, Role, User, Username};
#[cfg(test)]
use domain::errors::DomainError;
#[cfg(test)]
use domain::repositories::{EmailVerificationRepository, RoleRepository, UserRepository};
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::sync::{Arc, Mutex};
#[cfg(test)]
use uuid::Uuid;

#[cfg(test)]
pub struct FakeRepo {
    pub users: Mutex<HashMap<Uuid, User>>,
}

#[cfg(test)]
#[async_trait]
impl UserRepository for FakeRepo {
    async fn create(&self, user: &User) -> Result<(), DomainError> {
        self.users.lock().unwrap().insert(user.id, user.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, DomainError> {
        Ok(self.users.lock().unwrap().get(&id).cloned())
    }

    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, DomainError> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .values()
            .find(|u| u.email == *email)
            .cloned())
    }

    async fn find_by_username(&self, username: &Username) -> Result<Option<User>, DomainError> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .values()
            .find(|u| u.username == *username)
            .cloned())
    }

    async fn list(
        &self,
        limit: i64,
        offset: i64,
        active_only: Option<bool>,
    ) -> Result<Vec<User>, DomainError> {
        let users: Vec<User> = self.users.lock().unwrap().values().cloned().collect();
        let filtered = match active_only {
            Some(true) => users
                .into_iter()
                .filter(|u| u.is_active)
                .collect::<Vec<_>>(),
            Some(false) => users
                .into_iter()
                .filter(|u| !u.is_active)
                .collect::<Vec<_>>(),
            None => users,
        };
        let start = offset as usize;
        let end = (offset + limit) as usize;
        Ok(filtered.into_iter().skip(start).take(end - start).collect())
    }

    async fn update(&self, user: &User) -> Result<(), DomainError> {
        self.users.lock().unwrap().insert(user.id, user.clone());
        Ok(())
    }

    async fn count(&self) -> Result<i64, DomainError> {
        Ok(self.users.lock().unwrap().len() as i64)
    }
}

#[cfg(test)]
pub struct FakeHasher;

#[cfg(test)]
#[async_trait]
impl PasswordHasher for FakeHasher {
    async fn hash_password(&self, password: &str) -> Result<String, ApplicationError> {
        Ok(format!("hashed:{password}"))
    }

    async fn verify_password(&self, password: &str, hash: &str) -> Result<bool, ApplicationError> {
        Ok(hash == format!("hashed:{password}"))
    }
}

#[cfg(test)]
pub struct FakeTokenGenerator;

#[cfg(test)]
#[async_trait]
impl TokenGenerator for FakeTokenGenerator {
    async fn generate(&self, ctx: &AuthContext) -> Result<String, ApplicationError> {
        Ok(format!("token-{}", ctx.user_id))
    }
}

#[cfg(test)]
pub struct FakeTokenVerifier;

#[cfg(test)]
#[async_trait]
impl TokenVerifier for FakeTokenVerifier {
    async fn verify(&self, token: &str) -> Result<AuthContext, ApplicationError> {
        let user_id = token
            .strip_prefix("token-")
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or(ApplicationError::Unauthorized)?;
        Ok(AuthContext {
            user_id,
            roles: vec!["user".to_string()],
            scopes: vec!["users:read".to_string(), "users:write".to_string()],
        })
    }
}

#[cfg(test)]
pub struct FakeRoleRepo;

#[cfg(test)]
#[async_trait]
impl RoleRepository for FakeRoleRepo {
    async fn find_by_name(&self, name: &str) -> Result<Option<Role>, DomainError> {
        match name {
            "user" => Ok(Some(Role::builtin(
                "user",
                vec!["users:read".to_string(), "users:write".to_string()],
            ))),
            "admin" => Ok(Some(Role::builtin(
                "admin",
                vec![
                    "users:read".to_string(),
                    "users:write".to_string(),
                    "users:admin".to_string(),
                    "users:delete".to_string(),
                ],
            ))),
            _ => Ok(None),
        }
    }

    async fn list(&self) -> Result<Vec<Role>, DomainError> {
        Ok(vec![
            self.find_by_name("user").await.unwrap().unwrap(),
            self.find_by_name("admin").await.unwrap().unwrap(),
        ])
    }

    async fn save(&self, _role: &Role) -> Result<(), DomainError> {
        Ok(())
    }

    async fn delete(&self, _name: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

#[cfg(test)]
pub fn test_user(email: &str, username: &str, password: &str) -> User {
    User::new(
        Uuid::now_v7(),
        Email::new(email).unwrap(),
        Username::new(username).unwrap(),
        PasswordHash::new(format!("hashed:{password}")).unwrap(),
    )
}

#[cfg(test)]
pub fn test_repo_with(user: User) -> Arc<FakeRepo> {
    let mut map = HashMap::new();
    map.insert(user.id, user);
    Arc::new(FakeRepo {
        users: Mutex::new(map),
    })
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeEmailVerificationRepo {
    verifications: Mutex<HashMap<String, EmailVerification>>,
}

#[cfg(test)]
impl FakeEmailVerificationRepo {
    pub async fn find_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<EmailVerification>, DomainError> {
        Ok(self
            .verifications
            .lock()
            .unwrap()
            .values()
            .filter(|v| v.user_id == user_id)
            .cloned()
            .collect())
    }

    pub async fn count_for_user(&self, user_id: Uuid) -> usize {
        self.find_by_user_id(user_id).await.unwrap().len()
    }
}

#[cfg(test)]
#[async_trait]
impl EmailVerificationRepository for FakeEmailVerificationRepo {
    async fn save(&self, verification: &EmailVerification) -> Result<(), DomainError> {
        self.verifications
            .lock()
            .unwrap()
            .insert(verification.token.clone(), verification.clone());
        Ok(())
    }

    async fn find_by_token(&self, token: &str) -> Result<Option<EmailVerification>, DomainError> {
        Ok(self.verifications.lock().unwrap().get(token).cloned())
    }

    async fn mark_used(&self, token: &str) -> Result<(), DomainError> {
        if let Some(v) = self.verifications.lock().unwrap().get_mut(token) {
            v.used = true;
        }
        Ok(())
    }

    async fn invalidate_unused_for_user(&self, user_id: Uuid) -> Result<(), DomainError> {
        for v in self.verifications.lock().unwrap().values_mut() {
            if v.user_id == user_id && !v.used {
                v.used = true;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeEmailSender {
    pub sent: Mutex<Vec<(String, String, String)>>,
    failing: bool,
}

#[cfg(test)]
impl FakeEmailSender {
    pub fn failing() -> Self {
        Self {
            sent: Default::default(),
            failing: true,
        }
    }
}

#[cfg(test)]
#[async_trait]
impl EmailSender for FakeEmailSender {
    async fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), ApplicationError> {
        if self.failing {
            return Err(ApplicationError::EmailSendFailed);
        }
        self.sent
            .lock()
            .unwrap()
            .push((to.to_string(), subject.to_string(), body.to_string()));
        Ok(())
    }
}
