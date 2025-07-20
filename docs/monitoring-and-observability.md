# 📊 监控与可观测性

Berry API 提供基于面板的基础信心度观测，包括指标收集、日志记录和健康监控。

### 🎯 核心指标

**服务健康指标**

- **服务运行状态** - 服务是否正常运行
- **总请求数** - 累计处理的请求总数
- **成功请求数** - 成功处理的请求数
- **成功率** - 成功请求占总请求的百分比
- **时间戳** - 指标更新时间

**提供商健康指标**

- **总提供商数** - 配置的提供商总数
- **健康提供商数** - 当前健康的提供商数
- **健康比例** - 健康提供商占总数的比例

**模型健康指标**

- **总模型数** - 配置的模型总数
- **健康模型数** - 当前健康的模型数
- **健康比例** - 健康模型占总数的比例
- **模型详情** - 每个模型的详细健康状态

### 📈 基础信心度观测

**指标端点**

```bash
# 获取服务指标
curl http://localhost:3000/metrics

# 获取详细监控信息
curl http://localhost:3000/monitoring/info

# 获取性能指标
curl http://localhost:3000/monitoring/performance
```

**指标响应示例**

```json
{
  "service": {
    "running": true,
    "total_requests": 1250,
    "successful_requests": 1200,
    "success_rate": 0.96
  },
  "providers": {
    "total": 3,
    "healthy": 3,
    "health_ratio": 1.0
  },
  "models": {
    "total": 5,
    "healthy": 5,
    "health_ratio": 1.0,
    "details": {
      "gpt-4": {
        "healthy_backends": 2,
        "total_backends": 2,
        "health_ratio": 1.0,
        "is_healthy": true,
        "average_latency_ms": 850
      }
    }
  },
  "timestamp": "2024-01-15T10:30:00Z"
}
```

### 📝 日志管理

**日志级别配置**

```bash
# 环境变量配置
export RUST_LOG=info                    # 基础日志
export RUST_LOG=debug                   # 调试日志
export RUST_LOG=berry_api=debug         # 特定模块日志
export RUST_LOG=trace                   # 详细跟踪日志
```

**结构化日志示例**

```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "level": "INFO",
  "target": "berry_api::loadbalance",
  "message": "Backend selected",
  "fields": {
    "provider": "openai",
    "model": "gpt-4",
    "strategy": "weighted_failover",
    "latency_ms": 850
  }
}
```

**日志分析命令**

```bash
# 查看错误日志
grep "ERROR" logs/berry-api.log | jq .

# 监控健康检查
grep "health_check" logs/berry-api.log | tail -20

# 分析性能指标
grep "latency" logs/berry-api.log | jq '.fields.latency_ms' | sort -n

# 统计请求分布
grep "Backend selected" logs/berry-api.log | jq -r '.fields.provider' | sort | uniq -c
```

### 🔍 健康检查监控

**健康检查端点**

```bash
# 基础健康检查
curl http://localhost:3000/health

# 详细健康状态
curl http://localhost:3000/metrics | jq .

# 特定后端健康状态
curl http://localhost:3000/admin/backend-health
```

**健康状态响应示例**

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
  "load_balancer": {
    "total_selections": 5000,
    "strategy_distribution": {
      "weighted_failover": 3000,
      "smart_ai": 2000
    }
  }
}
```

### 🖥️ 管理接口

**系统状态监控**

```bash
# 获取系统统计信息
curl http://localhost:3000/admin/system-stats

# 获取模型权重信息
curl http://localhost:3000/admin/model-weights

# 获取速率限制使用情况
curl http://localhost:3000/admin/rate-limit-usage
```

**性能监控**

```bash
# 获取详细性能指标
curl http://localhost:3000/monitoring/performance

# 获取模型权重监控
curl http://localhost:3000/monitoring/model-weights
```
