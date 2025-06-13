# SmartAI 权重查看 API

## 概述

SmartAI API 提供了查看当前模型权重、信心度和健康状态的端点，帮助您监控和调试SmartAI负载均衡器的运行状态。

## 模型名称说明

API支持两种方式指定模型：

1. **配置键名**：配置文件中的模型键（如 `gpt_4o`、`claude_sonnet_4_20250514`）
2. **显示名称**：模型的显示名称（如 `gpt-4o`、`claude-sonnet-4`）

您可以通过 `GET /smart-ai/weights` 端点的 `available_models` 字段查看所有可用的模型名称。

## API 端点

### 1. 获取所有SmartAI模型的权重信息

**端点**: `GET /smart-ai/weights`

**查询参数**:
- `detailed` (boolean, 可选): 是否包含详细的健康状态信息，默认 `false`
- `enabled_only` (boolean, 可选): 是否只显示启用的后端，默认 `true`

**示例请求**:
```bash
# 基本信息
curl "http://localhost:3000/smart-ai/weights"

# 详细信息，包含所有后端
curl "http://localhost:3000/smart-ai/weights?detailed=true&enabled_only=false"
```

**响应示例**:
```json
{
  "models": [
    {
      "name": "claude-sonnet-4",
      "strategy": "SmartAi",
      "enabled": true,
      "backends": [
        {
          "provider": "polo_claude",
          "model": "claude-sonnet-4-20250514",
          "original_weight": 1.0,
          "effective_weight": 0.99,
          "confidence": 0.9,
          "is_premium": false,
          "enabled": true,
          "tags": [],
          "billing_mode": "PerToken",
          "health_details": {
            "total_requests": 45,
            "consecutive_successes": 12,
            "consecutive_failures": 0,
            "last_request_time": "30 seconds ago",
            "last_success_time": "30 seconds ago",
            "error_counts": {
              "NetworkError": 2,
              "TimeoutError": 1
            },
            "connectivity_ok": true,
            "last_connectivity_check": "120 seconds ago"
          }
        },
        {
          "provider": "gala_claude_rev",
          "model": "claude-sonnet-4-20250514",
          "original_weight": 0.7,
          "effective_weight": 0.665,
          "confidence": 0.95,
          "is_premium": true,
          "enabled": true,
          "tags": ["premium"],
          "billing_mode": "PerToken"
        }
      ],
      "stats": {
        "total_backends": 2,
        "enabled_backends": 2,
        "healthy_backends": 2,
        "premium_backends": 1,
        "average_confidence": 0.925,
        "weight_distribution": {
          "polo_claude": 0.99,
          "gala_claude_rev": 0.665
        }
      }
    }
  ],
  "total_smart_ai_models": 1,
  "available_models": [
    {
      "key": "claude_sonnet_4_20250514",
      "name": "claude-sonnet-4",
      "enabled": true
    },
    {
      "key": "gpt_4o",
      "name": "gpt-4o",
      "enabled": true
    }
  ],
  "timestamp": "2025-01-27T10:30:00Z",
  "settings": {
    "detailed": true,
    "enabled_only": true
  }
}
```

### 2. 获取特定模型的权重信息

**端点**: `GET /smart-ai/models/{model_name}/weights`

**路径参数**:
- `model_name` (string): 模型名称，可以使用配置文件中的键名（如 `gpt_4o`）或显示名称（如 `gpt-4o`）

**查询参数**:
- `detailed` (boolean, 可选): 是否包含详细信息，默认 `false`
- `enabled_only` (boolean, 可选): 是否只显示启用的后端，默认 `true`

**示例请求**:
```bash
# 使用配置键名查看特定模型的权重
curl "http://localhost:3000/smart-ai/models/gpt_4o/weights?detailed=true"

# 使用显示名称查看特定模型的权重
curl "http://localhost:3000/smart-ai/models/gpt-4o/weights?detailed=true"
```

**响应示例**:
```json
{
  "model": {
    "name": "claude-sonnet-4",
    "strategy": "SmartAi",
    "enabled": true,
    "backends": [
      {
        "provider": "polo_claude",
        "model": "claude-sonnet-4-20250514",
        "original_weight": 1.0,
        "effective_weight": 0.99,
        "confidence": 0.9,
        "is_premium": false,
        "enabled": true,
        "tags": [],
        "billing_mode": "PerToken"
      }
    ],
    "stats": {
      "total_backends": 1,
      "enabled_backends": 1,
      "healthy_backends": 1,
      "premium_backends": 0,
      "average_confidence": 0.9,
      "weight_distribution": {
        "polo_claude": 0.99
      }
    }
  },
  "timestamp": "2025-01-27T10:30:00Z"
}
```

## 响应字段说明

### 后端信息 (BackendWeightInfo)

| 字段 | 类型 | 说明 |
|------|------|------|
| `provider` | string | 提供商名称 |
| `model` | string | 模型名称 |
| `original_weight` | number | 配置文件中的原始权重 |
| `effective_weight` | number | SmartAI计算的有效权重 |
| `confidence` | number | 信心度 (0.0-1.0) |
| `is_premium` | boolean | 是否为premium后端 |
| `enabled` | boolean | 是否启用 |
| `tags` | array | 标签列表 |
| `billing_mode` | string | 计费模式 |
| `health_details` | object | 健康状态详情（仅在detailed=true时） |

### 健康状态详情 (BackendHealthDetails)

| 字段 | 类型 | 说明 |
|------|------|------|
| `total_requests` | number | 总请求数 |
| `consecutive_successes` | number | 连续成功次数 |
| `consecutive_failures` | number | 连续失败次数 |
| `last_request_time` | string | 最后请求时间 |
| `last_success_time` | string | 最后成功时间 |
| `last_failure_time` | string | 最后失败时间 |
| `error_counts` | object | 错误类型统计 |
| `connectivity_ok` | boolean | 连通性状态 |
| `last_connectivity_check` | string | 最后连通性检查时间 |

### 模型统计 (ModelStats)

| 字段 | 类型 | 说明 |
|------|------|------|
| `total_backends` | number | 总后端数 |
| `enabled_backends` | number | 启用的后端数 |
| `healthy_backends` | number | 健康的后端数 |
| `premium_backends` | number | Premium后端数 |
| `average_confidence` | number | 平均信心度 |
| `weight_distribution` | object | 各提供商的权重分布 |

## 权重计算说明

SmartAI的有效权重计算公式：
```
有效权重 = 原始权重 × 信心度权重 × 稳定性加成

信心度权重：
- 信心度 >= 0.8：按比例
- 信心度 0.6-0.8：× 0.8
- 信心度 0.3-0.6：× 0.5
- 信心度 < 0.3：固定 0.05

稳定性加成：
- 非premium后端且信心度 > 0.9：1.1
- 其他情况：1.0
```

## 使用场景

### 1. 监控成本分布
查看流量是否按预期分配到便宜的后端：
```bash
curl "http://localhost:3000/smart-ai/weights" | jq '.models[].stats.weight_distribution'
```

### 2. 调试健康问题
查看特定模型的详细健康状态：
```bash
curl "http://localhost:3000/smart-ai/models/your_model/weights?detailed=true"
```

### 3. 验证配置效果
检查premium后端是否正确标记，权重是否符合预期：
```bash
curl "http://localhost:3000/smart-ai/weights" | jq '.models[].backends[] | select(.is_premium == true)'
```

### 4. 性能分析
观察信心度变化趋势，识别不稳定的后端：
```bash
curl "http://localhost:3000/smart-ai/weights" | jq '.models[].backends[] | {provider, confidence, consecutive_failures}'
```

## 错误响应

### 模型不存在
```json
{
  "error": "Model not found",
  "model": "non_existent_model"
}
```

### 模型不使用SmartAI策略
```json
{
  "error": "Model does not use SmartAI strategy",
  "model": "your_model",
  "current_strategy": "WeightedRandom"
}
```

## 集成建议

1. **定期监控**: 设置定时任务定期调用API，监控权重分布
2. **告警集成**: 当premium后端权重过高时触发告警
3. **可视化**: 将API数据集成到监控面板中
4. **自动化**: 基于API数据自动调整配置或触发操作

这些API端点为SmartAI负载均衡器提供了完整的可观测性，帮助您更好地理解和优化系统的运行状态。
