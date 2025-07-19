use crate::models::user::{CreateUserRequest, LoginRequest, UpdateUserRequest, UserResponse};
use crate::repositories::user::UserRepository;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct UserHandler {
    repo: UserRepository,
}

impl UserHandler {
    pub fn new(repo: UserRepository) -> Self {
        UserHandler { repo }
    }

    pub async fn register(
        State(handler): State<Arc<UserHandler>>,
        Json(payload): Json<CreateUserRequest>,
    ) -> Result<(Json<serde_json::Value>, StatusCode), (StatusCode, Json<serde_json::Value>)> {
        // 检查用户名是否已存在
        if handler.repo.find_by_username(&payload.username).await.map_err(|_| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"})))
        })?.is_some() {
            return Err((StatusCode::CONFLICT, Json(json!({"error": "Username already exists"}))));
        }

        // 检查邮箱是否已存在
        if handler.repo.find_by_email(&payload.email).await.map_err(|_| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"})))
        })?.is_some() {
            return Err((StatusCode::CONFLICT, Json(json!({"error": "Email already exists"}))));
        }

        let user = handler.repo
            .create(
                &payload.username,
                &payload.email,
                &payload.password,
                payload.full_name.as_deref(),
            )
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to create user"}))))?;

        Ok((Json(json!({
            "message": "User created successfully",
            "user": UserResponse::from(user)
        })), StatusCode::CREATED))
    }

    pub async fn login(
        State(handler): State<Arc<UserHandler>>,
        Json(payload): Json<LoginRequest>,
    ) -> Result<(Json<serde_json::Value>, StatusCode), (StatusCode, Json<serde_json::Value>)> {
        let user = handler.repo
            .verify_password(&payload.username,
                &payload.password,
            )
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"}))))?;

        let user = user.ok_or_else(|| {
            (StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid credentials"})))
        })?;

        if !user.is_active {
            return Err((StatusCode::FORBIDDEN, Json(json!({"error": "Account is disabled"}))));
        }

        // 更新最后登录时间
        handler.repo.update_last_login(user.id).await.map_err(|_| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to update last login"})))
        })?;

        // 这里应该生成JWT token，暂时返回user对象
        Ok((Json(json!({
            "message": "Login successful",
            "user": UserResponse::from(user),
            "token": "placeholder_token" // TODO: 实现JWT
        })), StatusCode::OK))
    }

    pub async fn get_user(
        State(handler): State<Arc<UserHandler>>,
        Path(id): Path<Uuid>,
    ) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
        let user = handler.repo.find_by_id(id).await.map_err(|_| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"})))
        })?;

        let user = user.ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(json!({"error": "User not found"})))
        })?;

        Ok(Json(json!({"user": UserResponse::from(user)})))
    }

    pub async fn update_user(
        State(handler): State<Arc<UserHandler>>,
        Path(id): Path<Uuid>,
        Json(payload): Json<UpdateUserRequest>,
    ) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
        let user = handler.repo.update(id, &payload).await.map_err(|_| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"})))
        })?;

        let user = user.ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(json!({"error": "User not found"})))
        })?;

        Ok(Json(json!({
            "message": "User updated successfully",
            "user": UserResponse::from(user)
        })))
    }

    pub async fn delete_user(
        State(handler): State<Arc<UserHandler>>,
        Path(id): Path<Uuid>,
    ) -> Result<(Json<serde_json::Value>, StatusCode), (StatusCode, Json<serde_json::Value>)> {
        let deleted = handler.repo.delete(id).await.map_err(|_| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"})))
        })?;

        if deleted {
            Ok((Json(json!({"message": "User deleted successfully"})), StatusCode::OK))
        } else {
            Err((StatusCode::NOT_FOUND, Json(json!({"error": "User not found"}))))
        }
    }

    pub async fn list_users(
        State(handler): State<Arc<UserHandler>>,
    ) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
        let users = handler.repo.list(100, 0).await.map_err(|_| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"})))
        })?;

        let user_responses: Vec<UserResponse> = users.into_iter().map(UserResponse::from).collect();

        Ok(Json(json!({"users": user_responses})))
    }
}