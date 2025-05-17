use berry_api::router;
use tracing::Level;

const PROJECT_NAME: &str = "berry-api";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .init();

    tracing::info!("welcome to {}", PROJECT_NAME);

    let app = router::router::set_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    tracing::info!("listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
