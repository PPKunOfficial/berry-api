//! Berry API Server
//!
//! Main entry point for the Berry API load balancing service

use berry_api::start_server;
use std::time::Duration;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 启动服务器并获取监听地址、服务器句柄和应用状态
    let (addr, server, app_state) = start_server().await?;
    tracing::info!("Server successfully started on http://{}", addr);

    // 启动后台任务来定期清理过时的后端指标，支持优雅关闭
    let app_state_for_cleanup = app_state.clone();
    let cleanup_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(900)); // 每15分钟清理一次
        loop {
            interval.tick().await;
            tracing::info!("Running cleanup task for stale backend metrics");
            app_state_for_cleanup
                .load_balancer
                .get_metrics()
                .cleanup_stale_backends(Duration::from_secs(3600)); // 清理超过1小时未更新的后端
        }
    });

    // 等待关闭信号
    shutdown_signal().await;
    tracing::info!("Received shutdown signal, starting graceful shutdown...");

    // 优雅关闭服务器
    let server_handle = tokio::spawn(server);
    
    // 优雅关闭后台任务
    cleanup_handle.abort();
    tracing::info!("Cleanup task aborted");

    // 等待服务器优雅关闭
    if let Err(e) = server_handle.await {
        tracing::error!("Server shutdown error: {}", e);
    }

    // 优雅关闭应用状态
    app_state.shutdown().await;

    Ok(())
}

/// 优雅的关闭信号处理器
async fn shutdown_signal() {
    use signal::unix::{signal, SignalKind};

    let mut sigint = match signal(SignalKind::interrupt()) {
        Ok(sig) => sig,
        Err(e) => {
            tracing::error!("Failed to install SIGINT handler: {}", e);
            return;
        }
    };

    let mut sigterm = match signal(SignalKind::terminate()) {
        Ok(sig) => sig,
        Err(e) => {
            tracing::error!("Failed to install SIGTERM handler: {}", e);
            return;
        }
    };

    tokio::select! {
        _ = sigint.recv() => {
            tracing::info!("Received SIGINT, shutting down gracefully...");
        }
        _ = sigterm.recv() => {
            tracing::info!("Received SIGTERM, shutting down gracefully...");
        }
        _ = signal::ctrl_c() => {
            tracing::info!("Received Ctrl+C, shutting down gracefully...");
        }
    }
}
