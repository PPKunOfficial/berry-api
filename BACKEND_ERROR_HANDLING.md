# 后端错误处理改进

## 概述

本次改进增强了系统在后端出错时的错误处理能力，现在能够获取并输出后端的具体报错内容，帮助用户更好地理解和调试问题。

## 主要改进

### 1. 错误响应体获取

在以下三个场景中，系统现在会获取后端的错误响应体内容：

- **流式请求错误处理** (`try_streaming_request`)
- **非流式请求错误处理** (`try_non_streaming_request`)  
- **后台非流式请求错误处理** (`try_non_streaming_request_with_keepalive`)

### 2. 错误信息解析

新增了三个辅助方法来处理后端错误信息：

- `extract_backend_error()` - 从错误字符串中提取后端的具体错误内容
- `parse_http_error()` - 解析HTTP错误信息，提取状态码和错误消息
- `create_backend_error_response()` - 创建包含后端错误信息的响应

### 3. 智能错误分类

系统现在能够：
- 根据HTTP状态码自动分类错误类型
- 尝试解析JSON格式的后端错误响应
- 提取后端API返回的具体错误消息

## 错误响应格式

### 之前的错误响应
```json
{
  "error": {
    "message": "Request processing failed for model 'gpt-4'",
    "type": "InternalServerError",
    "status": 500,
    "details": "Request failed after multiple attempts. If the problem persists, contact support. Details: HTTP error 429"
  }
}
```

### 改进后的错误响应
```json
{
  "error": {
    "message": "Request processing failed for model 'gpt-4'",
    "type": "TooManyRequests", 
    "status": 429,
    "details": "Backend error: Rate limit exceeded. Please try again later."
  }
}
```

## 支持的后端错误格式

系统能够解析以下格式的后端错误：

### 1. JSON格式错误
```json
{
  "error": {
    "message": "Invalid API key provided",
    "type": "invalid_request_error"
  }
}
```

### 2. 纯文本错误
```
Rate limit exceeded. Please try again later.
```

### 3. HTML错误页面
系统会提取HTML中的错误信息或返回状态码信息。

## 错误类型映射

| HTTP状态码 | 错误类型 | 描述 |
|-----------|---------|------|
| 400 | BadRequest | 请求格式错误 |
| 401 | Unauthorized | 认证失败 |
| 403 | Forbidden | 权限不足 |
| 404 | NotFound | 资源未找到 |
| 408 | RequestTimeout | 请求超时 |
| 429 | TooManyRequests | 请求过多 |
| 503 | ServiceUnavailable | 服务不可用 |
| 504 | GatewayTimeout | 网关超时 |
| 其他 | InternalServerError | 内部服务器错误 |

## 使用示例

### 场景1：OpenAI API密钥错误
**后端返回：**
```json
{
  "error": {
    "message": "Incorrect API key provided",
    "type": "invalid_request_error"
  }
}
```

**系统响应：**
```json
{
  "error": {
    "message": "Incorrect API key provided",
    "type": "Unauthorized",
    "status": 401,
    "details": "Backend error: Incorrect API key provided"
  }
}
```

### 场景2：Claude API速率限制
**后端返回：**
```json
{
  "error": {
    "type": "rate_limit_error",
    "message": "Rate limit exceeded"
  }
}
```

**系统响应：**
```json
{
  "error": {
    "message": "Rate limit exceeded",
    "type": "TooManyRequests", 
    "status": 429,
    "details": "Backend error: Rate limit exceeded"
  }
}
```

## 日志记录

系统现在会在debug级别记录详细的后端错误信息：

```
DEBUG Streaming request failed with status: 429, body: {"error":{"message":"Rate limit exceeded"}}
DEBUG Non-streaming request failed with status: 401, body: {"error":{"message":"Invalid API key"}}
```

## 测试

新增了单元测试来验证错误处理功能：

- `test_extract_backend_error()` - 测试后端错误提取
- `test_parse_http_error()` - 测试HTTP错误解析

运行测试：
```bash
cargo test test_extract_backend_error
cargo test test_parse_http_error
```

## 兼容性

此改进完全向后兼容，不会影响现有的API接口和响应格式。只是在错误情况下提供了更详细和有用的信息。

## 错误处理封装改进

### 统一错误处理系统

为了解决代码中分散的错误处理逻辑，我们创建了一套统一的错误处理系统：

#### 1. ErrorHandler - 统一错误处理器
```rust
pub struct ErrorHandler;

impl ErrorHandler {
    /// 从anyhow::Error创建HTTP响应
    pub fn from_anyhow_error(error: &anyhow::Error, context: Option<&str>) -> impl IntoResponse

    /// 从HTTP状态码和响应体创建错误响应
    pub fn from_http_error(status_code: u16, response_body: &str, context: Option<&str>) -> impl IntoResponse

    /// 创建业务逻辑错误响应
    pub fn business_error(error_type: ErrorType, message: &str, details: Option<String>) -> impl IntoResponse

    /// 创建配置错误响应
    pub fn config_error(message: &str, details: Option<String>) -> impl IntoResponse

    /// 创建认证错误响应
    pub fn auth_error(message: &str, error_type: Option<ErrorType>) -> impl IntoResponse

    /// 创建后端不可用错误响应
    pub fn backend_unavailable(model_name: &str, details: Option<String>) -> impl IntoResponse
}
```

#### 2. ErrorRecorder - 错误记录工具
```rust
pub struct ErrorRecorder;

impl ErrorRecorder {
    /// 记录请求失败到负载均衡器
    pub async fn record_request_failure(...)

    /// 记录请求失败（字符串错误）
    pub async fn record_failure_with_message(...)

    /// 记录HTTP错误失败
    pub async fn record_http_failure(...)
}
```

#### 3. RetryErrorHandler - 重试错误处理器
```rust
pub struct RetryErrorHandler;

impl RetryErrorHandler {
    /// 处理重试过程中的错误
    pub fn handle_retry_error(...) -> Result<(), anyhow::Error>

    /// 创建重试失败的最终错误
    pub fn create_final_error(...) -> anyhow::Error
}
```

#### 4. ResponseBodyHandler - 响应体处理器
```rust
pub struct ResponseBodyHandler;

impl ResponseBodyHandler {
    /// 安全地读取响应体，处理读取失败的情况
    pub async fn read_error_body(response: reqwest::Response) -> (u16, String)

    /// 读取响应体并记录调试信息
    pub async fn read_and_log_error_body(response: reqwest::Response, request_type: &str) -> (u16, String)
}
```

### 重构前后对比

#### 重构前 - 分散的错误处理
```rust
// 每个地方都有重复的错误处理逻辑
let error_body = match response.text().await {
    Ok(body) => body,
    Err(e) => {
        tracing::warn!("Failed to read error response body: {}", e);
        "Failed to read error response".to_string()
    }
};

self.load_balancer
    .record_request_result(
        provider,
        model,
        RequestResult::Failure {
            error: format!("HTTP {} - {}", status, error_body),
        },
    )
    .await;

if attempt == max_retries - 1 {
    return Err(anyhow::anyhow!("Request failed after {} attempts: {}", max_retries, e));
}
tracing::warn!("Request failed on attempt {}, retrying: {}", attempt + 1, e);
```

#### 重构后 - 统一的错误处理
```rust
// 使用统一的工具类
let (status, error_body) = ResponseBodyHandler::read_and_log_error_body(
    response,
    "Streaming request"
).await;

ErrorRecorder::record_http_failure(
    &self.load_balancer,
    provider,
    model,
    status,
    &error_body,
).await;

if let Err(final_error) = RetryErrorHandler::handle_retry_error(
    attempt,
    max_retries,
    &anyhow::anyhow!("{}", e),
    "Request processing",
) {
    return Err(final_error);
}
```

### 改进效果

1. **代码重用** - 消除了大量重复的错误处理代码
2. **一致性** - 所有错误处理都使用相同的格式和逻辑
3. **可维护性** - 错误处理逻辑集中管理，易于修改和扩展
4. **可测试性** - 每个错误处理组件都可以独立测试
5. **日志统一** - 所有错误都有一致的日志格式

### 测试覆盖

新增了针对统一错误处理系统的测试：

- `test_error_handler_from_anyhow()` - 测试从anyhow错误创建响应
- `test_error_handler_from_http()` - 测试从HTTP错误创建响应
- `test_error_handler_business_error()` - 测试业务错误处理

运行测试：
```bash
cargo test test_error_handler
```
