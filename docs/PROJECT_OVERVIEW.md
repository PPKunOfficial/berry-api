# Berry API - 智能AI负载均衡代理系统

## 📋 项目概述

Berry API 是一个高性能的AI服务负载均衡代理系统，专为多AI服务提供商环境设计。它提供OpenAI兼容的API接口，支持智能负载均衡、健康检查、故障转移和用户认证等企业级功能。

### 🎯 核心特性

- **🔄 智能负载均衡**: 支持加权随机、轮询、最低延迟、故障转移等多种策略
- **🏥 健康检查**: 自动监控后端服务状态，实现故障自动切换和恢复
- **🔐 用户认证**: 基于Token的用户认证和权限控制
- **📊 实时监控**: 性能指标收集和健康状态监控
- **🌊 流式支持**: 完整支持流式和非流式响应
- **⚡ 高性能**: 全异步架构，支持高并发请求处理
- **🔧 配置热重载**: 支持运行时配置更新，无需重启服务

## 🏗️ 系统架构

### 整体架构图
```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Client Apps   │───▶│   Berry API     │───▶│  AI Providers   │
│                 │    │  Load Balancer  │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                              │
                              ▼
                       ┌─────────────────┐
                       │   Monitoring    │
                       │   & Metrics     │
                       └─────────────────┘
```

### 核心组件

#### 1. 应用层 (`src/app.rs`)
- **AppState**: 全局应用状态管理
- **生命周期管理**: 服务启动和优雅关闭
- **依赖注入**: 组件间的依赖管理

#### 2. 配置系统 (`src/config/`)
- **Config结构**: 完整的配置数据结构定义
- **Provider配置**: AI服务提供商连接信息
- **Model映射**: 自定义模型到Provider模型的映射
- **用户管理**: 用户认证和权限配置
- **全局设置**: 超时、重试、健康检查等参数

#### 3. 认证系统 (`src/auth/`)
- **Bearer Token认证**: HTTP Authorization头认证
- **权限控制**: 用户模型访问权限验证
- **中间件集成**: 与Axum框架集成
- **安全错误处理**: 标准化认证错误响应

#### 4. 负载均衡系统 (`src/loadbalance/`)
- **LoadBalanceService**: 负载均衡主服务接口
- **LoadBalanceManager**: 管理所有模型的选择器
- **BackendSelector**: 实现具体的负载均衡策略
- **HealthChecker**: 定期检查后端健康状态
- **MetricsCollector**: 收集性能指标

#### 5. 请求转发系统 (`src/relay/`)
- **LoadBalancedHandler**: 负载均衡的请求处理器
- **OpenAI Client**: OpenAI兼容的客户端实现
- **错误处理**: 完善的错误处理和重试机制

#### 6. 路由系统 (`src/router/`)
- **API路由**: OpenAI兼容的API端点
- **健康检查**: 服务状态监控端点
- **监控指标**: 性能指标查询端点
- **管理接口**: 配置和状态管理接口

## 🔧 配置系统

### 配置文件结构
```toml
# 全局设置
[settings]
health_check_interval_seconds = 30
request_timeout_seconds = 30
max_retries = 3
circuit_breaker_threshold = 5

# AI服务提供商配置
[providers.openai]
name = "OpenAI"
base_url = "https://api.openai.com/v1"
api_key = "sk-xxx"
models = ["gpt-4", "gpt-3.5-turbo"]
enabled = true
timeout_seconds = 30

# 自定义模型映射
[models.gpt-4-smart]
name = "GPT-4 Smart"
enabled = true
strategy = "smart_ai"

[[models.gpt-4-smart.backends]]
provider = "openai"
model = "gpt-4"
weight = 0.7
billing_mode = "per_token"

# 用户认证配置
[users.user1]
name = "User 1"
token = "berry-xxx"
allowed_models = ["gpt-4-smart"]
enabled = true
```

### 配置验证
- 自动验证配置文件完整性
- 检查Provider连接性
- 验证模型映射关系
- 确保用户权限配置正确

## ⚖️ 负载均衡策略

### 1. 加权随机 (WeightedRandom)
根据配置的权重随机选择后端，权重越高被选中概率越大。

### 2. 轮询 (RoundRobin)
按顺序轮流选择后端，确保请求均匀分布。

### 3. 最低延迟 (LowestLatency)
选择平均响应时间最短的后端。

### 4. 故障转移 (Failover)
优先使用主要后端，故障时自动切换到备用后端。

### 5. Smart AI (smart_ai)
智能负载均衡策略，结合健康状态、成本控制和性能优化。

## 🏥 健康检查系统

### 健康检查机制
- **主动检查**: 定期发送健康检查请求
- **被动检查**: 根据实际请求结果判断健康状态
- **差异化策略**: 按计费模式区分检查策略
  - `per_token`: 执行主动聊天检查
  - `per_request`: 跳过主动检查，使用被动验证

### 故障处理
- **自动标记**: 请求失败时自动标记后端为不健康
- **权重调整**: 不健康后端使用降低的权重
- **渐进恢复**: 成功请求后逐步恢复权重 (10%→30%→50%→100%)
- **超时处理**: 请求超时也会标记后端为不健康

## 🔐 安全特性

### 认证机制
- **Bearer Token**: 基于HTTP Authorization头的认证
- **用户管理**: 支持启用/禁用用户
- **权限控制**: 细粒度的模型访问权限
- **安全日志**: 认证失败审计记录

### 配置安全
- **API密钥保护**: 直接存储在配置文件中
- **敏感信息**: 不在日志中记录敏感信息
- **权限控制**: 配置文件访问权限控制

## 📊 API接口

### OpenAI兼容接口
| 端点 | 方法 | 描述 |
|------|------|------|
| `/v1/chat/completions` | POST | 聊天完成接口 |
| `/v1/models` | GET | 获取可用模型列表 |

### 管理接口
| 端点 | 方法 | 描述 |
|------|------|------|
| `/health` | GET | 详细健康检查 |
| `/v1/health` | GET | 简单健康检查 |
| `/metrics` | GET | 性能指标查询 |
| `/smart-ai/weights` | GET | 查看所有模型权重 |
| `/smart-ai/models/{model}/weights` | GET | 查看特定模型权重 |

### 静态文件服务
| 端点 | 描述 |
|------|------|
| `/status` | 监控面板首页 |
| `/status/*` | 静态资源文件 |

## 🚀 部署方式

### Docker部署
```bash
# 构建镜像
docker build -t berry-api .

# 运行容器
docker run -d \
  --name berry-api \
  -p 8080:8080 \
  -v $(pwd)/config.toml:/app/config.toml \
  berry-api
```

### Docker Compose
```yaml
version: '3.8'
services:
  berry-api:
    build: .
    ports:
      - "8080:8080"
    volumes:
      - ./config.toml:/app/config.toml
    environment:
      - RUST_LOG=info
```

### 直接运行
```bash
# 编译
cargo build --release

# 运行
RUST_LOG=info ./target/release/berry-api
```

## 📈 性能特性

### 异步架构
- 全异步I/O处理
- 高并发支持
- 非阻塞请求处理

### 连接管理
- HTTP连接复用
- 智能超时控制
- Keep-alive支持

### 内存优化
- 零拷贝数据传输
- 流式响应处理
- 内存池管理

## 🔍 监控和调试

### 日志系统
- 结构化日志输出
- 可配置日志级别
- 请求链路追踪

### 指标收集
- 请求延迟统计
- 成功率监控
- 后端健康状态
- 负载均衡权重

### 调试功能
- 详细错误信息
- 请求参数验证
- 后端选择日志
- 健康检查日志

## 🛠️ 开发指南

### 环境要求
- Rust 1.70+
- 操作系统: Linux, macOS, Windows
- 内存: 最少512MB，推荐1GB+

### 快速开始
```bash
# 克隆项目
git clone https://github.com/PPKunOfficial/berry-api.git
cd berry-api

# 配置文件
cp config_example.toml config.toml

# 编译运行
cargo run
```

### 测试
```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test auth

# 集成测试
./test_auth.sh
```

## 📚 相关文档

- [API参考文档](API_REFERENCE.md)
- [配置指南](CONFIGURATION_EXAMPLES.md)
- [认证指南](AUTH_GUIDE.md)
- [架构详解](ARCHITECTURE.md)
- [使用指南](USAGE_GUIDE.md)
- [Docker部署](DOCKER_README.md)
- [调试指南](DEBUG_LOGGING_GUIDE.md)

## 🤝 贡献指南

欢迎提交Issue和Pull Request来改进项目。请确保：
- 遵循Rust代码规范
- 添加适当的测试
- 更新相关文档
- 提供清晰的提交信息

## 🔄 工作流程

### 请求处理流程
```
1. 客户端请求 → 2. 认证验证 → 3. 模型权限检查 → 4. 负载均衡选择
                                                              ↓
8. 返回响应 ← 7. 错误处理 ← 6. 健康状态更新 ← 5. 转发到后端
```

### 详细处理步骤

#### 1. 请求接收
- 解析HTTP请求头和Body
- 验证请求格式和参数
- 提取认证信息

#### 2. 用户认证
- 验证Bearer Token
- 检查用户是否启用
- 验证用户对请求模型的访问权限

#### 3. 负载均衡
- 根据模型配置选择负载均衡策略
- 考虑后端健康状态和权重
- 选择最优后端服务

#### 4. 请求转发
- 构建目标请求
- 设置超时和重试参数
- 发送请求到选中的后端

#### 5. 响应处理
- 处理流式和非流式响应
- 错误检测和分类
- 更新后端健康状态

#### 6. 结果返回
- 格式化响应数据
- 设置正确的HTTP状态码
- 返回给客户端

## 🎛️ 配置详解

### Provider配置详解
```toml
[providers.example]
name = "示例提供商"           # 显示名称
base_url = "https://api.example.com/v1"  # API基础URL
api_key = "your-api-key"      # API密钥
models = ["model1", "model2"] # 支持的模型列表
enabled = true                # 是否启用
timeout_seconds = 30          # 请求超时时间
max_retries = 3              # 最大重试次数

# 可选的自定义请求头
[providers.example.headers]
"User-Agent" = "Berry-API/1.0"
"X-Custom-Header" = "custom-value"
```

### Model映射配置详解
```toml
[models.custom-gpt4]
name = "自定义GPT-4"          # 面向客户的模型名称
enabled = true               # 是否启用此模型
strategy = "smart_ai"        # 负载均衡策略

# 后端配置 - 可以配置多个后端
[[models.custom-gpt4.backends]]
provider = "openai"          # 对应的Provider名称
model = "gpt-4"             # Provider中的实际模型名
weight = 0.7                # 权重 (0.0-1.0)
priority = 1                # 优先级 (数字越小优先级越高)
enabled = true              # 是否启用此后端
billing_mode = "per_token"  # 计费模式: per_token 或 per_request
tags = ["premium", "stable"] # 标签，用于分类和筛选

[[models.custom-gpt4.backends]]
provider = "azure"
model = "gpt-4"
weight = 0.3
priority = 2
enabled = true
billing_mode = "per_token"
tags = ["backup"]
```

### 用户配置详解
```toml
[users.example_user]
name = "示例用户"            # 用户显示名称
token = "berry-user-token"   # API Token
enabled = true              # 是否启用用户
allowed_models = ["custom-gpt4", "custom-gpt3"] # 允许访问的模型
tags = ["premium", "internal"] # 用户标签

# 可选的速率限制配置
[users.example_user.rate_limit]
requests_per_minute = 60    # 每分钟请求限制
requests_per_hour = 1000    # 每小时请求限制
requests_per_day = 10000    # 每天请求限制
```

### 全局设置详解
```toml
[settings]
# 健康检查间隔（秒）
health_check_interval_seconds = 30

# 默认请求超时时间（秒）
request_timeout_seconds = 30

# 默认最大重试次数
max_retries = 3

# 熔断器阈值（连续失败次数）
circuit_breaker_threshold = 5

# 连接超时时间（秒）
connection_timeout_seconds = 10

# 首字节超时时间（秒，仅用于流式请求）
first_byte_timeout_seconds = 30
```

## 🧠 Smart AI 负载均衡

### Smart AI 策略特点
- **成本优化**: 优先使用成本较低的后端
- **健康感知**: 自动避开不健康的后端
- **渐进恢复**: 不健康后端逐步恢复权重
- **小流量验证**: 低流量环境下的健康检查优化

### 权重计算逻辑
```rust
// 基础权重计算
base_weight = configured_weight

// 健康状态调整
if backend.is_healthy() {
    final_weight = base_weight
} else {
    // 不健康后端使用降低的权重
    final_weight = base_weight * health_penalty_factor
}

// 计费模式调整
if billing_mode == "per_request" && !is_healthy {
    // 按请求计费的不健康后端使用10%权重
    final_weight = base_weight * 0.1
}
```

### 健康恢复机制
```
不健康状态 → 成功请求 → 30%权重 → 成功请求 → 50%权重 → 成功请求 → 100%权重
```

## 🚨 错误处理

### 错误分类
- **认证错误** (401): Token无效或用户被禁用
- **权限错误** (403): 用户无权访问请求的模型
- **请求错误** (400): 请求格式错误或参数无效
- **服务不可用** (503): 所有后端都不健康
- **网关超时** (504): 后端响应超时
- **内部错误** (500): 系统内部错误

### 错误响应格式
```json
{
  "error": {
    "type": "service_unavailable",
    "message": "All backends are currently unavailable",
    "details": "No healthy backends found for model: gpt-4",
    "code": "BACKEND_UNAVAILABLE"
  }
}
```

### 重试机制
- **指数退避**: 重试间隔逐渐增加
- **最大重试次数**: 可配置的重试上限
- **错误类型过滤**: 只对可重试的错误进行重试
- **熔断保护**: 防止重试风暴

## 📊 监控指标

### 系统指标
- **请求总数**: 总请求计数
- **成功率**: 成功请求百分比
- **平均延迟**: 请求平均响应时间
- **错误率**: 各类错误的发生率

### 后端指标
- **健康状态**: 每个后端的健康状况
- **权重分布**: 当前权重分配情况
- **请求分布**: 请求在后端间的分布
- **响应时间**: 各后端的响应时间统计

### 用户指标
- **活跃用户**: 活跃用户数量
- **请求分布**: 用户请求分布
- **模型使用**: 各模型的使用情况
- **错误统计**: 用户相关的错误统计

## 🔧 故障排除

### 常见问题

#### 1. 配置文件错误
```bash
# 检查配置文件语法
cargo run -- --check-config

# 查看详细错误信息
RUST_LOG=debug cargo run
```

#### 2. 后端连接失败
```bash
# 检查网络连接
curl -H "Authorization: Bearer sk-xxx" https://api.openai.com/v1/models

# 查看健康检查日志
RUST_LOG=berry_api::loadbalance::health_checker=debug cargo run
```

#### 3. 认证问题
```bash
# 测试认证
curl -H "Authorization: Bearer your-token" http://localhost:8080/v1/models

# 查看认证日志
RUST_LOG=berry_api::auth=debug cargo run
```

#### 4. 负载均衡问题
```bash
# 查看权重分布
curl http://localhost:8080/smart-ai/weights

# 查看特定模型权重
curl http://localhost:8080/smart-ai/models/gpt-4/weights
```

### 调试技巧
- 使用 `RUST_LOG=debug` 获取详细日志
- 检查 `/health` 端点获取系统状态
- 使用 `/metrics` 端点查看性能指标
- 查看配置文件验证结果

## 💡 最佳实践

### 配置最佳实践

#### 1. Provider配置
```toml
# 建议为每个Provider设置合理的超时时间
[providers.openai]
timeout_seconds = 30  # OpenAI通常响应较快
max_retries = 3

[providers.claude]
timeout_seconds = 60  # Claude可能需要更长时间
max_retries = 2
```

#### 2. 权重分配策略
```toml
# 主要后端 + 备用后端的配置
[[models.gpt-4-balanced.backends]]
provider = "openai"
model = "gpt-4"
weight = 0.7          # 主要流量
priority = 1
tags = ["primary"]

[[models.gpt-4-balanced.backends]]
provider = "azure"
model = "gpt-4"
weight = 0.3          # 备用流量
priority = 2
tags = ["backup"]
```

#### 3. 用户权限管理
```toml
# 为不同类型用户设置不同权限
[users.admin]
allowed_models = []   # 空数组表示允许所有模型

[users.regular_user]
allowed_models = ["gpt-3.5-turbo", "claude-instant"]  # 限制访问

[users.premium_user]
allowed_models = ["gpt-4", "claude-2", "gpt-3.5-turbo"]
```

### 运维最佳实践

#### 1. 监控设置
```bash
# 设置适当的日志级别
export RUST_LOG="info,berry_api::loadbalance=debug"

# 定期检查健康状态
curl http://localhost:8080/health | jq .

# 监控权重分布
curl http://localhost:8080/smart-ai/weights | jq .
```

#### 2. 性能优化
- 根据实际使用情况调整健康检查间隔
- 为高频使用的模型配置更多后端
- 使用标签对后端进行分类管理
- 定期清理无效的配置项

#### 3. 安全建议
- 定期轮换API密钥
- 使用强随机Token作为用户认证
- 限制配置文件的访问权限
- 启用请求日志审计

## 🔍 使用示例

### 基本聊天请求
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-berry-token" \
  -d '{
    "model": "gpt-4-smart",
    "messages": [
      {"role": "user", "content": "Hello, how are you?"}
    ],
    "stream": false
  }'
```

### 流式聊天请求
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-berry-token" \
  -d '{
    "model": "gpt-4-smart",
    "messages": [
      {"role": "user", "content": "Write a short story"}
    ],
    "stream": true
  }'
```

### 指定后端请求（调试用）
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-berry-token" \
  -d '{
    "model": "gpt-4-smart",
    "messages": [
      {"role": "user", "content": "Hello"}
    ],
    "backend": "openai:gpt-4"
  }'
```

### 获取可用模型
```bash
curl -H "Authorization: Bearer your-berry-token" \
  http://localhost:8080/v1/models
```

### 查看系统状态
```bash
# 简单健康检查
curl http://localhost:8080/v1/health

# 详细健康检查
curl http://localhost:8080/health

# 性能指标
curl http://localhost:8080/metrics

# 负载均衡权重
curl http://localhost:8080/smart-ai/weights
```

## 🐳 Docker部署示例

### 基础部署
```dockerfile
# Dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/berry-api /usr/local/bin/
COPY --from=builder /app/public /app/public
WORKDIR /app
EXPOSE 8080
CMD ["berry-api"]
```

### Docker Compose完整示例
```yaml
version: '3.8'

services:
  berry-api:
    build: .
    ports:
      - "8080:8080"
    volumes:
      - ./config.toml:/app/config.toml:ro
      - ./logs:/app/logs
    environment:
      - RUST_LOG=info
      - RUST_BACKTRACE=1
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s

  # 可选：添加监控
  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml:ro
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--web.console.libraries=/etc/prometheus/console_libraries'
      - '--web.console.templates=/etc/prometheus/consoles'

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    volumes:
      - grafana-storage:/var/lib/grafana

volumes:
  grafana-storage:
```

## 🧪 测试指南

### 单元测试
```bash
# 运行所有测试
cargo test

# 运行特定模块测试
cargo test auth
cargo test loadbalance
cargo test config

# 运行测试并显示输出
cargo test -- --nocapture
```

### 集成测试
```bash
# 认证测试
./test_auth.sh

# 后端选择测试
./scripts/test_backend_selection.sh

# Smart AI API测试
./scripts/test_smart_ai_api.sh

# 流式错误处理测试
./scripts/test_streaming_errors.sh
```

### 性能测试
```bash
# 使用wrk进行压力测试
wrk -t12 -c400 -d30s \
  -H "Authorization: Bearer your-token" \
  -H "Content-Type: application/json" \
  --script=test_script.lua \
  http://localhost:8080/v1/chat/completions
```

## 🔄 版本升级指南

### 配置文件迁移
当升级到新版本时，请注意：
1. 备份现有配置文件
2. 检查新版本的配置格式变化
3. 使用配置验证功能确保配置正确
4. 逐步迁移配置项

### 平滑升级步骤
1. **准备阶段**
   ```bash
   # 备份配置
   cp config.toml config.toml.backup

   # 检查当前状态
   curl http://localhost:8080/health
   ```

2. **升级阶段**
   ```bash
   # 停止服务
   docker-compose down

   # 更新代码
   git pull origin main

   # 重新构建
   docker-compose build
   ```

3. **验证阶段**
   ```bash
   # 启动服务
   docker-compose up -d

   # 验证健康状态
   curl http://localhost:8080/health

   # 测试API功能
   ./test_auth.sh
   ```

## 📞 技术支持

### 问题报告
如果遇到问题，请提供以下信息：
- Berry API版本
- 配置文件（去除敏感信息）
- 错误日志
- 复现步骤
- 系统环境信息

### 社区资源
- GitHub Issues: 报告Bug和功能请求
- 文档Wiki: 详细使用文档
- 示例代码: 实际使用案例

## 📄 许可证

本项目采用MIT许可证，详见[LICENSE](../LICENSE)文件。
