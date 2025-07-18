//! Berry API Server
//!
//! Main entry point for the Berry API load balancing service

use berry_api::start_server;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app_state = start_server().await?;

    // 启动一个后台任务来定期清理过时的后端指标
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(900)); // 每15分钟清理一次
        loop {
            interval.tick().await;
            tracing::info!("Running cleanup task for stale backend metrics");
            app_state
                .load_balancer
                .get_metrics()
                .cleanup_stale_backends(Duration::from_secs(3600)); // 清理超过1小时未更新的后端
        }
    });

    // 保持主线程运行
    tokio::signal::ctrl_c().await?;
    Ok(())
}
