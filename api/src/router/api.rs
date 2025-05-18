use axum::{Router, routing::get};

pub async fn set_api_router() -> Router {
    Router::new().route("/", get(|| async { "Api router" }))
}
