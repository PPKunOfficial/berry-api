# ⚖️ 负载均衡策略

Berry API 提供8种负载均衡策略，适应不同的业务场景：

### 策略选择指南

| 策略 | 适用场景 | 优势 | 劣势 |
|------|----------|------|------|
| `weighted_random` | 成本控制、按性能分配 | 灵活的权重分配 | 可能不够均匀 |
| `round_robin` | 简单均衡、相同性能后端 | 完全均匀分配 | 不考虑后端性能差异 |
| `least_latency` | 性能优化、延迟敏感 | 自动选择最快后端 | 需要延迟统计 |
| `failover` | 高可用、主备场景 | 明确的优先级 | 主后端压力大 |
| `random` | 简单场景、测试 | 实现简单 | 无优化策略 |
| `weighted_failover` | 智能负载均衡 | 结合权重和故障转移 | 配置相对复杂 |

### 1. 加权随机 (weighted_random)

根据权重随机选择后端，适合按成本或性能分配流量：

```toml
[models.cost_optimized]
name = "cost-optimized"
strategy = "weighted_random"
enabled = true

[[models.cost_optimized.backends]]
provider = "cheap-provider"
model = "gpt-3.5-turbo"
weight = 0.7  # 70% 流量给便宜的服务
priority = 1
enabled = true

[[models.cost_optimized.backends]]
provider = "premium-provider"
model = "gpt-3.5-turbo"
weight = 0.3  # 30% 流量给高质量服务
priority = 2
enabled = true
```

### 2. 轮询 (round_robin)

依次轮询所有可用后端，适合性能相近的后端：

```toml
[models.balanced]
name = "balanced"
strategy = "round_robin"
enabled = true

[[models.balanced.backends]]
provider = "provider-a"
model = "gpt-4"
weight = 1.0  # 轮询中权重无效
priority = 1
enabled = true

[[models.balanced.backends]]
provider = "provider-b"
model = "gpt-4"
weight = 1.0
priority = 2
enabled = true
```

### 3. 最低延迟 (least_latency)

自动选择响应时间最短的后端：

```toml
[models.fast_response]
name = "fast-response"
strategy = "least_latency"
enabled = true

[[models.fast_response.backends]]
provider = "fast-provider"
model = "gpt-3.5-turbo"
weight = 1.0
priority = 1
enabled = true

[[models.fast_response.backends]]
provider = "slow-provider"
model = "gpt-3.5-turbo"
weight = 1.0
priority = 2
enabled = true
```

### 4. 故障转移 (failover)

按优先级顺序选择，主要用于主备场景：

```toml
[models.high_availability]
name = "high-availability"
strategy = "failover"
enabled = true

[[models.high_availability.backends]]
provider = "primary-provider"
model = "gpt-4"
weight = 1.0
priority = 1  # 最高优先级，优先使用
enabled = true

[[models.high_availability.backends]]
provider = "backup-provider"
model = "gpt-4"
weight = 1.0
priority = 2  # 备用，主服务故障时使用
enabled = true

[[models.high_availability.backends]]
provider = "emergency-provider"
model = "gpt-4"
weight = 1.0
priority = 3  # 应急，前两个都故障时使用
enabled = true
```

### 5. 权重故障转移 (weighted_failover) 🆕

结合权重选择和故障转移的智能策略：

**工作原理**：

1.  **正常情况**: 从所有健康的后端中按权重随机选择
2.  **故障情况**: 自动屏蔽不健康的后端，只在健康的后端中选择
3.  **全部故障**: 如果所有后端都不健康，仍按权重选择（而非优先级）
4.  **自动恢复**: 后端恢复健康后自动重新加入负载均衡

```toml
[models.smart_model]
name = "smart-model"
strategy = "weighted_failover"
enabled = true

[[models.smart_model.backends]]
provider = "openai-main"
model = "gpt-4"
weight = 0.6    # 60%权重 - 主要服务
priority = 1    # 最高优先级
enabled = true

[[models.smart_model.backends]]
provider = "openai-backup"
model = "gpt-4"
weight = 0.3    # 30%权重 - 备用服务
priority = 2    # 中等优先级
enabled = true

[[models.smart_model.backends]]
provider = "azure"
model = "gpt-4"
weight = 0.1    # 10%权重 - 应急服务
priority = 3    # 最低优先级
enabled = true
```

### 6. 随机 (random)

完全随机选择，适合简单场景：

```toml
[models.simple_random]
name = "simple-random"
strategy = "random"
enabled = true

[[models.simple_random.backends]]
provider = "provider-a"
model = "gpt-3.5-turbo"
weight = 1.0  # 随机策略中权重无效
priority = 1
enabled = true
```

### 🧠 SmartAI策略详解

SmartAI是Berry API的核心创新，专为小流量、成本敏感的场景设计：

**核心特性：**

-   **成本感知选择**：优先选择便宜的后端，premium后端作为备选
-   **小流量优化**：80%选择最佳后端，20%探索其他选项
-   **智能健康检查**：基于用户请求进行被动验证
-   **信心度机制**：动态调整后端选择权重

**工作原理：**

```
1. 初始化：所有后端获得初始信心度(0.8)
2. 请求处理：根据信心度和权重选择后端
3. 结果反馈：成功提升信心度，失败降低信心度
4. 动态调整：信心度影响下次选择概率
5. 探索机制：20%流量用于测试其他后端
```
