# 路由选择器概览

## 📖 简介

路由选择器（RouteSelector）是Berry API的新一代负载均衡接口，旨在简化负载均衡的使用方式，将复杂的负载均衡逻辑抽象为简单的线路选择和状态报告操作。

## 🎯 核心优势

### 1. 接口大幅简化
- **之前**: 需要管理多个组件和复杂的状态
- **现在**: 只需要两个核心操作：选择路由和报告结果

### 2. 更清晰的职责分离
- **路由选择器**: 专注负载均衡逻辑（健康检查、权重计算、故障转移）
- **业务代码**: 专注请求处理逻辑（请求构建、响应解析）

### 3. 更好的可测试性
- 易于创建 mock 实现
- 清晰的接口边界
- 独立的组件测试

## 📚 文档导航

### 🏗️ 设计和架构
- **[路由选择器设计方案](route-selector-design.md)** - 详细的设计思路和架构说明
- **[实现总结](route-selector-implementation.md)** - 完整的实现内容和验证结果

### 🔄 迁移和使用
- **[迁移指南](route-selector-migration.md)** - 从现有接口迁移到路由选择器的详细步骤

## 🚀 快速开始

### 1. 基本使用
```rust
use berry_loadbalance::{LoadBalanceService, LoadBalanceRouteSelector, RouteSelector};

// 创建路由选择器
let load_balancer = Arc::new(LoadBalanceService::new(config)?);
load_balancer.start().await?;

let route_selector: Arc<dyn RouteSelector> = 
    Arc::new(LoadBalanceRouteSelector::new(load_balancer));

// 选择路由
let route = route_selector.select_route("gpt-4", None).await?;

// 使用路由
let api_url = route.get_api_url("v1/chat/completions");
let api_key = route.get_api_key()?;

// 发送请求...
let result = send_request(&api_url, &api_key).await;

// 报告结果
match result {
    Ok(_) => {
        route_selector.report_result(
            &route.route_id,
            RouteResult::Success { latency }
        ).await;
    }
    Err(e) => {
        route_selector.report_result(
            &route.route_id,
            RouteResult::Failure { 
                error: e.to_string(),
                error_type: Some(RouteErrorType::Network)
            }
        ).await;
    }
}
```

### 2. 使用新的处理器
```rust
use berry_relay::relay::handler::RouteBasedHandler;

// 创建处理器
let handler = Arc::new(RouteBasedHandler::new(route_selector));

// 在HTTP路由中使用
.route("/v1/chat/completions", post(|
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    TypedHeader(content_type): TypedHeader<ContentType>,
    Json(body): Json<Value>
| async move {
    handler.handle_completions(
        TypedHeader(auth),
        TypedHeader(content_type),
        Json(body)
    ).await
}))
```

### 3. 强制选择特定后端
路由选择器支持通过 `backend` 参数强制选择特定的后端提供商：

```rust
// 在请求体中添加 backend 参数
let request_body = json!({
    "model": "gpt-4",
    "messages": [...],
    "backend": "openai"  // 强制使用 openai 提供商
});

// 或者直接调用选择器方法
let route = route_selector
    .select_specific_route("gpt-4", "openai")
    .await?;
```

**注意**: `backend` 参数会在转发给上游API之前被自动移除，不会影响实际的API调用。

## 📊 监控和统计

### 获取路由统计信息
```rust
let stats = route_selector.get_route_stats().await;

println!("总请求数: {}", stats.total_requests);
println!("成功率: {:.2}%", stats.success_rate() * 100.0);
println!("健康路由数: {}", stats.healthy_routes_count());

// 查看每个路由的详细信息
for (route_id, detail) in &stats.route_details {
    println!("路由 {}: 健康={}, 请求数={}, 错误数={}", 
        route_id, detail.is_healthy, detail.request_count, detail.error_count);
}
```

### 错误处理
```rust
match route_selector.select_route(model_name, user_tags).await {
    Ok(route) => {
        // 处理成功的路由选择
    }
    Err(e) => {
        println!("路由选择失败: {}", e.message);
        println!("总路由数: {}", e.total_routes);
        println!("健康路由数: {}", e.healthy_routes);
        
        // 查看失败的尝试
        for attempt in &e.failed_attempts {
            println!("失败尝试: {}:{} - {}", 
                attempt.provider, attempt.model, attempt.reason);
        }
    }
}
```

## 🔄 向后兼容

路由选择器完全向后兼容现有的 `LoadBalanceService` 接口：

- ✅ 现有代码无需修改
- ✅ 可以同时使用两种接口
- ✅ 渐进式迁移策略
- ✅ 所有现有功能保持不变

## 🧪 测试

### 单元测试示例
```rust
#[tokio::test]
async fn test_route_selector() {
    let route_selector = create_test_route_selector().await;
    
    // 测试路由选择
    let route = route_selector.select_route("test-model", None).await.unwrap();
    assert!(!route.route_id.is_empty());
    
    // 测试结果报告
    route_selector.report_result(
        &route.route_id,
        RouteResult::Success { latency: Duration::from_millis(100) }
    ).await;
    
    // 验证统计信息
    let stats = route_selector.get_route_stats().await;
    assert!(stats.total_requests > 0);
}
```

### Mock实现
```rust
struct MockRouteSelector {
    routes: Vec<SelectedRoute>,
}

#[async_trait]
impl RouteSelector for MockRouteSelector {
    async fn select_route(&self, _: &str, _: Option<&[String]>) 
        -> Result<SelectedRoute, RouteSelectionError> {
        Ok(self.routes[0].clone())
    }
    
    async fn report_result(&self, _: &str, _: RouteResult) {
        // Mock实现
    }
    
    async fn get_route_stats(&self) -> RouteStats {
        RouteStats::default()
    }
}
```

## 🎉 总结

路由选择器成功地将复杂的负载均衡逻辑抽象为简单易用的接口，同时保持了所有现有功能。它为Berry API提供了更好的开发体验和更清晰的架构设计。

建议新项目直接使用路由选择器接口，现有项目可以根据迁移指南逐步迁移。
