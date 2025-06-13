# 后端指定功能使用示例

## 快速开始

### 1. 基本用法

```bash
# 正常请求（自动负载均衡）
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-token" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Hello"}]
  }'

# 指定后端请求
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-token" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Hello"}],
    "backend": "openai_official"
  }'
```

### 2. 健康检查

```bash
# 检查特定后端
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-token" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "ping"}],
    "backend": "openai_official",
    "max_tokens": 1
  }'
```

### 3. 流式测试

```bash
# 测试流式响应
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-token" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Count to 5"}],
    "backend": "openai_official",
    "stream": true,
    "max_tokens": 50
  }' \
  --no-buffer
```

## 脚本工具

### 1. Bash 健康检查脚本

```bash
#!/bin/bash
# health_check.sh

BACKENDS=("openai_official" "anthropic_claude" "google_gemini")
MODEL="gpt-4o"
BASE_URL="http://localhost:3000"
TOKEN="your-token"

echo "🏥 后端健康检查报告"
echo "==================="
echo "时间: $(date)"
echo "模型: $MODEL"
echo

for backend in "${BACKENDS[@]}"; do
    echo "检查后端: $backend"
    
    start_time=$(date +%s.%N)
    response=$(curl -s -w "HTTPSTATUS:%{http_code}" \
      -X POST "$BASE_URL/v1/chat/completions" \
      -H "Content-Type: application/json" \
      -H "Authorization: Bearer $TOKEN" \
      -d "{
        \"model\": \"$MODEL\",
        \"messages\": [{\"role\": \"user\", \"content\": \"ping\"}],
        \"backend\": \"$backend\",
        \"max_tokens\": 1
      }")
    end_time=$(date +%s.%N)
    
    http_status=$(echo "$response" | grep -o "HTTPSTATUS:[0-9]*" | cut -d: -f2)
    response_time=$(echo "$end_time - $start_time" | bc)
    
    if [ "$http_status" = "200" ]; then
        echo "✅ $backend: 健康 (${response_time}s)"
    else
        echo "❌ $backend: 不健康 (HTTP $http_status)"
    fi
    echo
done
```

### 2. Python 监控工具

```python
# 使用我们提供的 backend_health_checker.py

# 单次检查所有后端
python3 examples/backend_health_checker.py check

# 检查特定后端
python3 examples/backend_health_checker.py check --backends openai_official anthropic_claude

# 测试流式请求
python3 examples/backend_health_checker.py check --streaming

# 持续监控（每60秒检查一次）
python3 examples/backend_health_checker.py monitor --interval 60

# 性能基准测试（3轮）
python3 examples/backend_health_checker.py benchmark --rounds 3
```

## 故障排除场景

### 1. 调试特定后端问题

```bash
# 当某个后端出现问题时，直接测试该后端
curl -v -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-token" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "test"}],
    "backend": "problematic_backend",
    "max_tokens": 5
  }'
```

### 2. 比较不同后端的响应

```bash
# 测试多个后端的相同请求
for backend in openai_official anthropic_claude; do
    echo "Testing $backend:"
    curl -s -X POST http://localhost:3000/v1/chat/completions \
      -H "Content-Type: application/json" \
      -H "Authorization: Bearer your-token" \
      -d "{
        \"model\": \"gpt-4o\",
        \"messages\": [{\"role\": \"user\", \"content\": \"What is 2+2?\"}],
        \"backend\": \"$backend\",
        \"max_tokens\": 10
      }" | jq '.choices[0].message.content'
    echo
done
```

### 3. 性能对比测试

```bash
# 测试响应时间
for backend in openai_official anthropic_claude; do
    echo "Performance test for $backend:"
    time curl -s -X POST http://localhost:3000/v1/chat/completions \
      -H "Content-Type: application/json" \
      -H "Authorization: Bearer your-token" \
      -d "{
        \"model\": \"gpt-4o\",
        \"messages\": [{\"role\": \"user\", \"content\": \"Hello\"}],
        \"backend\": \"$backend\",
        \"max_tokens\": 5
      }" > /dev/null
    echo
done
```

## 监控集成

### 1. Prometheus 指标收集

```python
import requests
import time
from prometheus_client import Gauge, start_http_server

# 创建指标
backend_health_gauge = Gauge('backend_health', 'Backend health status', ['backend', 'model'])
backend_response_time_gauge = Gauge('backend_response_time_seconds', 'Backend response time', ['backend', 'model'])

def collect_metrics():
    backends = ["openai_official", "anthropic_claude"]
    model = "gpt-4o"
    
    for backend in backends:
        start_time = time.time()
        try:
            response = requests.post(
                "http://localhost:3000/v1/chat/completions",
                headers={
                    "Content-Type": "application/json",
                    "Authorization": "Bearer your-token"
                },
                json={
                    "model": model,
                    "messages": [{"role": "user", "content": "ping"}],
                    "backend": backend,
                    "max_tokens": 1
                },
                timeout=30
            )
            
            response_time = time.time() - start_time
            health_status = 1 if response.status_code == 200 else 0
            
            backend_health_gauge.labels(backend=backend, model=model).set(health_status)
            backend_response_time_gauge.labels(backend=backend, model=model).set(response_time)
            
        except Exception as e:
            backend_health_gauge.labels(backend=backend, model=model).set(0)
            backend_response_time_gauge.labels(backend=backend, model=model).set(0)

# 启动指标服务器
start_http_server(8000)

# 定期收集指标
while True:
    collect_metrics()
    time.sleep(60)
```

### 2. 告警脚本

```bash
#!/bin/bash
# alert_check.sh

WEBHOOK_URL="https://hooks.slack.com/your/webhook/url"
BACKENDS=("openai_official" "anthropic_claude")
MODEL="gpt-4o"

for backend in "${BACKENDS[@]}"; do
    response=$(curl -s -w "HTTPSTATUS:%{http_code}" \
      -X POST "http://localhost:3000/v1/chat/completions" \
      -H "Content-Type: application/json" \
      -H "Authorization: Bearer your-token" \
      -d "{
        \"model\": \"$MODEL\",
        \"messages\": [{\"role\": \"user\", \"content\": \"ping\"}],
        \"backend\": \"$backend\",
        \"max_tokens\": 1
      }")
    
    http_status=$(echo "$response" | grep -o "HTTPSTATUS:[0-9]*" | cut -d: -f2)
    
    if [ "$http_status" != "200" ]; then
        # 发送告警
        curl -X POST "$WEBHOOK_URL" \
          -H "Content-Type: application/json" \
          -d "{
            \"text\": \"🚨 Backend Alert: $backend is unhealthy (HTTP $http_status)\"
          }"
    fi
done
```

## 最佳实践

### 1. 生产环境使用

- ✅ 用于健康检查和监控
- ✅ 用于故障排除和调试
- ✅ 用于性能测试和对比
- ⚠️ 避免在正常业务请求中使用
- ⚠️ 不要依赖特定后端进行业务逻辑

### 2. 安全考虑

- 🔒 `backend` 参数不会传递给上游API
- 🔒 只能选择配置文件中已定义的后端
- 🔒 所有请求都会被记录在日志中
- 🔒 不绕过用户权限检查

### 3. 监控建议

- 📊 定期检查所有后端的健康状态
- 📊 监控响应时间和成功率
- 📊 设置告警阈值
- 📊 记录历史数据用于分析

### 4. 调试技巧

- 🔍 使用 `RUST_LOG=debug` 查看详细日志
- 🔍 比较不同后端的响应差异
- 🔍 测试流式和非流式请求
- 🔍 检查网络连接和API密钥配置

这个功能为运维和开发提供了强大的工具，帮助您更好地管理和监控多后端系统。
