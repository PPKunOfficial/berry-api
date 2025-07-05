# 路由选择器迁移指南

## 概述

本指南介绍如何从现有的 `LoadBalanceService` 迁移到新的 `RouteSelector` 接口。

## 迁移优势

### 1. 接口简化
- **之前**: 需要管理多个组件和复杂的状态
- **现在**: 只需要两个核心操作：选择路由和报告结果

### 2. 更清晰的职责分离
- **路由选择器**: 专注负载均衡逻辑
- **业务代码**: 专注请求处理逻辑

### 3. 更好的错误处理
- 统一的错误类型和详细的错误信息
- 包含失败尝试记录，便于调试

## 迁移步骤

### 阶段1: 添加新依赖

在 `Cargo.toml` 中确保包含路由选择器相关类型：

```toml
[dependencies]
berry-loadbalance = { path = "../berry-loadbalance" }
berry-relay = { path = "../berry-relay" }
```

### 阶段2: 创建路由选择器

```rust
use berry_loadbalance::{LoadBalanceService, LoadBalanceRouteSelector, RouteSelector};

// 创建负载均衡服务（保持不变）
let load_balancer = Arc::new(LoadBalanceService::new(config)?);
load_balancer.start().await?;

// 创建路由选择器（新增）
let route_selector: Arc<dyn RouteSelector> = 
    Arc::new(LoadBalanceRouteSelector::new(load_balancer));
```

### 阶段3: 更新请求处理逻辑

#### 之前的代码：
```rust
// 1. 选择后端
let selected_backend = load_balancer
    .select_backend_with_user_tags(model_name, user_tags)
    .await?;

// 2. 构建请求
let api_url = selected_backend.get_api_url("v1/chat/completions");
let api_key = selected_backend.get_api_key()?;

// 3. 发送请求
let result = send_request(&api_url, &api_key).await;

// 4. 记录结果
match result {
    Ok(_) => {
        load_balancer.record_request_result(
            &selected_backend.backend.provider,
            &selected_backend.backend.model,
            RequestResult::Success { latency }
        ).await;
    }
    Err(e) => {
        load_balancer.record_request_result(
            &selected_backend.backend.provider,
            &selected_backend.backend.model,
            RequestResult::Failure { error: e.to_string() }
        ).await;
    }
}
```

#### 新的代码：
```rust
// 1. 选择路由
let route = route_selector
    .select_route(model_name, user_tags)
    .await?;

// 2. 构建请求
let api_url = route.get_api_url("v1/chat/completions");
let api_key = route.get_api_key()?;

// 3. 发送请求
let result = send_request(&api_url, &api_key).await;

// 4. 报告结果
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
                error_type: Some(classify_error(&e))
            }
        ).await;
    }
}
```

### 阶段4: 使用新的处理器

可以直接使用 `RouteBasedHandler` 来处理HTTP请求：

```rust
use berry_relay::relay::handler::RouteBasedHandler;

// 创建处理器
let handler = Arc::new(RouteBasedHandler::new(route_selector));

// 在路由中使用
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

### 阶段5: 使用强制后端选择功能

路由选择器支持通过 `backend` 参数强制选择特定的后端提供商：

#### HTTP请求中使用
```json
{
    "model": "gpt-4",
    "messages": [...],
    "backend": "openai"
}
```

#### 代码中直接使用
```rust
// 强制选择特定提供商
let route = route_selector
    .select_specific_route("gpt-4", "openai")
    .await?;

// 正常的路由选择
let route = route_selector
    .select_route("gpt-4", None)
    .await?;
```

**用途**:
- 调试特定提供商的问题
- 测试不同提供商的响应
- 临时绕过负载均衡逻辑

## 监控和调试

### 获取路由统计
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

## 测试

### 单元测试
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

## 性能考虑

1. **内存开销**: 新接口增加的内存开销很小
2. **运行时开销**: 适配器模式的间接调用开销可忽略
3. **优化建议**: 
   - 可以缓存 `route_id` 解析结果
   - 批量状态报告可以减少锁竞争

## 向后兼容

- 现有的 `LoadBalanceService` 接口保持不变
- 可以同时使用两种接口
- 渐进式迁移，不需要一次性修改所有代码

## 最佳实践

1. **错误分类**: 正确分类错误类型以获得更好的负载均衡效果
2. **及时报告**: 尽快报告请求结果，不要延迟
3. **监控统计**: 定期检查路由统计信息
4. **测试覆盖**: 为路由选择逻辑编写充分的测试

## 总结

路由选择器接口大大简化了负载均衡的使用方式，同时保持了所有现有功能。建议新项目直接使用路由选择器接口，现有项目可以渐进式迁移。
