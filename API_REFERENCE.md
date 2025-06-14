# Berry API 接口参考

## 🔐 认证

所有需要认证的API都使用Bearer Token认证：

```http
Authorization: Bearer your-token-here
```

## 📋 OpenAI兼容接口

### POST /v1/chat/completions

创建聊天完成请求，完全兼容OpenAI API。

**请求体**：
```json
{
  "model": "gpt-4",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Hello!"}
  ],
  "stream": false,
  "max_tokens": 1000,
  "temperature": 0.7,
  "top_p": 1.0,
  "frequency_penalty": 0,
  "presence_penalty": 0,
  "stop": null
}
```

**响应（非流式）**：
```json
{
  "id": "chatcmpl-123",
  "object": "chat.completion",
  "created": 1677652288,
  "model": "gpt-4",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Hello! How can I help you today?"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 9,
    "completion_tokens": 12,
    "total_tokens": 21
  }
}
```

**响应（流式）**：
```
data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"gpt-4","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"gpt-4","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]
```

### GET /v1/models

获取可用模型列表。

**响应**：
```json
{
  "object": "list",
  "data": [
    {
      "id": "gpt-4",
      "object": "model",
      "created": 1677610602,
      "owned_by": "berry-api"
    },
    {
      "id": "gpt-3.5-turbo",
      "object": "model",
      "created": 1677610602,
      "owned_by": "berry-api"
    }
  ]
}
```

## 🏥 健康检查接口

### GET /health

基础健康检查，返回服务状态。

**响应**：
```json
{
  "status": "healthy",
  "timestamp": "2024-01-15T10:30:00Z"
}
```

### GET /v1/health

OpenAI兼容的健康检查接口。

**响应**：
```json
{
  "status": "ok"
}
```

## 📊 指标接口

### GET /metrics

获取详细的服务指标和健康状态。

**响应**：
```json
{
  "status": "healthy",
  "timestamp": "2024-01-15T10:30:00Z",
  "providers": {
    "openai": {
      "healthy": true,
      "last_check": "2024-01-15T10:29:45Z",
      "total_requests": 1250,
      "successful_requests": 1200,
      "failed_requests": 50,
      "average_latency_ms": 850,
      "models": {
        "gpt-4": {
          "healthy": true,
          "requests": 800,
          "errors": 20
        }
      }
    }
  },
  "models": {
    "gpt-4": {
      "total_requests": 800,
      "successful_requests": 780,
      "failed_requests": 20,
      "strategy": "weighted_failover"
    }
  },
  "load_balancer": {
    "total_selections": 5000,
    "strategy_distribution": {
      "weighted_failover": 3000,
      "smart_ai": 2000
    }
  },
  "static_files": {
    "total_files": 15,
    "total_size_bytes": 2048576
  }
}
```

### GET /prometheus

获取Prometheus格式的指标数据。

**响应**：
```
# HELP http_requests_total Total number of HTTP requests
# TYPE http_requests_total counter
http_requests_total{method="POST",status="200",endpoint="/v1/chat/completions"} 1250

# HELP backend_health_status Backend health status (0=unhealthy, 1=healthy)
# TYPE backend_health_status gauge
backend_health_status{provider="openai",model="gpt-4"} 1

# HELP backend_latency_seconds Backend response latency
# TYPE backend_latency_seconds histogram
backend_latency_seconds_bucket{provider="openai",model="gpt-4",le="0.1"} 100
backend_latency_seconds_bucket{provider="openai",model="gpt-4",le="0.5"} 800
backend_latency_seconds_bucket{provider="openai",model="gpt-4",le="1.0"} 1200
backend_latency_seconds_bucket{provider="openai",model="gpt-4",le="+Inf"} 1250
```

## 🎛️ 管理接口

### GET /admin/model-weights

获取模型权重信息（需要管理员权限）。

**响应**：
```json
{
  "models": {
    "gpt-4": {
      "strategy": "weighted_failover",
      "backends": [
        {
          "provider": "openai",
          "model": "gpt-4",
          "weight": 0.7,
          "priority": 1,
          "healthy": true,
          "current_weight": 0.7
        },
        {
          "provider": "azure",
          "model": "gpt-4",
          "weight": 0.3,
          "priority": 2,
          "healthy": true,
          "current_weight": 0.3
        }
      ]
    }
  }
}
```

### GET /admin/backend-health

获取后端健康状态详情（需要管理员权限）。

**响应**：
```json
{
  "backends": {
    "openai:gpt-4": {
      "healthy": true,
      "last_check": "2024-01-15T10:29:45Z",
      "consecutive_failures": 0,
      "total_requests": 800,
      "successful_requests": 780,
      "failed_requests": 20,
      "average_latency_ms": 850,
      "last_error": null
    },
    "azure:gpt-4": {
      "healthy": false,
      "last_check": "2024-01-15T10:29:30Z",
      "consecutive_failures": 3,
      "total_requests": 200,
      "successful_requests": 180,
      "failed_requests": 20,
      "average_latency_ms": 1200,
      "last_error": "Connection timeout"
    }
  }
}
```

### GET /admin/system-stats

获取系统统计信息（需要管理员权限）。

**响应**：
```json
{
  "uptime_seconds": 86400,
  "total_requests": 5000,
  "requests_per_second": 10.5,
  "memory_usage_mb": 128,
  "active_connections": 25,
  "load_balancer_stats": {
    "total_selections": 5000,
    "average_selection_time_ms": 2.5,
    "cache_hit_rate": 0.95
  }
}
```

## 🧠 SmartAI接口

### GET /smart-ai/weights

获取SmartAI全局权重信息。

**响应**：
```json
{
  "smart_ai_enabled": true,
  "global_settings": {
    "initial_confidence": 0.8,
    "min_confidence": 0.05,
    "exploration_ratio": 0.2
  },
  "models": {
    "gpt-4": {
      "strategy": "smart_ai",
      "backends": [
        {
          "provider": "cheap_provider",
          "model": "gpt-3.5-turbo",
          "confidence": 0.75,
          "effective_weight": 0.85,
          "tags": []
        },
        {
          "provider": "premium_provider",
          "model": "gpt-4",
          "confidence": 0.90,
          "effective_weight": 0.45,
          "tags": ["premium"]
        }
      ]
    }
  }
}
```

### GET /smart-ai/models/{model}/weights

获取特定模型的SmartAI权重信息。

**响应**：
```json
{
  "model": "gpt-4",
  "strategy": "smart_ai",
  "last_updated": "2024-01-15T10:30:00Z",
  "backends": [
    {
      "provider": "cheap_provider",
      "model": "gpt-3.5-turbo",
      "base_weight": 1.0,
      "confidence": 0.75,
      "stability_bonus": 1.1,
      "effective_weight": 0.825,
      "selection_probability": 0.65,
      "recent_requests": 150,
      "recent_successes": 145,
      "tags": []
    },
    {
      "provider": "premium_provider",
      "model": "gpt-4",
      "base_weight": 0.5,
      "confidence": 0.90,
      "stability_bonus": 1.0,
      "effective_weight": 0.45,
      "selection_probability": 0.35,
      "recent_requests": 80,
      "recent_successes": 78,
      "tags": ["premium"]
    }
  ]
}
```

## ❌ 错误响应

所有错误响应都遵循统一格式：

```json
{
  "error": {
    "type": "invalid_token",
    "message": "The provided API key is invalid",
    "code": 401
  }
}
```

**常见错误类型**：
- `invalid_token` (401): 无效的认证Token
- `model_access_denied` (403): 模型访问被拒绝
- `rate_limit_exceeded` (429): 超过速率限制
- `service_unavailable` (503): 服务不可用
- `gateway_timeout` (504): 网关超时
- `internal_error` (500): 内部服务器错误

## 📝 请求示例

### cURL示例

```bash
# 聊天完成
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-token" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'

# 获取模型列表
curl -H "Authorization: Bearer your-token" \
     http://localhost:3000/v1/models

# 健康检查
curl http://localhost:3000/health
```

### Python示例

```python
import openai

client = openai.OpenAI(
    api_key="your-token",
    base_url="http://localhost:3000/v1"
)

response = client.chat.completions.create(
    model="gpt-4",
    messages=[{"role": "user", "content": "Hello!"}]
)
```

### JavaScript示例

```javascript
import OpenAI from 'openai';

const openai = new OpenAI({
  apiKey: 'your-token',
  baseURL: 'http://localhost:3000/v1',
});

const response = await openai.chat.completions.create({
  model: 'gpt-4',
  messages: [{ role: 'user', content: 'Hello!' }],
});
```
