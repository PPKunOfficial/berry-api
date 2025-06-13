# 后端指定功能 API 文档

## 概述

Berry API 现在支持在请求体中添加 `backend` 参数来直接指定要使用的后端提供商，这对于调试和健康检查非常有用。

## 功能特性

- 🎯 **直接指定后端**: 通过 `backend` 参数绕过负载均衡逻辑
- 🔍 **调试友好**: 方便测试特定后端的可用性
- 🏥 **健康检查**: 可以单独检查每个后端的状态
- 🔒 **安全性**: 只能选择配置文件中已定义的后端
- 🧹 **自动清理**: `backend` 参数不会传递给上游API

## 使用方法

### 基本语法

在标准的OpenAI API请求中添加 `backend` 字段：

```json
{
  "model": "gpt-4o",
  "messages": [{"role": "user", "content": "Hello"}],
  "backend": "provider_name",
  "stream": false
}
```

### 参数说明

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `backend` | string | 否 | 指定要使用的后端提供商名称 |

**注意**: `backend` 参数必须与配置文件中的 `provider` 名称完全匹配。

## 示例用法

### 1. 测试特定后端

```bash
# 测试 OpenAI 后端
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-token" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Hello"}],
    "backend": "openai_official",
    "max_tokens": 50
  }'
```

```bash
# 测试 Claude 后端
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-token" \
  -d '{
    "model": "claude-sonnet-4",
    "messages": [{"role": "user", "content": "Hello"}],
    "backend": "anthropic_claude",
    "max_tokens": 50
  }'
```

### 2. 流式请求测试

```bash
# 测试流式响应
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-token" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Count to 10"}],
    "backend": "openai_official",
    "stream": true,
    "max_tokens": 100
  }' \
  --no-buffer
```

### 3. 健康检查脚本

```bash
#!/bin/bash
# 检查所有后端的健康状态

BACKENDS=("openai_official" "anthropic_claude" "google_gemini")
MODEL="gpt-4o"

for backend in "${BACKENDS[@]}"; do
    echo "Testing backend: $backend"
    
    response=$(curl -s -w "HTTPSTATUS:%{http_code}" \
      -X POST http://localhost:3000/v1/chat/completions \
      -H "Content-Type: application/json" \
      -H "Authorization: Bearer your-token" \
      -d "{
        \"model\": \"$MODEL\",
        \"messages\": [{\"role\": \"user\", \"content\": \"ping\"}],
        \"backend\": \"$backend\",
        \"max_tokens\": 5
      }")
    
    http_status=$(echo "$response" | grep -o "HTTPSTATUS:[0-9]*" | cut -d: -f2)
    body=$(echo "$response" | sed 's/HTTPSTATUS:[0-9]*$//')
    
    if [ "$http_status" = "200" ]; then
        echo "✅ $backend: OK"
    else
        echo "❌ $backend: HTTP $http_status"
        echo "   Error: $body"
    fi
    echo
done
```

## 配置示例

确保您的配置文件中定义了相应的后端：

```toml
# config.toml

[providers.openai_official]
name = "OpenAI Official"
base_url = "https://api.openai.com/v1"
api_key = "sk-..."
models = ["gpt-4o", "gpt-4o-mini"]
enabled = true

[providers.anthropic_claude]
name = "Anthropic Claude"
base_url = "https://api.anthropic.com/v1"
api_key = "sk-ant-..."
models = ["claude-sonnet-4"]
enabled = true

[[models.gpt_4o.backends]]
provider = "openai_official"
model = "gpt-4o"
weight = 1.0
enabled = true

[[models.claude_sonnet_4.backends]]
provider = "anthropic_claude"
model = "claude-3-5-sonnet-20241022"
weight = 1.0
enabled = true
```

## 错误处理

### 1. 后端不存在

```json
{
  "error": {
    "message": "Specified backend 'invalid_backend' is not available for model 'gpt-4o': Backend 'invalid_backend' not found or disabled for model 'gpt-4o'"
  }
}
```

**HTTP状态码**: 500 Internal Server Error

### 2. 后端已禁用

```json
{
  "error": {
    "message": "Specified backend 'disabled_backend' is not available for model 'gpt-4o': Backend 'disabled_backend' not found or disabled for model 'gpt-4o'"
  }
}
```

**HTTP状态码**: 500 Internal Server Error

### 3. 模型不存在

```json
{
  "error": {
    "message": "Specified backend 'openai_official' is not available for model 'invalid_model': Model 'invalid_model' not found"
  }
}
```

**HTTP状态码**: 500 Internal Server Error

## 监控和日志

### 日志示例

当使用 `backend` 参数时，系统会记录相应的日志：

```
INFO berry_api_api::relay::handler::loadbalanced: Using specified backend 'openai_official' for model 'gpt-4o'
INFO berry_api_api::relay::handler::loadbalanced: Selected specific backend for model 'gpt-4o': provider='openai_official', model='gpt-4o', selection_time=2ms
```

### 调试建议

1. **检查配置**: 确保 `backend` 参数值与配置文件中的 `provider` 名称匹配
2. **查看日志**: 使用 `RUST_LOG=debug` 获取详细的选择过程日志
3. **测试连通性**: 先用简单的请求测试后端连通性
4. **验证权限**: 确保API密钥和权限配置正确

## 最佳实践

### 1. 健康检查自动化

```python
import requests
import json

def check_backend_health(backend_name, model="gpt-4o"):
    """检查指定后端的健康状态"""
    payload = {
        "model": model,
        "messages": [{"role": "user", "content": "ping"}],
        "backend": backend_name,
        "max_tokens": 1
    }
    
    try:
        response = requests.post(
            "http://localhost:3000/v1/chat/completions",
            headers={
                "Content-Type": "application/json",
                "Authorization": "Bearer your-token"
            },
            json=payload,
            timeout=30
        )
        
        if response.status_code == 200:
            return {"status": "healthy", "backend": backend_name}
        else:
            return {
                "status": "unhealthy", 
                "backend": backend_name,
                "error": response.text
            }
    except Exception as e:
        return {
            "status": "error",
            "backend": backend_name, 
            "error": str(e)
        }

# 使用示例
backends = ["openai_official", "anthropic_claude"]
for backend in backends:
    result = check_backend_health(backend)
    print(f"{backend}: {result['status']}")
```

### 2. 性能对比测试

```bash
# 比较不同后端的响应时间
for backend in openai_official anthropic_claude; do
    echo "Testing $backend..."
    time curl -s -X POST http://localhost:3000/v1/chat/completions \
      -H "Content-Type: application/json" \
      -H "Authorization: Bearer your-token" \
      -d "{
        \"model\": \"gpt-4o\",
        \"messages\": [{\"role\": \"user\", \"content\": \"Hello\"}],
        \"backend\": \"$backend\",
        \"max_tokens\": 10
      }" > /dev/null
done
```

### 3. 故障排除

当某个后端出现问题时，可以使用此功能进行针对性测试：

```bash
# 测试问题后端
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

## 安全注意事项

1. **访问控制**: 此功能不绕过用户权限检查
2. **配置限制**: 只能选择配置文件中已定义的后端
3. **日志记录**: 所有指定后端的请求都会被记录
4. **不影响计费**: 仍然按照正常的计费模式计算

这个功能为调试、监控和测试提供了强大的工具，同时保持了系统的安全性和稳定性。
