# 🔌 API使用指南

Berry API 完全兼容 OpenAI API 格式，可以无缝替换现有的 OpenAI 客户端。

### 🔐 认证与权限管理

**认证方式**

```bash
Authorization: Bearer your-token-here
```

**权限控制**

-   **管理员用户**：`allowed_models = []` 可访问所有模型
-   **普通用户**：`allowed_models = ["gpt-4"]` 只能访问指定模型
-   **用户标签**：支持基于标签的后端过滤

### 💬 聊天完成接口

**非流式请求**

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer berry-admin-token-12345" \
  -d '{
    "model": "gpt-4",
    "messages": [
      {"role": "system", "content": "You are a helpful assistant."},
      {"role": "user", "content": "Hello, world!"}
    ],
    "stream": false,
    "max_tokens": 1000,
    "temperature": 0.7,
    "top_p": 1.0,
    "frequency_penalty": 0,
    "presence_penalty": 0
  }'
```

**流式请求**

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer berry-admin-token-12345" \
  -d '{
    "model": "gpt-4",
    "messages": [
      {"role": "user", "content": "写一首关于春天的诗"}
    ],
    "stream": true,
    "max_tokens": 1000
  }'
```

**Python SDK 示例**

```python
import openai

# 配置客户端
client = openai.OpenAI(
    api_key="berry-admin-token-12345",
    base_url="http://localhost:3000/v1"
)

# 非流式请求
response = client.chat.completions.create(
    model="gpt-4",
    messages=[
        {"role": "system", "content": "You are a helpful assistant."},
        {"role": "user", "content": "Hello, world!"}
    ],
    stream=False
)
print(response.choices[0].message.content)

# 流式请求
stream = client.chat.completions.create(
    model="gpt-4",
    messages=[{"role": "user", "content": "Tell me a story"}],
    stream=True
)

for chunk in stream:
    if chunk.choices[0].delta.content is not None:
        print(chunk.choices[0].delta.content, end="")
```

**Node.js 示例**

```javascript
import OpenAI from 'openai';

const openai = new OpenAI({
  apiKey: 'berry-admin-token-12345',
  baseURL: 'http://localhost:3000/v1',
});

async function main() {
  const completion = await openai.chat.completions.create({
    messages: [{ role: 'user', content: 'Hello world' }],
    model: 'gpt-4',
  });

  console.log(completion.choices[0].message.content);
}

main();
```

### 📋 模型管理

**获取可用模型**

```bash
curl http://localhost:3000/v1/models \
  -H "Authorization: Bearer berry-admin-token-12345"
```

**响应示例**

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

### 🎯 路由选择器使用

Berry API 内置智能路由选择器，采用 **SmartAI 模式** 进行负载均衡：

-   **SmartAI 模式**：根据后端健康状态、响应时间、错误率和动态权重等因素，智能地选择最优后端。
-   **自动故障转移和恢复**：当后端出现故障时，自动将其从可用列表中移除，并在恢复后重新加入。
-   **用户标签过滤**：支持根据用户配置的标签对后端进行过滤，实现更精细的路由控制，例如用于环境隔离（开发/测试/生产）或 A/B 测试。

### 🏥 健康检查与监控

**基础健康检查**

```bash
curl http://localhost:3000/health
```

**详细健康状态**

```bash
curl http://localhost:3000/metrics
```

**Prometheus 指标**

```bash
curl http://localhost:3000/prometheus
```

### 🎛️ 管理接口

**获取模型权重**

```bash
curl http://localhost:3000/admin/model-weights \
  -H "Authorization: Bearer admin-token"
```

**获取后端健康状态**

```bash
curl http://localhost:3000/admin/backend-health \
  -H "Authorization: Bearer admin-token"
```

**SmartAI 权重查看**

```bash
curl http://localhost:3000/smart-ai/weights
curl http://localhost:3000/smart-ai/models/gpt-4/weights
```

## 📊 完整API端点

| 端点 | 方法 | 认证 | 描述 |
|------|------|------|------|
| `/` | GET | ❌ | 服务首页 |
| `/health` | GET | ❌ | 基础健康检查 |
| `/metrics` | GET | ❌ | 详细性能指标 |
| `/prometheus` | GET | ❌ | Prometheus格式指标 |
| `/models` | GET | ✅ | 可用模型列表 |
| `/v1/chat/completions` | POST | ✅ | 聊天完成（OpenAI兼容） |
| `/v1/models` | GET | ✅ | 模型列表（OpenAI兼容） |
| `/v1/health` | GET | ❌ | OpenAI兼容健康检查 |
| `/admin/model-weights` | GET | ✅ | 模型权重信息 |
| `/admin/backend-health` | GET | ✅ | 后端健康状态 |
| `/admin/system-stats` | GET | ✅ | 系统统计信息 |
| `/smart-ai/weights` | GET | ❌ | SmartAI全局权重 |
| `/smart-ai/models/{model}/weights` | GET | ❌ | 特定模型SmartAI权重 |