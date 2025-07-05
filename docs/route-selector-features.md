# 路由选择器功能特性

## 🎯 核心功能

路由选择器提供了完整的负载均衡功能，包括自动路由选择和强制后端选择两种模式。

### 1. 自动路由选择
根据负载均衡策略自动选择最佳后端：

```rust
// 基本的自动路由选择
let route = route_selector.select_route("gpt-4", None).await?;

// 带用户标签的路由选择
let user_tags = vec!["premium".to_string(), "fast".to_string()];
let route = route_selector.select_route("gpt-4", Some(&user_tags)).await?;
```

### 2. 强制后端选择
直接指定特定的后端提供商：

```rust
// 强制选择 OpenAI 提供商
let route = route_selector.select_specific_route("gpt-4", "openai").await?;

// 强制选择 Anthropic 提供商
let route = route_selector.select_specific_route("claude-3", "anthropic").await?;
```

## 🌐 HTTP API 支持

### 通过请求体参数强制选择后端

在 HTTP 请求中添加 `backend` 参数来强制选择特定提供商：

```json
{
    "model": "gpt-4",
    "messages": [
        {"role": "user", "content": "Hello!"}
    ],
    "backend": "openai"
}
```

**重要特性**：
- `backend` 参数会在转发给上游API之前被自动移除
- 不会影响实际的API调用
- 支持所有配置中的提供商名称

### 示例请求

#### 正常负载均衡请求
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

#### 强制选择 OpenAI
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello!"}],
    "backend": "openai"
  }'
```

#### 强制选择 Anthropic
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "model": "claude-3-opus",
    "messages": [{"role": "user", "content": "Hello!"}],
    "backend": "anthropic"
  }'
```

## 🔧 使用场景

### 1. 调试和测试
```rust
// 测试特定提供商的响应
let openai_route = route_selector.select_specific_route("gpt-4", "openai").await?;
let anthropic_route = route_selector.select_specific_route("claude-3", "anthropic").await?;

// 比较不同提供商的性能
let start_time = Instant::now();
let response = send_request_to_route(&openai_route, &request).await?;
let latency = start_time.elapsed();
```

### 2. 故障转移
```rust
// 主要提供商失败时，尝试备用提供商
let route = match route_selector.select_route("gpt-4", None).await {
    Ok(route) => route,
    Err(_) => {
        // 如果自动选择失败，尝试特定的备用提供商
        route_selector.select_specific_route("gpt-4", "backup-provider").await?
    }
};
```

### 3. A/B 测试
```rust
// 根据用户ID决定使用哪个提供商
let provider = if user_id % 2 == 0 { "openai" } else { "anthropic" };
let route = route_selector.select_specific_route("gpt-4", provider).await?;
```

### 4. 成本优化
```rust
// 优先使用成本较低的提供商
let route = match route_selector.select_specific_route("gpt-4", "cost-effective-provider").await {
    Ok(route) => route,
    Err(_) => {
        // 如果低成本提供商不可用，回退到正常负载均衡
        route_selector.select_route("gpt-4", None).await?
    }
};
```

## 📊 监控和统计

### 获取路由统计信息
```rust
let stats = route_selector.get_route_stats().await;

println!("总请求数: {}", stats.total_requests);
println!("成功率: {:.2}%", stats.success_rate() * 100.0);
println!("健康路由数: {}", stats.healthy_routes_count());

// 查看每个路由的详细信息
for (route_id, detail) in &stats.route_details {
    println!("路由 {}: 提供商={}, 模型={}, 健康={}, 请求数={}", 
        route_id, detail.provider, detail.model, detail.is_healthy, detail.request_count);
}
```

### HTTP 监控端点
```bash
# 获取路由统计信息
curl http://localhost:8080/v1/routes/stats

# 响应示例
{
    "total_requests": 1000,
    "successful_requests": 950,
    "success_rate": 0.95,
    "healthy_routes_count": 3,
    "routes": [
        {
            "route_id": "openai:gpt-4",
            "provider": "openai",
            "model": "gpt-4",
            "is_healthy": true,
            "request_count": 500,
            "error_count": 10,
            "average_latency_ms": 150,
            "current_weight": 1.0
        }
    ]
}
```

## ⚠️ 错误处理

### 路由选择失败
```rust
match route_selector.select_route("gpt-4", None).await {
    Ok(route) => {
        // 使用路由
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

### 特定提供商不可用
```rust
match route_selector.select_specific_route("gpt-4", "unavailable-provider").await {
    Ok(route) => {
        // 使用路由
    }
    Err(e) => {
        if e.message.contains("not found") {
            // 提供商不存在，回退到自动选择
            let route = route_selector.select_route("gpt-4", None).await?;
        } else {
            // 其他错误
            return Err(e.into());
        }
    }
}
```

## 🧪 测试支持

### Mock 实现
```rust
use berry_loadbalance::route_selector::MockRouteSelector;

#[tokio::test]
async fn test_backend_selection() {
    let routes = vec![
        MockRouteSelector::create_test_route("openai:gpt-4", "openai", "gpt-4"),
        MockRouteSelector::create_test_route("anthropic:claude", "anthropic", "claude-3"),
    ];

    let selector = MockRouteSelector::new(routes);

    // 测试自动选择
    let route = selector.select_route("gpt-4", None).await.unwrap();
    assert_eq!(route.provider.name, "openai");

    // 测试强制选择
    let route = selector.select_specific_route("claude", "anthropic").await.unwrap();
    assert_eq!(route.provider.name, "anthropic");
}
```

## 🔒 安全考虑

1. **参数验证**: `backend` 参数会被验证，只允许配置中存在的提供商
2. **权限控制**: 可以在应用层添加权限检查，限制某些用户使用特定提供商
3. **审计日志**: 所有强制选择的请求都会被记录在日志中

## 📈 性能影响

- **自动选择**: 无额外性能开销
- **强制选择**: 跳过负载均衡算法，性能略有提升
- **参数处理**: JSON 解析和参数移除的开销可忽略不计

## 🎉 总结

路由选择器的强制后端选择功能提供了：

1. **灵活性** - 支持自动和手动两种选择模式
2. **易用性** - 简单的 API 和 HTTP 参数支持
3. **调试友好** - 便于测试和故障排查
4. **生产就绪** - 完整的错误处理和监控支持
5. **向后兼容** - 不影响现有的负载均衡功能

这个功能为开发者提供了完全的控制权，既可以享受自动负载均衡的便利，也可以在需要时精确控制请求路由。
