use axum::{extract::State, routing::post, Json, Router};
use serde::Deserialize;
use std::sync::Arc;
use crate::error::ApiError;
use crate::middleware::auth::{Claims, create_token};
use crate::response::response::Response;
use domain::user::service::UserService;
use domain::user::entity::UserStatus;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub tenant_id: String,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Clone)]
pub struct AuthHandler {
    pub user_service: UserService,
}

impl AuthHandler {
    pub fn new(user_service: UserService) -> Self {
        Self { user_service }
    }
}

pub fn routes(handler: Arc<AuthHandler>) -> Router {
    Router::new()
        .route("/login", post(login))
        .route("/register", post(register))
        .with_state(handler)
}

async fn login(
    State(handler): State<Arc<AuthHandler>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<Response<serde_json::Value>>, ApiError> {
    let user = handler
        .user_service
        .get_user_by_username(&req.username)
        .await
        .map_err(|_| ApiError::bad_request("Invalid username or password"))?;

    if user.status != UserStatus::Active {
        return Err(ApiError::bad_request("User is disabled"));
    }

    let valid = bcrypt::verify(&req.password, &user.password_hash)
        .map_err(|_| ApiError::internal("Password verification failed"))?;

    if !valid {
        return Err(ApiError::bad_request("Invalid username or password"));
    }

    let role = if user.is_super_admin {
        "SuperAdmin".to_string()
    } else {
        let user_role = handler
            .user_service
            .get_role(&user.user_id, &req.tenant_id)
            .await
            .map_err(|_| ApiError::bad_request("User does not belong to this tenant"))?;
        format!("{}", user_role.role)
    };

    let exp = chrono::Utc::now().timestamp() as usize + 86400; // 24h
    let claims = Claims {
        sub: user.user_id.clone(),
        username: user.username.clone(),
        is_super_admin: user.is_super_admin,
        tenant_id: req.tenant_id,
        role,
        exp,
    };

    let token = create_token(&claims)
        .map_err(|e| ApiError::internal(format!("Token creation failed: {}", e)))?;

    Ok(Json(Response::success(serde_json::json!({
        "token": token,
        "user_id": user.user_id,
        "username": user.username,
    }))))
}

async fn register(
    State(handler): State<Arc<AuthHandler>>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<Response<serde_json::Value>>, ApiError> {
    let password_hash = bcrypt::hash(&req.password, bcrypt::DEFAULT_COST)
        .map_err(|e| ApiError::internal(format!("Password hashing failed: {}", e)))?;

    let user = handler
        .user_service
        .create_user(req.username, req.email, password_hash, false)
        .await?;

    Ok(Json(Response::success(serde_json::json!({
        "user_id": user.user_id,
        "username": user.username,
    }))))
}
