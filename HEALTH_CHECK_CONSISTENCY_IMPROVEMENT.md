# 健康检查一致性改进

## 概述

本次改进解决了健康检查和恢复检查之间的不一致性问题。之前的逻辑是：
- 常规健康检查使用 model list API
- 但恢复检查时会切换到 chat 请求

现在改进为：**如果是通过 model list 发现不可用的，恢复时也继续使用 model list 检查**，保持检查方式的一致性。

## 问题分析

### 之前的问题

1. **检查方式不一致**：常规检查用 model list，恢复检查用 chat
2. **逻辑混乱**：同一个后端可能因为不同的检查方式得到不同的结果
3. **成本浪费**：不必要的 chat 请求增加了成本
4. **调试困难**：不同的检查方式让问题排查变得复杂

### 改进目标

- ✅ 保持检查方式的一致性
- ✅ 减少不必要的 chat 请求
- ✅ 提高系统的可预测性
- ✅ 简化调试过程

## 技术实现

### 1. 新增健康检查方式枚举

```rust
/// 健康检查方式
#[derive(Debug, Clone, PartialEq)]
pub enum HealthCheckMethod {
    /// 使用model list API检查
    ModelList,
    /// 使用chat请求检查
    Chat,
    /// 网络连接检查
    Network,
}
```

### 2. 扩展不健康后端信息结构

```rust
/// 不健康后端信息
#[derive(Debug, Clone)]
pub struct UnhealthyBackend {
    pub backend_key: String,
    pub first_failure_time: Instant,
    pub last_failure_time: Instant,
    pub failure_count: u32,
    pub last_recovery_attempt: Option<Instant>,
    pub recovery_attempts: u32,
    /// 记录导致不健康的检查方式，用于恢复时保持一致性
    pub failure_check_method: HealthCheckMethod,
}
```

### 3. 新增带检查方式的失败记录方法

```rust
/// 记录请求失败（带检查方式）
pub fn record_failure_with_method(&self, backend_key: &str, check_method: HealthCheckMethod)
```

### 4. 智能恢复检查逻辑

```rust
// 根据失败检查方式和计费模式选择恢复方式
match (&unhealthy_backend.failure_check_method, backend_billing_mode) {
    (HealthCheckMethod::ModelList, BillingMode::PerToken) => {
        // 使用model list进行恢复检查，保持一致性
        self.check_recovery_with_model_list(provider_id, provider, model_name).await;
    }
    (HealthCheckMethod::Chat, BillingMode::PerToken) => {
        // 使用chat请求进行恢复检查
        self.check_recovery_with_chat(provider_id, provider, model_name).await;
    }
    (HealthCheckMethod::Network, BillingMode::PerToken) => {
        // 网络错误通常用model list检查恢复
        self.check_recovery_with_model_list(provider_id, provider, model_name).await;
    }
    (_, BillingMode::PerRequest) => {
        // 按请求计费：跳过主动恢复检查，依赖被动验证
        self.metrics.record_recovery_attempt(&unhealthy_backend.backend_key);
    }
}
```

### 5. 新增 model list 恢复检查方法

```rust
/// 使用model list API检查provider恢复状态
async fn check_recovery_with_model_list(
    &self,
    provider_id: &str,
    provider: &Provider,
    model_name: &str,
) {
    // 发送models API请求进行恢复检查
    // 保持与常规健康检查的一致性
}
```

## 改进效果

### 🎯 **一致性提升**

**改进前**：
```
常规检查: model list API → 失败 → 标记不健康
恢复检查: chat 请求 → 成功 → 标记健康
结果: 可能出现逻辑不一致
```

**改进后**：
```
常规检查: model list API → 失败 → 标记不健康(记录检查方式)
恢复检查: model list API → 成功 → 标记健康
结果: 检查方式完全一致
```

### 💰 **成本优化**

- **减少不必要的 chat 请求**：只有真正需要时才使用 chat 检查
- **避免重复检查**：同一种检查方式贯穿整个生命周期
- **智能选择**：根据失败原因选择最合适的恢复方式

### 🔍 **调试改善**

- **清晰的检查轨迹**：可以追踪每个后端使用的检查方式
- **一致的日志格式**：所有相关日志都包含检查方式信息
- **可预测的行为**：系统行为更加可预测和可理解

## 使用场景示例

### 场景1：Model List 检查失败

```
1. 常规健康检查: GET /v1/models → 401 Unauthorized
2. 记录失败: failure_check_method = ModelList
3. 恢复检查: GET /v1/models → 200 OK
4. 标记恢复: 使用相同的检查方式
```

### 场景2：Chat 检查失败

```
1. 常规健康检查: POST /v1/chat/completions → 429 Rate Limited
2. 记录失败: failure_check_method = Chat
3. 恢复检查: POST /v1/chat/completions → 200 OK
4. 标记恢复: 使用相同的检查方式
```

### 场景3：网络错误

```
1. 常规健康检查: 网络连接失败
2. 记录失败: failure_check_method = Network
3. 恢复检查: GET /v1/models → 200 OK (默认使用model list)
4. 标记恢复: 网络恢复后使用轻量级检查
```

## 配置兼容性

此改进完全向后兼容，不需要修改任何配置文件：

- ✅ 现有配置文件无需更改
- ✅ 现有API接口保持不变
- ✅ 现有日志格式得到增强
- ✅ 现有监控指标继续有效

## 日志改进

新的日志格式提供更详细的信息：

```
# 失败记录
DEBUG Recording failure for backend: provider:model with method: ModelList

# 恢复检查
INFO Attempting model list recovery check for per-token backend provider:model (originally failed via model list)

# 恢复成功
INFO Model list recovery check passed for provider:model (150ms)
DEBUG Successfully restored backend provider:model to healthy state via model list check
```

## 测试验证

- ✅ 所有现有测试继续通过 (103个测试)
- ✅ 编译无错误无警告
- ✅ 向后兼容性验证通过
- ✅ 新功能逻辑验证通过

## 总结

这次改进通过引入检查方式记录机制，实现了健康检查和恢复检查的完全一致性。主要优势：

1. **逻辑一致性**：同一个后端始终使用相同的检查方式
2. **成本效率**：避免不必要的 chat 请求
3. **调试友好**：清晰的检查轨迹和日志
4. **向后兼容**：不影响现有功能和配置

这个改进让健康检查系统更加智能、高效和可靠！
