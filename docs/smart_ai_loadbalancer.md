# SmartAI 负载均衡器

## 概述

SmartAI 是一个专为个人项目设计的智能负载均衡策略，特别适合小流量场景。它基于客户流量进行被动健康检查，不产生额外的AI调用费用，同时具备成本感知能力，避免过度偏向昂贵但稳定的正价后端。

## 核心特性

### 1. 零成本健康检查
- **完全基于用户流量**：不主动发送付费的AI请求
- **免费连通性检查**：使用HTTP HEAD和models API等免费接口
- **真实用户体验**：基于实际用户请求评估后端健康状态

### 2. 小流量优化
- **高价值利用**：最大化利用每个用户请求的价值
- **单次失败权重**：在样本少的情况下，单次失败影响更大
- **智能探索**：80%选择最佳后端，20%探索其他后端

### 3. 成本感知
- **Premium标记识别**：利用现有的 `tags = ["premium"]` 标记
- **稳定性加成限制**：只有非premium后端才能获得稳定性加成
- **公平竞争**：premium后端凭借原始权重竞争，不获得额外优先度

### 4. 智能信心度系统
- **动态评分**：基于成功率和错误类型的信心度评分（0.0-1.0）
- **时间衰减**：长期无流量时适度降低信心度
- **渐进恢复**：失败后端通过成功请求逐步恢复信心度

## 配置说明

### 基础配置

```toml
[models.your_model]
name = "your-model"
strategy = "smart_ai"  # 使用 SmartAI 策略
enabled = true

# 便宜后端（可获得稳定性加成）
[[models.your_model.backends]]
provider = "cheap_provider"
model = "gpt-3.5-turbo"
weight = 1.0
tags = []  # 非premium

# 正价后端（不获得稳定性加成）
[[models.your_model.backends]]
provider = "premium_provider"
model = "gpt-4"
weight = 1.0
tags = ["premium"]  # premium标记
```

### SmartAI 参数配置

```toml
[settings.smart_ai]
# 初始信心度
initial_confidence = 0.8
# 最小信心度（保留恢复机会）
min_confidence = 0.05
# 启用时间衰减
enable_time_decay = true
# 轻量级检查间隔（秒）
lightweight_check_interval_seconds = 600
# 探索流量比例
exploration_ratio = 0.2
# 非premium后端稳定性加成
non_premium_stability_bonus = 1.1

# 信心度调整参数
[settings.smart_ai.confidence_adjustments]
success_boost = 0.1
network_error_penalty = 0.3
auth_error_penalty = 0.8
rate_limit_penalty = 0.1
server_error_penalty = 0.2
model_error_penalty = 0.3
timeout_penalty = 0.2
```

## 工作原理

### 1. 信心度计算

**初始状态**：新后端信心度为 0.8

**成功请求**：信心度 += 0.1（最大1.0）

**失败请求**：根据错误类型扣减信心度
- 网络错误：-0.3
- 认证错误：-0.8（几乎致命）
- 限流错误：-0.1（临时问题）
- 服务器错误：-0.2
- 模型错误：-0.3
- 超时错误：-0.2

### 2. 权重计算

```
有效权重 = 基础权重 × 信心度权重 × 稳定性加成

信心度权重：
- 信心度 >= 0.8：按比例
- 信心度 0.6-0.8：× 0.8
- 信心度 0.3-0.6：× 0.5
- 信心度 < 0.3：固定 0.05

稳定性加成：
- 非premium后端且信心度 > 0.9：1.1
- 其他情况：1.0
```

### 3. 选择策略

1. **计算有效权重**：为每个后端计算有效权重
2. **过滤可用后端**：排除权重过低的后端
3. **智能选择**：
   - 80%概率选择权重最高的后端
   - 20%概率进行加权随机选择（探索其他后端）

### 4. 时间衰减

长期无流量时的信心度衰减：
- 1小时内：无衰减
- 2-6小时：× 0.95
- 7-24小时：× 0.9
- 1-3天：× 0.8
- 3天以上：× 0.7（最低保持0.5）

## 使用建议

### 1. 后端配置
- **便宜后端**：不添加premium标记，可获得稳定性加成
- **正价后端**：添加 `tags = ["premium"]`，避免过度使用

### 2. 权重设置
- 所有后端可以使用相同的基础权重
- SmartAI会根据实际表现动态调整

### 3. 监控指标
- 关注各后端的信心度变化
- 监控流量分配比例
- 观察成本效率

### 4. 适用场景
- **个人项目**：流量较小，成本敏感
- **开发测试**：需要在多个后端间平衡成本和稳定性
- **成本优化**：希望优先使用便宜后端，正价后端作为备选

## 监控和调试

SmartAI提供详细的日志信息：

```
SmartAI backend provider:model: confidence=0.850, effective_weight=0.935 (original=1.000)
SmartAI selected backend provider:model for model 'your-model'
SmartAI success for provider:model: confidence=0.950, consecutive_successes=3
```

通过这些日志可以观察：
- 各后端的信心度变化
- 权重调整过程
- 选择决策过程

## 与其他策略的对比

| 策略 | 成本感知 | 小流量优化 | 零额外费用 | 智能恢复 |
|------|----------|------------|------------|----------|
| SmartAI | ✅ | ✅ | ✅ | ✅ |
| WeightedRandom | ❌ | ❌ | ✅ | ❌ |
| WeightedFailover | ❌ | ❌ | ✅ | ❌ |
| SmartWeightedFailover | ❌ | ❌ | ✅ | ✅ |

SmartAI是唯一同时具备成本感知和小流量优化的策略，特别适合个人项目使用。
