# 混合Provider健康检查修复

## 问题描述

在之前的实现中，当一个Provider同时包含 `per_token` 和 `per_request` 计费模式的模型时，健康检查逻辑存在问题：

1. **触发条件**：只要Provider中有任何一个 `per_token` 模型，就会执行主动健康检查
2. **影响范围**：但是 `check_test_provider` 和 `check_real_provider` 会对Provider的**所有模型**进行操作
3. **意外后果**：`per_request` 模型也会被这些函数意外地进行健康检查

## 问题场景

假设有以下配置：

```toml
[providers.mixed_provider]
name = "Mixed Provider"
base_url = "https://api.openai.com"
api_key = "sk-xxx"
models = ["gpt-3.5-turbo", "dall-e-3"]

# 聊天模型 - per_token计费
[[models.chat_model.backends]]
provider = "mixed_provider"
model = "gpt-3.5-turbo"
billing_mode = "per_token"

# 图像模型 - per_request计费  
[[models.image_model.backends]]
provider = "mixed_provider"
model = "dall-e-3"
billing_mode = "per_request"
```

在修复前：
- 因为 `gpt-3.5-turbo` 是 `per_token`，系统会执行主动健康检查
- `check_real_provider` 会对Provider的所有模型（包括 `dall-e-3`）进行健康检查
- 这导致 `per_request` 模型也被意外检查

## 修复方案

### 1. 修改函数签名

为 `check_test_provider` 和 `check_real_provider` 添加 `per_token_models` 参数：

```rust
async fn check_test_provider(
    provider_id: &str,
    provider: &Provider,
    client: &Client,
    metrics: &MetricsCollector,
    start_time: Instant,
    is_initial_check: bool,
    per_token_models: &[String],  // 新增参数
)

async fn check_real_provider(
    provider_id: &str,
    provider: &Provider,
    metrics: &MetricsCollector,
    start_time: Instant,
    is_initial_check: bool,
    per_token_models: &[String],  // 新增参数
)
```

### 2. 修改调用逻辑

在调用这些函数前，先收集 `per_token` 模型列表：

```rust
// 获取per-token模型列表
let mut per_token_models = Vec::new();
for (_, model_mapping) in &config.models {
    for backend in &model_mapping.backends {
        if backend.provider == provider_id && provider.models.contains(&backend.model) {
            if backend.billing_mode == BillingMode::PerToken {
                per_token_models.push(backend.model.clone());
            }
        }
    }
}

// 传递per_token_models而不是所有模型
Self::check_real_provider(provider_id, provider, metrics, start_time, is_initial_check, &per_token_models).await;
```

### 3. 修改处理逻辑

在健康检查函数内部，只处理传入的 `per_token_models`：

```rust
// 修复前：处理所有模型
for model in &provider.models {
    // ...
}

// 修复后：只处理per_token模型
for model in per_token_models {
    // ...
}
```

## 修复效果

修复后的行为：

1. **精确检查**：只有 `per_token` 模型会被主动健康检查
2. **独立处理**：`per_request` 模型保持独立的被动验证机制
3. **避免干扰**：混合Provider中的不同计费模式模型不会相互影响

## 测试验证

添加了专门的测试用例 `test_mixed_provider_health_check` 来验证修复效果：

```rust
#[tokio::test]
async fn test_mixed_provider_health_check() {
    // 创建包含per_token和per_request模型的混合Provider
    // 验证健康检查行为正确
}
```

## 总结

这个修复确保了：
- 混合Provider中的 `per_token` 和 `per_request` 模型按照各自的计费模式进行正确的健康检查
- 避免了 `per_request` 模型被意外的主动健康检查
- 保持了系统的成本控制和性能优化目标
