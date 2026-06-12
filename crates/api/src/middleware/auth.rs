use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::header::AUTHORIZATION,
    web, Error, HttpMessage, HttpResponse,
};
use application::users::token::AuthContext;
use std::future::{ready, Future, Ready};
use std::pin::Pin;
use std::sync::Arc;
use tracing::{info, warn};

/// Routes that are intentionally accessible without authentication.
const PUBLIC_ROUTES: &[(&str, &str)] = &[
    ("GET", "/api/v1/health"),
    ("POST", "/api/v1/auth/login"),
    ("POST", "/api/v1/users"),
    ("GET", "/api/v1/auth/verify-email"),
    ("POST", "/api/v1/auth/resend-verification"),
    ("POST", "/api/v1/auth/forgot-password"),
    ("POST", "/api/v1/auth/reset-password"),
];

/// Middleware that validates a `Bearer` JWT for protected routes and inserts the
/// resulting `AuthContext` into the request extensions.
pub struct Authentication;

impl<S, B> Transform<S, ServiceRequest> for Authentication
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Transform = AuthenticationMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthenticationMiddleware {
            service: Arc::new(service),
        }))
    }
}

pub struct AuthenticationMiddleware<S> {
    service: Arc<S>,
}

type LocalBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

impl<S, B> Service<ServiceRequest> for AuthenticationMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let state = req
            .app_data::<web::Data<crate::AppState>>()
            .cloned()
            .expect("AppState not available");

        let method = req.method().as_str().to_owned();
        let path = req.path().to_owned();

        if PUBLIC_ROUTES
            .iter()
            .any(|(m, p)| m == &method && p == &path)
        {
            let fut = self.service.call(req);
            return Box::pin(async move {
                let res = fut.await?;
                Ok(res.map_into_left_body())
            });
        }

        let token = req
            .headers()
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .map(|s| s.to_string());

        let mut req = Some(req);
        let service = self.service.clone();

        Box::pin(async move {
            let token = match token {
                Some(t) => t,
                None => {
                    warn!(%method, %path, "request rejected: missing authorization header");
                    return Ok(ServiceResponse::new(
                        req.take().unwrap().into_parts().0,
                        HttpResponse::Unauthorized()
                            .json(crate::errors::ErrorBody {
                                code: "unauthorized".to_string(),
                                message: "missing or invalid authorization header".to_string(),
                            })
                            .map_into_right_body(),
                    ));
                }
            };

            match state.token_validator.verify(&token).await {
                Ok(ctx) => {
                    info!(user_id = %ctx.user_id, roles = ?ctx.roles, %method, %path, "request authenticated");
                    let req = req.take().unwrap();
                    req.extensions_mut().insert(ctx);
                    let res = service.call(req).await?;
                    Ok(res.map_into_left_body())
                }
                Err(e) => {
                    warn!(error = %e, %method, %path, "request rejected: invalid token");
                    Ok(ServiceResponse::new(
                        req.take().unwrap().into_parts().0,
                        HttpResponse::Unauthorized()
                            .json(crate::errors::ErrorBody {
                                code: "unauthorized".to_string(),
                                message: "invalid or expired token".to_string(),
                            })
                            .map_into_right_body(),
                    ))
                }
            }
        })
    }
}

/// Route-level helper: require a specific scope.
pub fn require_scope(ctx: &AuthContext, scope: &str) -> Result<(), crate::errors::ApiError> {
    if !ctx.has_scope(scope) {
        warn!(user_id = %ctx.user_id, required = %scope, "request rejected: insufficient scope");
        return Err(crate::errors::ApiError::Forbidden);
    }
    Ok(())
}

/// Route-level helper: require ownership of the resource or an admin role.
pub fn require_owner_or_admin(
    ctx: &AuthContext,
    resource_id: uuid::Uuid,
) -> Result<(), crate::errors::ApiError> {
    if ctx.user_id == resource_id || ctx.has_role("admin") {
        return Ok(());
    }
    warn!(user_id = %ctx.user_id, resource_id = %resource_id, "request rejected: not owner or admin");
    Err(crate::errors::ApiError::Forbidden)
}
