use actix_web::{http::StatusCode, test, web::Data};
use api::routes;
use api::AppState;
use application::users::authenticate_user::AuthenticateUser;
use application::users::create_user::{CreateUser, PasswordHasher};
use application::users::deactivate_user::DeactivateUser;
use application::users::get_user::GetUser;
use application::users::list_users::ListUsers;
use application::users::token::TokenGenerator;
use application::users::update_user::UpdateUser;
use async_trait::async_trait;
use domain::entities::{Email, User, Username};
use domain::errors::DomainError;
use domain::repositories::UserRepository;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

struct FakeRepo {
    users: Mutex<HashMap<Uuid, User>>,
}

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
        _limit: i64,
        _offset: i64,
        active_only: Option<bool>,
    ) -> Result<Vec<User>, DomainError> {
        let users: Vec<User> = self.users.lock().unwrap().values().cloned().collect();
        match active_only {
            Some(true) => Ok(users.into_iter().filter(|u| u.is_active).collect()),
            Some(false) => Ok(users.into_iter().filter(|u| !u.is_active).collect()),
            None => Ok(users),
        }
    }

    async fn update(&self, user: &User) -> Result<(), DomainError> {
        self.users.lock().unwrap().insert(user.id, user.clone());
        Ok(())
    }
}

struct FakeHasher;

#[async_trait]
impl PasswordHasher for FakeHasher {
    async fn hash_password(
        &self,
        password: &str,
    ) -> Result<String, application::errors::ApplicationError> {
        Ok(format!("hashed:{password}"))
    }

    async fn verify_password(
        &self,
        password: &str,
        hash: &str,
    ) -> Result<bool, application::errors::ApplicationError> {
        Ok(hash == format!("hashed:{password}"))
    }
}

struct FakeTokenGenerator;

#[async_trait]
impl TokenGenerator for FakeTokenGenerator {
    async fn generate(
        &self,
        user_id: Uuid,
    ) -> Result<String, application::errors::ApplicationError> {
        Ok(format!("token-{user_id}"))
    }
}

fn test_state() -> AppState {
    let repo: Arc<dyn UserRepository> = Arc::new(FakeRepo {
        users: Mutex::new(HashMap::new()),
    });
    let hasher: Arc<dyn PasswordHasher> = Arc::new(FakeHasher);
    let token: Arc<dyn TokenGenerator> = Arc::new(FakeTokenGenerator);

    AppState {
        create_user: CreateUser::new(repo.clone(), hasher.clone()),
        get_user: GetUser::new(repo.clone()),
        list_users: ListUsers::new(repo.clone()),
        update_user: UpdateUser::new(repo.clone()),
        deactivate_user: DeactivateUser::new(repo.clone()),
        authenticate_user: AuthenticateUser::new(repo, hasher, token),
    }
}

#[actix_rt::test]
async fn create_user_returns_201() {
    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(test_state()))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/users")
        .set_json(serde_json::json!({
            "email": "test@example.com",
            "username": "testuser",
            "password": "password123"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("id").is_some());
}

#[actix_rt::test]
async fn create_user_returns_400_for_invalid_input() {
    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(test_state()))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/users")
        .set_json(serde_json::json!({
            "email": "not-an-email",
            "username": "ab",
            "password": "short"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn login_rejects_inactive_user() {
    let state = test_state();
    let created = state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "inactive@example.com".to_string(),
            username: "inactive".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();

    state
        .deactivate_user
        .execute(application::users::dto::DeactivateUserCommand { id: created.id })
        .await
        .unwrap();

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "inactive@example.com",
            "password": "password123"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
