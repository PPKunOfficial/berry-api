use axum::{
    Router,
    routing::{get, post},
};

use crate::relay;

pub fn set_relay_router() -> Router {
    Router::new().nest("/v1", set_relay_v1_router())
}
fn set_relay_v1_router() -> Router {
    Router::new()
        .route(
            "/chat/completions",
            post(relay::openai::completions::handle_completions),
        )
        .route("/models", get(relay::openai::model::handle_model))
}
