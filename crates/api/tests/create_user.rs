use actix_web::http::StatusCode;
use actix_web::{http::header, test, web::Data};
use api::routes;
use api::AppState;
use application::roles::assign_user_roles::AssignUserRoles;
use application::roles::list_roles::ListRoles;
use application::roles::update_role_scopes::UpdateRoleScopes;
use application::users::authenticate_user::AuthenticateUser;
use application::users::create_user::{CreateUser, PasswordHasher};
use application::users::deactivate_user::DeactivateUser;
use application::users::get_user::GetUser;
use application::users::list_users::ListUsers;
use application::users::token::{AuthContext, TokenGenerator, TokenVerifier};
use application::users::update_user::UpdateUser;
use async_trait::async_trait;
use domain::entities::{Email, Role, User, Username};
use domain::errors::DomainError;
use domain::repositories::{RoleRepository, UserRepository};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Once};
use uuid::Uuid;

static INIT_TRACING: Once = Once::new();

fn init_test_tracing() {
    INIT_TRACING.call_once(|| {
        let _ = tracing_subscriber::fmt()
            .with_env_filter("info")
            .with_test_writer()
            .try_init();
    });
}

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

struct FakeTokenService {
    repo: Arc<dyn UserRepository>,
    role_repo: Arc<dyn RoleRepository>,
}

#[async_trait]
impl TokenGenerator for FakeTokenService {
    async fn generate(
        &self,
        ctx: &AuthContext,
    ) -> Result<String, application::errors::ApplicationError> {
        Ok(format!("token-{}", ctx.user_id))
    }
}

#[async_trait]
impl TokenVerifier for FakeTokenService {
    async fn verify(
        &self,
        token: &str,
    ) -> Result<AuthContext, application::errors::ApplicationError> {
        let user_id = token
            .strip_prefix("token-")
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or(application::errors::ApplicationError::Unauthorized)?;

        let user = self
            .repo
            .find_by_id(user_id)
            .await
            .map_err(|_| application::errors::ApplicationError::Unauthorized)?
            .ok_or(application::errors::ApplicationError::Unauthorized)?;

        let mut scopes = std::collections::HashSet::new();
        for role in &user.roles {
            if let Ok(Some(def)) = self.role_repo.find_by_name(role).await {
                scopes.extend(def.scopes);
            }
        }
        let mut scopes: Vec<_> = scopes.into_iter().collect();
        scopes.sort();

        Ok(AuthContext {
            user_id,
            roles: user.roles,
            scopes,
        })
    }
}

struct FakeRoleRepo {
    roles: Mutex<HashMap<String, Role>>,
}

#[async_trait]
impl RoleRepository for FakeRoleRepo {
    async fn find_by_name(&self, name: &str) -> Result<Option<Role>, DomainError> {
        Ok(self.roles.lock().unwrap().get(name).cloned())
    }

    async fn list(&self) -> Result<Vec<Role>, DomainError> {
        Ok(self.roles.lock().unwrap().values().cloned().collect())
    }

    async fn save(&self, role: &Role) -> Result<(), DomainError> {
        self.roles
            .lock()
            .unwrap()
            .insert(role.name.clone(), role.clone());
        Ok(())
    }

    async fn delete(&self, _name: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

fn seeded_role_repo() -> Arc<FakeRoleRepo> {
    Arc::new(FakeRoleRepo {
        roles: Mutex::new(HashMap::from([
            (
                "user".to_string(),
                Role::builtin(
                    "user",
                    vec!["users:read".to_string(), "users:write".to_string()],
                ),
            ),
            (
                "admin".to_string(),
                Role::builtin(
                    "admin",
                    vec![
                        "users:read".to_string(),
                        "users:write".to_string(),
                        "users:admin".to_string(),
                        "users:delete".to_string(),
                    ],
                ),
            ),
        ])),
    })
}

fn test_state() -> AppState {
    let repo: Arc<dyn UserRepository> = Arc::new(FakeRepo {
        users: Mutex::new(HashMap::new()),
    });
    let role_repo: Arc<dyn RoleRepository> = seeded_role_repo();
    let hasher: Arc<dyn PasswordHasher> = Arc::new(FakeHasher);
    let token: Arc<FakeTokenService> = Arc::new(FakeTokenService {
        repo: repo.clone(),
        role_repo: role_repo.clone(),
    });

    AppState {
        create_user: CreateUser::new(repo.clone(), hasher.clone()),
        get_user: GetUser::new(repo.clone()),
        list_users: ListUsers::new(repo.clone()),
        update_user: UpdateUser::new(repo.clone()),
        assign_user_roles: AssignUserRoles::new(repo.clone()),
        deactivate_user: DeactivateUser::new(repo.clone()),
        authenticate_user: AuthenticateUser::new(
            repo.clone(),
            role_repo.clone(),
            hasher,
            token.clone(),
        ),
        list_roles: ListRoles::new(role_repo.clone()),
        update_role_scopes: UpdateRoleScopes::new(role_repo),
        token_validator: token,
    }
}

async fn login(state: &AppState, email: &str) -> String {
    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(state.clone()))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({ "email": email, "password": "password123" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    body["token"].as_str().unwrap().to_string()
}

#[actix_rt::test]
async fn health_returns_ok() {
    init_test_tracing();
    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(test_state()))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get().uri("/api/v1/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_rt::test]
async fn create_user_returns_201() {
    init_test_tracing();
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
    init_test_tracing();
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
async fn get_user_returns_401_when_unauthenticated() {
    init_test_tracing();
    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(test_state()))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/users/{}", Uuid::nil()))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn get_user_returns_401_for_invalid_token() {
    init_test_tracing();
    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(test_state()))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/users/{}", Uuid::nil()))
        .insert_header((header::AUTHORIZATION, "Bearer not-a-token"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn get_user_returns_200_when_authenticated() {
    init_test_tracing();
    let state = test_state();
    let created = state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "get@example.com".to_string(),
            username: "getuser".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&state, "get@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/users/{}", created.id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_rt::test]
async fn get_user_returns_404_when_missing() {
    init_test_tracing();
    let state = test_state();
    state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "missing@example.com".to_string(),
            username: "missing".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&state, "missing@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/users/{}", Uuid::nil()))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_rt::test]
async fn list_users_returns_401_when_unauthenticated() {
    init_test_tracing();
    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(test_state()))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/users?page=1&per_page=10")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn list_users_returns_200_when_authenticated() {
    init_test_tracing();
    let state = test_state();
    state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "list@example.com".to_string(),
            username: "listuser".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&state, "list@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/users?page=1&per_page=10")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_rt::test]
async fn update_user_returns_200_for_owner() {
    init_test_tracing();
    let state = test_state();
    let created = state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "update@example.com".to_string(),
            username: "updateuser".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&state, "update@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/users/{}", created.id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(serde_json::json!({ "username": "updateduser" }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_rt::test]
async fn update_user_returns_403_for_non_owner() {
    init_test_tracing();
    let state = test_state();
    let owner = state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "owner@example.com".to_string(),
            username: "owner".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "other@example.com".to_string(),
            username: "other".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&state, "other@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/users/{}", owner.id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(serde_json::json!({ "username": "hacked" }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn deactivate_user_returns_200_and_blocks_login() {
    init_test_tracing();
    let state = test_state();
    // First user becomes the protected admin.
    state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "admin@example.com".to_string(),
            username: "admin".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let created = state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "inactive@example.com".to_string(),
            username: "inactive".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&state, "inactive@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let deactivate = test::TestRequest::delete()
        .uri(&format!("/api/v1/users/{}", created.id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, deactivate).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let login = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "email": "inactive@example.com",
            "password": "password123"
        }))
        .to_request();
    let resp = test::call_service(&app, login).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn deactivate_admin_returns_403() {
    init_test_tracing();
    let state = test_state();
    let admin = state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "admin@example.com".to_string(),
            username: "admin".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&state, "admin@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/users/{}", admin.id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn login_returns_200_for_active_user() {
    init_test_tracing();
    let state = test_state();
    state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "login@example.com".to_string(),
            username: "loginuser".to_string(),
            password: "password123".to_string(),
        })
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
            "email": "login@example.com",
            "password": "password123"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.get("token").is_some());
}

#[actix_rt::test]
async fn login_returns_401_for_invalid_credentials() {
    init_test_tracing();
    let state = test_state();
    state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "bad@example.com".to_string(),
            username: "baduser".to_string(),
            password: "password123".to_string(),
        })
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
            "email": "bad@example.com",
            "password": "wrongpassword"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn admin_can_list_roles() {
    init_test_tracing();
    let state = test_state();
    state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "admin@example.com".to_string(),
            username: "admin".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&state, "admin@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/roles")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let roles = body["roles"].as_array().unwrap();
    assert!(roles.iter().any(|r| r["name"] == "admin"));
}

#[actix_rt::test]
async fn admin_can_update_role_scopes() {
    init_test_tracing();
    let state = test_state();
    state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "admin@example.com".to_string(),
            username: "admin".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&state, "admin@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::put()
        .uri("/api/v1/roles/moderator")
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(serde_json::json!({ "scopes": ["users:read", "users:write"] }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["name"], "moderator");
    assert!(body["scopes"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("users:read")));
}

#[actix_rt::test]
async fn admin_can_assign_roles_to_user() {
    init_test_tracing();
    let state = test_state();
    state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "admin@example.com".to_string(),
            username: "admin".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let target = state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "target@example.com".to_string(),
            username: "target".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&state, "admin@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/users/{}/roles", target.id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(serde_json::json!({ "roles": ["admin"] }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["roles"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("admin")));
}

#[actix_rt::test]
async fn non_admin_cannot_assign_roles() {
    init_test_tracing();
    let state = test_state();
    state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "admin@example.com".to_string(),
            username: "admin".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let target = state
        .create_user
        .execute(application::users::dto::CreateUserCommand {
            email: "target@example.com".to_string(),
            username: "target".to_string(),
            password: "password123".to_string(),
        })
        .await
        .unwrap();
    let token = login(&state, "target@example.com").await;

    let app = test::init_service(
        actix_web::App::new()
            .app_data(Data::new(state))
            .configure(routes::configure),
    )
    .await;

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/users/{}/roles", target.id))
        .insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        .set_json(serde_json::json!({ "roles": ["admin"] }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
