use axum::{routing::{get, post}, Router};

use crate::relay;

pub fn set_relay_router() -> Router {
    Router::new().nest("/v1", set_relay_v1_router())
}
fn set_relay_v1_router() -> Router {
    Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .nest("/chat", set_relay_v1_chat_router())
}
fn set_relay_v1_chat_router() -> Router {
    Router::new().route("/completions", post(relay::openai::handle_completions))
}
