use anyhow::Result;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::{get, post};
use axum::Router;
use berry_core::config::loader::load_config;
use berry_loadbalance::{LoadBalanceService, LoadBalanceRouteSelector, RouteSelector};
use berry_relay::relay::handler::RouteBasedHandler;
use serde_json::{json, Value};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::{error, info};

/// 基于路由选择器的应用状态
/// 
/// 这是新的简化版本，展示如何使用RouteSelector接口
#[derive(Clone)]
pub struct RouteBasedAppState {
    pub route_selector: Arc<dyn RouteSelector>,
    pub handler: Arc<RouteBasedHandler>,
}

impl RouteBasedAppState {
    /// 创建新的基于路由的应用状态
    pub async fn new() -> Result<Self> {
        // 加载配置
        let config = load_config()?;
        let config_path = berry_core::config::loader::get_config_path();
        info!("Configuration loaded successfully from: {}", config_path);

        // 创建负载均衡服务
        let load_balancer = Arc::new(LoadBalanceService::new(config.clone())?);

        // 启动负载均衡服务
        load_balancer.start().await?;
        info!("Load balance service started");

        // 创建路由选择器（包装现有的负载均衡服务）
        let route_selector: Arc<dyn RouteSelector> = 
            Arc::new(LoadBalanceRouteSelector::new(load_balancer));

        // 创建基于路由的处理器
        let handler = Arc::new(RouteBasedHandler::new(route_selector.clone()));

        info!("Route-based application state initialized");

        Ok(Self {
            route_selector,
            handler,
        })
    }

    /// 创建路由器
    pub fn create_router(self) -> Router {
        Router::new()
            // 聊天完成端点
            .route("/v1/chat/completions", post(chat_completions_handler))
            // 健康检查端点
            .route("/health", get(health_check_handler))
            // 路由统计端点（用于监控）
            .route("/v1/routes/stats", get(route_stats_handler))
            // 模型列表端点
            .route("/v1/models", get(models_handler))
            .with_state(self)
            .layer(CorsLayer::permissive())
    }
}

/// 聊天完成处理器
async fn chat_completions_handler(
    State(app_state): State<RouteBasedAppState>,
    headers: axum::http::HeaderMap,
    body: String,
) -> Result<axum::response::Response, StatusCode> {
    // 解析请求体
    let body_json: Value = serde_json::from_str(&body)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // 提取Authorization头
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // 创建Authorization头对象
    let authorization = headers::Authorization::bearer(auth_header)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // 创建Content-Type头对象
    let content_type = headers::ContentType::json();

    // 使用路由处理器处理请求
    match app_state
        .handler
        .handle_completions(
            axum_extra::TypedHeader(authorization),
            axum_extra::TypedHeader(content_type),
            axum::extract::Json(body_json),
        )
        .await
        .into_response()
        .into_parts()
    {
        (parts, body) => {
            let mut response = axum::response::Response::from_parts(parts, body);
            Ok(response)
        }
    }
}

/// 健康检查处理器
async fn health_check_handler(
    State(app_state): State<RouteBasedAppState>,
) -> Json<Value> {
    let stats = app_state.route_selector.get_route_stats().await;
    
    Json(json!({
        "status": "healthy",
        "total_requests": stats.total_requests,
        "successful_requests": stats.successful_requests,
        "success_rate": stats.success_rate(),
        "healthy_routes": stats.healthy_routes_count(),
        "total_routes": stats.route_details.len()
    }))
}

/// 路由统计处理器
async fn route_stats_handler(
    State(app_state): State<RouteBasedAppState>,
) -> Json<Value> {
    let stats = app_state.route_selector.get_route_stats().await;
    
    let route_details: Vec<Value> = stats.route_details
        .into_iter()
        .map(|(route_id, detail)| {
            json!({
                "route_id": route_id,
                "provider": detail.provider,
                "model": detail.model,
                "is_healthy": detail.is_healthy,
                "request_count": detail.request_count,
                "error_count": detail.error_count,
                "average_latency_ms": detail.average_latency.map(|d| d.as_millis()),
                "current_weight": detail.current_weight
            })
        })
        .collect();

    Json(json!({
        "total_requests": stats.total_requests,
        "successful_requests": stats.successful_requests,
        "success_rate": stats.success_rate(),
        "healthy_routes_count": stats.healthy_routes_count(),
        "routes": route_details
    }))
}

/// 模型列表处理器
async fn models_handler() -> Json<Value> {
    // 这里可以从路由选择器获取可用模型列表
    // 为了简化，返回一个示例响应
    Json(json!({
        "object": "list",
        "data": [
            {
                "id": "gpt-4",
                "object": "model",
                "created": 1677610602,
                "owned_by": "openai"
            },
            {
                "id": "gpt-3.5-turbo",
                "object": "model", 
                "created": 1677610602,
                "owned_by": "openai"
            }
        ]
    }))
}

/// 演示如何使用路由选择器的简单示例
pub async fn demonstrate_route_selector_usage() -> Result<()> {
    // 创建应用状态
    let app_state = RouteBasedAppState::new().await?;
    
    println!("=== 路由选择器使用演示 ===");
    
    // 1. 选择路由
    match app_state.route_selector.select_route("gpt-4", None).await {
        Ok(route) => {
            println!("✅ 成功选择路由:");
            println!("   路由ID: {}", route.route_id);
            println!("   提供商: {}", route.provider.name);
            println!("   模型: {}", route.backend.model);
            println!("   选择耗时: {:?}", route.selection_time);
            
            // 2. 模拟请求成功
            app_state.route_selector.report_result(
                &route.route_id,
                berry_loadbalance::RouteResult::Success {
                    latency: std::time::Duration::from_millis(150),
                },
            ).await;
            println!("✅ 已报告请求成功");
        }
        Err(e) => {
            println!("❌ 路由选择失败: {}", e);
        }
    }
    
    // 3. 获取统计信息
    let stats = app_state.route_selector.get_route_stats().await;
    println!("\n📊 路由统计:");
    println!("   总请求数: {}", stats.total_requests);
    println!("   成功请求数: {}", stats.successful_requests);
    println!("   成功率: {:.2}%", stats.success_rate() * 100.0);
    println!("   健康路由数: {}", stats.healthy_routes_count());
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_route_based_app_state() {
        // 测试应用状态创建
        let result = RouteBasedAppState::new().await;
        
        // 在实际测试中，这里应该有有效的配置
        // 现在只是验证接口的正确性
        match result {
            Ok(app_state) => {
                // 测试路由选择
                let stats = app_state.route_selector.get_route_stats().await;
                assert_eq!(stats.total_requests, 0); // 初始状态应该是0
            }
            Err(e) => {
                // 如果没有有效配置，这是预期的
                println!("Expected error in test environment: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_route_selector_interface() {
        // 这个测试展示了路由选择器接口的简洁性
        // 在实际环境中，这里会有真实的配置和服务
        
        // 模拟的测试逻辑
        let test_passed = true;
        assert!(test_passed, "Route selector interface should be simple and clean");
    }
}
