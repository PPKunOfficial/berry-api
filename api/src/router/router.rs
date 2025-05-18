use axum::{Router, routing::get};
use tower_http::trace::TraceLayer;

use super::{api::set_api_router, relay::set_relay_router};

pub fn set_router() -> Router {
    Router::new()
        .route("/", get(index))
        .merge(set_relay_router())
        .nest("/api", set_api_router())
        .layer(TraceLayer::new_for_http())
}
async fn index() -> &'static str {
    "Hello, World!"
}
