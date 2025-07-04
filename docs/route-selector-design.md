# 线路选择器设计方案

## 概述

将负载均衡抽象为线路选择器（RouteSelector），简化调用接口，将复杂的负载均衡逻辑封装在选择器内部。

## 设计目标

1. **简化接口** - 调用方只需要关心两个核心操作：选择线路和报告状态
2. **解耦复杂性** - 将健康检查、权重计算、故障转移等复杂逻辑封装在选择器内部
3. **清晰的职责分离** - 选择器专注于线路选择，外部专注于请求处理
4. **更好的可测试性** - 可以独立测试选择器逻辑，也可以mock选择器进行上层测试

## 核心接口

### RouteSelector Trait

```rust
#[async_trait]
pub trait RouteSelector: Send + Sync {
    /// 选择线路 - 根据模型名称和用户标签选择最佳后端线路
    async fn select_route(
        &self,
        model_name: &str,
        user_tags: Option<&[String]>,
    ) -> Result<SelectedRoute, RouteSelectionError>;

    /// 选择指定提供商的线路（用于调试和测试）
    async fn select_specific_route(
        &self,
        model_name: &str,
        provider_name: &str,
    ) -> Result<SelectedRoute, RouteSelectionError>;

    /// 报告请求结果 - 选择器了解线路状态的唯一方式
    async fn report_result(&self, route_id: &str, result: RouteResult);

    /// 获取线路统计信息（用于监控）
    async fn get_route_stats(&self) -> RouteStats;
}
```

## 使用方式对比

### 当前方式（复杂）

```rust
// 1. 选择后端
let selected_backend = load_balancer.select_backend_with_user_tags(model_name, user_tags).await?;

// 2. 构建请求
let api_url = selected_backend.get_api_url("v1/chat/completions");
let api_key = selected_backend.get_api_key()?;
let timeout = selected_backend.get_timeout();

// 3. 发送请求
let result = send_request(&api_url, &api_key, timeout).await;

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

### 新方式（简化）

```rust
// 1. 选择线路
let route = route_selector.select_route(model_name, user_tags).await?;

// 2. 构建请求
let api_url = route.get_api_url("v1/chat/completions");
let api_key = route.get_api_key()?;
let timeout = route.get_timeout();

// 3. 发送请求
let result = send_request(&api_url, &api_key, timeout).await;

// 4. 报告结果
match result {
    Ok(_) => {
        route_selector.report_result(&route.route_id, RouteResult::Success { latency }).await;
    }
    Err(e) => {
        route_selector.report_result(&route.route_id, RouteResult::Failure { 
            error: e.to_string(),
            error_type: Some(RouteErrorType::Network)
        }).await;
    }
}
```

## 优势分析

### 1. 接口简化
- **之前**: 需要了解 `LoadBalanceService`、`SelectedBackend`、`RequestResult` 等多个类型
- **现在**: 只需要了解 `RouteSelector`、`SelectedRoute`、`RouteResult` 三个核心类型

### 2. 错误处理改进
- **之前**: 错误信息分散在不同层次，难以统一处理
- **现在**: 统一的 `RouteSelectionError` 提供详细的错误信息和失败尝试记录

### 3. 状态管理简化
- **之前**: 需要手动管理 provider 和 model 信息用于状态报告
- **现在**: 使用 `route_id` 统一标识，避免信息丢失

### 4. 监控能力增强
- **之前**: 监控信息分散在多个组件中
- **现在**: 统一的 `RouteStats` 提供完整的监控视图

## 实现策略

### 1. 渐进式迁移
- 保持现有 `LoadBalanceService` 不变
- `LoadBalanceRouteSelector` 作为适配器包装现有服务
- 新代码使用 `RouteSelector` 接口
- 旧代码可以继续使用现有接口

### 2. 向后兼容
- 所有现有功能通过适配器保持可用
- 现有的健康检查、权重计算、故障转移逻辑不变
- 配置格式和行为保持一致

### 3. 测试友好
```rust
// 可以轻松创建 mock 实现用于测试
struct MockRouteSelector {
    routes: Vec<SelectedRoute>,
    current_index: AtomicUsize,
}

#[async_trait]
impl RouteSelector for MockRouteSelector {
    async fn select_route(&self, _: &str, _: Option<&[String]>) -> Result<SelectedRoute, RouteSelectionError> {
        // 返回预定义的路由
    }
    
    async fn report_result(&self, _: &str, _: RouteResult) {
        // 记录调用用于验证
    }
}
```

## 性能影响

### 1. 内存开销
- 新增的类型结构相对轻量
- 适配器模式增加一层间接调用，但开销很小

### 2. 运行时开销
- `route_id` 的解析增加少量字符串操作
- 类型转换的开销可以忽略不计

### 3. 优化空间
- 可以缓存 `route_id` 到 `(provider, model)` 的映射
- 批量状态报告可以减少锁竞争

## 迁移建议

### 阶段1：引入新接口
- ✅ 实现 `RouteSelector` trait 和相关类型
- ✅ 实现 `LoadBalanceRouteSelector` 适配器
- ✅ 添加使用示例和文档

### 阶段2：局部迁移
- 选择一个模块（如 `berry-relay`）开始使用新接口
- 验证功能正确性和性能表现
- 收集使用反馈

### 阶段3：全面推广
- 逐步迁移其他模块到新接口
- 优化性能热点
- 完善监控和调试工具

### 阶段4：清理优化
- 评估是否需要保留旧接口
- 优化内部实现
- 完善文档和最佳实践

## 总结

线路选择器方案成功地将复杂的负载均衡逻辑抽象为简单的选择和报告接口，具有以下优势：

1. **接口简化** - 从多个复杂接口简化为单一清晰接口
2. **职责分离** - 选择器专注负载均衡，调用方专注业务逻辑
3. **错误处理** - 统一的错误类型和详细的错误信息
4. **可测试性** - 易于mock和单元测试
5. **向后兼容** - 不破坏现有代码
6. **渐进迁移** - 可以逐步采用新接口

这个方案很好地平衡了简化接口和保持功能完整性的需求，建议采用。
