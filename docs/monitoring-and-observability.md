# 📊 监控与可观测性

Berry API 提供完整的可观测性支持，包括指标收集、日志记录和健康监控。

### 🎯 核心指标

**HTTP 请求指标**

-   `http_requests_total` - 总请求数（按状态码、方法、路径分类）
-   `http_request_duration_seconds` - 请求延迟分布
-   `http_requests_in_flight` - 当前处理中的请求数

**后端健康指标**

-   `backend_health_status` - 后端健康状态（0=不健康，1=健康）
-   `backend_request_count_total` - 后端请求总数
-   `backend_error_count_total` - 后端错误总数
-   `backend_latency_seconds` - 后端响应延迟

**负载均衡指标**

-   `load_balance_selections_total` - 负载均衡选择次数
-   `smart_ai_confidence_score` - SmartAI信心度分数
-   `circuit_breaker_state` - 熔断器状态

### 📈 Prometheus 集成

**启用可观测性功能**

```bash
# 编译时启用observability特性
cargo build --release --features observability

# 或在Cargo.toml中配置
[features]
default = ["observability"]
observability = ["prometheus", "axum-prometheus"]
```

**Prometheus 配置**

```yaml
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'berry-api'
    static_configs:
      - targets: ['localhost:3000']
    metrics_path: '/prometheus'
    scrape_interval: 10s
```

**Grafana 仪表板**

创建 Grafana 仪表板监控关键指标：

```json
{
  "dashboard": {
    "title": "Berry API Dashboard",
    "panels": [
      {
        "title": "Request Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(http_requests_total[5m])",
            "legendFormat": "{{method}} {{status}}"
          }
        ]
      },
      {
        "title": "Backend Health",
        "type": "stat",
        "targets": [
          {
            "expr": "backend_health_status",
            "legendFormat": "{{provider}}:{{model}}"
          }
        ]
      }
    ]
  }
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

### 🚨 告警配置

**Prometheus 告警规则**

```yaml
# alerts.yml
groups:
  - name: berry-api
    rules:
      - alert: HighErrorRate
        expr: rate(http_requests_total{status=~"5.."}[5m]) > 0.1
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "High error rate detected"

      - alert: BackendDown
        expr: backend_health_status == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Backend {{ $labels.provider }}:{{ $labels.model }} is down"

      - alert: HighLatency
        expr: histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m])) > 2
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High latency detected"
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
