pub mod config;
pub mod relay;
pub mod router;

use tracing::Level;

use crate::config::loader::load_config;

const PROJECT_NAME: &str = "berry-api";

#[tokio::main]
pub async fn start() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .with_file(true)
        .with_line_number(true)
        .init();

    tracing::info!("welcome to {}", PROJECT_NAME);

    let config = load_config();
    match config {
        Ok(config) => {
            tracing::info!("config loaded: {:?}", config);
        }
        Err(e) => {
            tracing::error!("load config error: {}", e);
        }
    }

    let app = router::router::set_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    tracing::info!("listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
