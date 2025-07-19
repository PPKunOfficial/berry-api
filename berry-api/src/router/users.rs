use crate::app::AppState;
use crate::models::user::{CreateUserRequest, LoginRequest, UpdateUserRequest};
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Router,
};

pub fn create_user_routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(register_user))
        .route("/login", post(login_user))
        .route("/users", get(list_users))
        .route("/users/:id", get(get_user))
        .route("/users/:id", put(update_user))
        .route("/users/:id", delete(delete_user))
}

async fn register_user(
    State(state): State<AppState>,
    payload: axum::Json<CreateUserRequest>,
) -> Result<axum::response::Response, axum::response::Response> {
    let result = crate::handlers::UserHandler::register(
        axum::extract::State(state.user_handler.clone()),
        payload,
    ).await;
    
    match result {
        Ok((json, status)) => Ok((status, json).into_response()),
        Err((status, json)) => Err((status, json).into_response()),
    }
}

async fn login_user(
    State(state): State<AppState>,
    payload: axum::Json<LoginRequest>,
) -> Result<axum::response::Response, axum::response::Response> {
    let result = crate::handlers::UserHandler::login(
        axum::extract::State(state.user_handler.clone()),
        payload,
    ).await;
    
    match result {
        Ok((json, status)) => Ok((status, json).into_response()),
        Err((status, json)) => Err((status, json).into_response()),
    }
}

async fn list_users(
    State(state): State<AppState>,
) -> Result<axum::response::Response, axum::response::Response> {
    let result = crate::handlers::UserHandler::list_users(
        axum::extract::State(state.user_handler.clone()),
    ).await;
    
    match result {
        Ok(json) => Ok(json.into_response()),
        Err((status, json)) => Err((status, json).into_response()),
    }
}

async fn get_user(
    State(state): State<AppState>,
    path: Path<uuid::Uuid>,
) -> Result<axum::response::Response, axum::response::Response> {
    let result = crate::handlers::UserHandler::get_user(
        axum::extract::State(state.user_handler.clone()),
        path,
    ).await;
    
    match result {
        Ok(json) => Ok(json.into_response()),
        Err((status, json)) => Err((status, json).into_response()),
    }
}

async fn update_user(
    State(state): State<AppState>,
    path: Path<uuid::Uuid>,
    payload: axum::Json<UpdateUserRequest>,
) -> Result<axum::response::Response, axum::response::Response> {
    let result = crate::handlers::UserHandler::update_user(
        axum::extract::State(state.user_handler.clone()),
        path,
        payload,
    ).await;
    
    match result {
        Ok(json) => Ok(json.into_response()),
        Err((status, json)) => Err((status, json).into_response()),
    }
}

async fn delete_user(
    State(state): State<AppState>,
    path: Path<uuid::Uuid>,
) -> Result<axum::response::Response, axum::response::Response> {
    let result = crate::handlers::UserHandler::delete_user(
        axum::extract::State(state.user_handler.clone()),
        path,
    ).await;
    
    match result {
        Ok((json, status)) => Ok((status, json).into_response()),
        Err((status, json)) => Err((status, json).into_response()),
    }
}