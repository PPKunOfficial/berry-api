# 流式请求错误处理修复

## 问题描述

之前的实现在流式请求失败时，仍然返回HTTP 200状态码，但在SSE流中包含错误信息。这不符合HTTP语义，客户端无法通过状态码判断请求是否成功。

## 修复内容

### 1. 问题分析

**修复前的行为**:
```
HTTP/1.1 200 OK
Content-Type: text/event-stream

data: {"error": {"message": "All retry attempts failed", "details": "..."}}
```

**问题**:
- HTTP状态码为200，表示成功
- 错误信息在SSE流中，客户端需要解析流才能发现错误
- 不符合HTTP语义，混淆了传输层和应用层的错误

### 2. 修复方案

**修复后的行为**:
```
HTTP/1.1 503 Service Unavailable
Content-Type: application/json

{
  "error": {
    "message": "Service temporarily unavailable for model 'gpt-4o'",
    "details": "All backends are currently unhealthy or unavailable. Details: ..."
  }
}
```

### 3. 错误状态码映射

修复后的系统会根据错误类型返回正确的HTTP状态码：

| 错误类型 | HTTP状态码 | 说明 |
|---------|-----------|------|
| 后端选择失败 | 503 Service Unavailable | 所有后端不可用 |
| 请求超时 | 504 Gateway Timeout | 上游服务超时 |
| 配置错误 | 500 Internal Server Error | API密钥等配置问题 |
| 通用错误 | 500 Internal Server Error | 其他内部错误 |

### 4. 代码修改

#### 主要修改点

1. **保持重试机制**: 流式请求失败时仍然会触发重试
2. **错误传播**: 所有重试失败后，错误会向上传播到主处理函数
3. **状态码映射**: 主处理函数根据错误类型返回正确的HTTP状态码
4. **删除错误SSE流**: 移除了在SSE流中包含错误的兜底方法

#### 关键代码变更

```rust
// 修复前：返回错误SSE流
Err(e) => {
    let error_stream = futures::stream::once(async move {
        Ok(Event::default().data(json!({"error": ...}).to_string()))
    }).boxed();
    Sse::new(error_stream)
}

// 修复后：让错误向上传播
Err(e) => Err(anyhow::anyhow!("Streaming request failed: {}", e))
```

### 5. 测试方法

#### 测试流式请求错误处理

```bash
# 测试后端不可用的情况
curl -v -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer invalid-token" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Hello"}],
    "stream": true
  }'
```

**预期结果**:
```
< HTTP/1.1 503 Service Unavailable
< Content-Type: application/json
{
  "error": {
    "message": "Service temporarily unavailable for model 'gpt-4o'",
    "details": "All backends are currently unhealthy or unavailable..."
  }
}
```

#### 测试非流式请求（对比）

```bash
# 测试非流式请求的错误处理
curl -v -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer invalid-token" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Hello"}],
    "stream": false
  }'
```

**预期结果**: 同样返回正确的HTTP错误状态码

### 6. 客户端处理建议

修复后，客户端可以正确处理流式请求的错误：

```javascript
// JavaScript 示例
fetch('/v1/chat/completions', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer your-token'
  },
  body: JSON.stringify({
    model: 'gpt-4o',
    messages: [{role: 'user', content: 'Hello'}],
    stream: true
  })
})
.then(response => {
  if (!response.ok) {
    // 现在可以正确检测到错误
    throw new Error(`HTTP ${response.status}: ${response.statusText}`);
  }
  
  // 只有成功时才处理SSE流
  const reader = response.body.getReader();
  // ... 处理流式数据
})
.catch(error => {
  console.error('Request failed:', error);
});
```

### 7. 兼容性说明

这个修复是**向后兼容**的：

- ✅ 成功的流式请求行为不变
- ✅ 非流式请求行为不变  
- ✅ 错误响应格式保持一致
- ⚠️ 错误情况下HTTP状态码从200变为正确的错误码

### 8. 监控和日志

修复后的错误处理会产生更清晰的日志：

```
ERROR berry_api_api::relay::handler::loadbalanced: All retry attempts failed for model 'gpt-4o': Backend selection failed for model 'gpt-4o' after 3 attempts...
```

客户端也可以通过HTTP状态码进行监控和告警。

### 9. 相关文件

- `api/src/relay/handler/loadbalanced.rs` - 主要修复文件
- `api/src/relay/handler/types.rs` - 错误响应创建函数

### 10. 总结

这个修复确保了：

1. **HTTP语义正确性**: 错误时返回错误状态码，成功时返回200
2. **客户端友好**: 客户端可以通过状态码快速判断请求结果
3. **保持功能完整**: 重试机制和错误处理逻辑保持不变
4. **向后兼容**: 成功情况下的行为完全不变

现在流式请求的错误处理符合HTTP标准，客户端可以正确处理各种错误情况。
