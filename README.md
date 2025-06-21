# Berry API - 智能AI负载均衡网关

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-GPL--3.0-blue.svg)](LICENSE)
[![Docker](https://img.shields.io/badge/docker-ready-blue.svg)](Dockerfile)
[![OpenAI Compatible](https://img.shields.io/badge/OpenAI-Compatible-green.svg)](https://platform.openai.com/docs/api-reference)

Berry API 是一个高性能、生产就绪的AI服务负载均衡网关，专为多AI提供商环境设计。它提供智能负载均衡、自动故障转移、健康检查和成本优化功能，完全兼容OpenAI API格式。

## ✨ 核心特性

### 🎯 智能负载均衡
- **8种负载均衡策略**：加权随机、轮询、最低延迟、故障转移、SmartAI等
- **SmartAI策略**：基于成本感知的智能选择，小流量健康检查优化
- **权重故障转移**：结合权重分配和自动故障切换
- **用户标签过滤**：支持基于用户标签的后端选择

### 🏥 智能健康检查
- **差异化检查策略**：按token计费执行主动检查，按请求计费使用被动验证
- **自动故障恢复**：支持渐进式权重恢复（30%→50%→100%）
- **熔断机制**：自动熔断故障服务，防止级联失败
- **实时监控**：提供详细的健康状态和性能指标

### 🔐 企业级认证
- **Token认证**：基于Bearer Token的用户认证
- **权限控制**：细粒度的模型访问权限管理
- **速率限制**：支持用户级别的请求频率控制
- **用户标签**：支持用户分组和权限标签

### 🚀 高性能架构
- **异步处理**：基于Tokio的高并发异步架构
- **流式支持**：完整支持SSE流式和非流式响应
- **连接保活**：智能保活机制防止连接超时
- **配置热重载**：运行时配置更新，无需重启服务

### 📊 可观测性
- **Prometheus指标**：完整的性能和健康指标导出
- **结构化日志**：支持多级别日志和调试模式
- **健康检查端点**：提供详细的服务状态信息
- **管理API**：丰富的管理和监控接口

## 📚 文档导航

### 🚀 快速开始
- **[⚡ 快速开始指南](QUICKSTART.md)** - 5分钟部署运行
- **[🐳 Docker部署](#-docker部署)** - 容器化部署方案
- **[⚙️ 配置指南](#-配置指南)** - 详细配置说明

### 📖 使用指南
- **[🔌 API参考文档](API_REFERENCE.md)** - 完整的API接口文档
- **[🏥 健康检查](#-健康检查与故障处理)** - 健康检查机制详解
- **[⚖️ 负载均衡](#-负载均衡策略详解)** - 负载均衡策略选择
- **[🔐 认证授权](#-认证与权限管理)** - 用户认证和权限配置

### 🛠️ 运维指南
- **[📊 监控告警](#-监控与可观测性)** - 监控指标和告警配置
- **[🔧 故障排除](#-故障排除)** - 常见问题和解决方案
- **[🎯 性能优化](#-性能优化)** - 性能调优建议
- **[🔄 配置管理](#-配置热重载)** - 配置热重载和管理

### 🏗️ 开发指南
- **[🏛️ 架构设计](ARCHITECTURE.md)** - 系统架构和组件设计
- **[🧪 测试指南](#-测试与调试)** - 测试和调试方法
- **[🤝 贡献指南](#-贡献指南)** - 开发环境和贡献流程

## 🏗️ 系统架构

Berry API 采用模块化架构设计，由5个核心模块组成：

```
┌─────────────────────────────────────────────────────────────────┐
│                        Berry API Gateway                        │
├─────────────────┬─────────────────┬─────────────────┬───────────┤
│   berry-api     │  berry-relay    │ berry-loadbalance│berry-core │
│   Web服务层     │   请求转发层    │   负载均衡层     │  核心库   │
│                 │                 │                 │           │
│ • HTTP路由      │ • 请求转发      │ • 后端选择      │ • 配置管理│
│ • 认证中间件    │ • 流式处理      │ • 健康检查      │ • 认证系统│
│ • 静态文件      │ • 错误处理      │ • 指标收集      │ • 共享类型│
│ • 管理接口      │ • 协议适配      │ • 策略实现      │ • 工具函数│
└─────────────────┴─────────────────┴─────────────────┴───────────┘
                                │
                    ┌───────────┴───────────┐
                    │      berry-cli        │
                    │     命令行工具        │
                    │                       │
                    │ • 配置验证            │
                    │ • 健康检查            │
                    │ • 指标查看            │
                    │ • 后端测试            │
                    └───────────────────────┘
```

### 🔄 请求处理流程

```mermaid
graph TD
    A[客户端请求] --> B[认证中间件]
    B --> C{认证成功?}
    C -->|否| D[返回401错误]
    C -->|是| E[权限检查]
    E --> F{有权限?}
    F -->|否| G[返回403错误]
    F -->|是| H[速率限制检查]
    H --> I{未超限?}
    I -->|否| J[返回429错误]
    I -->|是| K[负载均衡选择]
    K --> L[后端健康检查]
    L --> M{后端健康?}
    M -->|否| N[重试其他后端]
    M -->|是| O[转发请求]
    O --> P[处理响应]
    P --> Q{流式响应?}
    Q -->|是| R[SSE流式传输]
    Q -->|否| S[JSON响应]
    R --> T[返回客户端]
    S --> T
    N --> K
```

### 🧩 核心组件

| 组件 | 功能 | 技术栈 |
|------|------|--------|
| **berry-api** | Web服务层，提供HTTP API | Axum, Tower |
| **berry-relay** | 请求转发层，处理上游请求 | Reqwest, Tokio |
| **berry-loadbalance** | 负载均衡层，实现选择策略 | 自研算法, Metrics |
| **berry-core** | 核心库，配置和认证管理 | Serde, TOML |
| **berry-cli** | 命令行工具，运维管理 | Clap, 配置验证 |

## ⚖️ 负载均衡策略

Berry API 提供8种负载均衡策略，适应不同的业务场景：

| 策略 | 适用场景 | 优势 | 配置复杂度 |
|------|----------|------|------------|
| `weighted_random` | 成本控制、按性能分配 | 灵活的权重分配 | ⭐⭐ |
| `round_robin` | 简单均衡、相同性能后端 | 完全均匀分配 | ⭐ |
| `least_latency` | 性能优化、延迟敏感 | 自动选择最快后端 | ⭐⭐ |
| `failover` | 高可用、主备场景 | 明确的优先级 | ⭐⭐ |
| `weighted_failover` | 智能负载均衡 | 结合权重和故障转移 | ⭐⭐⭐ |
| `smart_weighted_failover` | 渐进式恢复 | 支持按请求计费优化 | ⭐⭐⭐ |
| `smart_ai` | 成本感知优化 | 小流量健康检查 | ⭐⭐⭐⭐ |
| `random` | 简单场景、测试 | 实现简单 | ⭐ |

### 🧠 SmartAI策略详解

SmartAI是Berry API的核心创新，专为小流量、成本敏感的场景设计：

**核心特性：**
- **成本感知选择**：优先选择便宜的后端，premium后端作为备选
- **小流量优化**：80%选择最佳后端，20%探索其他选项
- **智能健康检查**：基于用户请求进行被动健康验证
- **信心度机制**：动态调整后端选择权重

**工作原理：**
```
1. 初始化：所有后端获得初始信心度(0.8)
2. 请求处理：根据信心度和权重选择后端
3. 结果反馈：成功提升信心度，失败降低信心度
4. 动态调整：信心度影响下次选择概率
5. 探索机制：20%流量用于测试其他后端
```

### 🏥 健康检查机制

Berry API 实现了差异化的健康检查策略：

**按计费模式分类：**
- **按Token计费**：执行主动健康检查（调用模型API）
- **按请求计费**：使用被动验证（基于用户请求结果）

**检查流程：**
```
定期检查 → 模型列表API → 简单聊天测试 → 更新健康状态
     ↓
用户请求 → 成功/失败 → 自动恢复/标记故障
     ↓
渐进恢复 → 30% → 50% → 100% 权重恢复
```

## ⚡ 快速开始

### 📋 环境要求

| 组件 | 版本要求 | 推荐版本 |
|------|----------|----------|
| **Rust** | 1.70+ | 1.75+ |
| **操作系统** | Linux/macOS/Windows | Ubuntu 22.04+ |
| **内存** | 512MB+ | 1GB+ |
| **CPU** | 1核+ | 2核+ |
| **网络** | 访问AI服务商API | 稳定网络连接 |

### 🚀 一键部署

**方式1：Docker部署（推荐）**
```bash
# 1. 克隆项目
git clone https://github.com/PPKunOfficial/berry-api.git
cd berry-api

# 2. 复制配置文件
cp smart_ai_example.toml config.toml

# 3. 编辑配置文件，添加你的API密钥
vim config.toml

# 4. 启动服务
docker-compose up -d

# 5. 验证服务
curl http://localhost:3000/health
```

**方式2：源码编译**
```bash
# 1. 克隆项目
git clone https://github.com/PPKunOfficial/berry-api.git
cd berry-api

# 2. 编译项目（启用可观测性功能）
cargo build --release --features observability

# 3. 配置文件
cp smart_ai_example.toml config.toml
# 编辑config.toml，配置你的AI服务提供商

# 4. 启动服务
RUST_LOG=info ./target/release/berry-api

# 5. 验证服务
curl http://localhost:3000/health
```

### ⚙️ 基础配置

**配置文件加载机制**

Berry API 支持灵活的配置文件加载，按以下优先级顺序：

1. **环境变量** `CONFIG_PATH` 指定的路径
2. **默认路径** `config.toml`（当前目录）
3. **示例配置** `config-example.toml`
4. **SmartAI示例** `config/smart_ai_example.toml`

**使用配置模板**
```bash
# 方式1：复制完整配置示例
cp config-example.toml config.toml

# 方式2：复制SmartAI配置示例
cp config/smart_ai_example.toml config.toml

# 方式3：使用环境变量指定配置文件
export CONFIG_PATH=/path/to/your/config.toml

# 编辑配置文件
vim config.toml
```

**最小配置示例**：
```toml
# 全局设置
[settings]
health_check_interval_seconds = 30
request_timeout_seconds = 30
max_retries = 3
max_internal_retries = 2

# 用户认证
[users.admin]
name = "Administrator"
token = "berry-admin-token-12345"
allowed_models = []  # 空数组表示允许访问所有模型
enabled = true

# AI服务提供商
[providers.openai]
name = "OpenAI"
base_url = "https://api.openai.com/v1"
api_key = "sk-your-openai-key-here"
models = ["gpt-4", "gpt-3.5-turbo"]
enabled = true

# 模型映射
[models.gpt_4]
name = "gpt-4"
strategy = "weighted_failover"  # 推荐策略
enabled = true

[[models.gpt_4.backends]]
provider = "openai"
model = "gpt-4"
weight = 1.0
priority = 1
enabled = true
```

**配置文件说明**：
- `config-example.toml` - 完整配置示例，包含所有选项和详细注释
- `smart_ai_example.toml` - SmartAI策略专用配置示例
- 根据需要选择合适的模板进行修改

### 🧪 快速测试

```bash
# 1. 检查服务状态
curl http://localhost:3000/health

# 2. 获取可用模型
curl -H "Authorization: Bearer berry-admin-token-12345" \
     http://localhost:3000/v1/models

# 3. 发送聊天请求
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer berry-admin-token-12345" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello!"}],
    "stream": false
  }'
```

### 🐳 Docker部署

Berry API 提供了两种 Docker 构建方式，以满足不同的性能和使用需求：

#### 🚀 方式一：预编译构建（推荐）

**特点：** 在宿主机预编译，避免 Docker 内编译的性能损失

```bash
# 1. 预编译二进制文件
cargo build --bin berry-api --release --target x86_64-unknown-linux-gnu

# 2. 准备 Docker 构建文件
mkdir -p ./docker-binaries
cp target/x86_64-unknown-linux-gnu/release/berry-api ./docker-binaries/

# 3. 构建 Docker 镜像
docker build -f Dockerfile.prebuilt -t berry-api:latest .

# 4. 运行容器
docker run -p 3000:3000 -v ./config.toml:/app/config.toml:ro berry-api:latest
```

**或使用便捷脚本：**
```bash
# 一键预编译构建
./scripts/build-precompiled-docker.sh
```

#### 🔧 方式二：传统多阶段构建

**特点：** 完全在容器内编译，适合本地开发

```bash
# 使用传统 Dockerfile 构建
docker build -f Dockerfile -t berry-api:latest .
```

#### 📦 Docker Compose 部署

```yaml
# docker-compose.yml
services:
  berry-api:
    image: ppkun00/berry-api:latest  # 使用预构建镜像
    # 或者本地构建: build: .
    ports:
      - "3000:3000"
    environment:
      - RUST_LOG=info
      - CONFIG_PATH=/app/config.toml
    volumes:
      - ./config.toml:/app/config.toml:ro
    restart: unless-stopped
```

> 📖 **详细说明：** 查看 [DOCKER_BUILD.md](DOCKER_BUILD.md) 了解两种构建方式的详细对比和使用指南

```bash
# 启动服务
docker-compose up -d

# 查看日志
docker-compose logs -f berry-api

# 停止服务
docker-compose down
```

## � 配置指南

### 🔧 配置文件结构

Berry API 使用TOML格式的配置文件，主要包含4个部分：

```toml
[settings]        # 全局设置
[users.*]         # 用户认证配置
[providers.*]     # AI服务提供商配置
[models.*]        # 模型映射配置
```

### ⚙️ 全局设置 (settings)

```toml
[settings]
# 基础设置
health_check_interval_seconds = 30    # 健康检查间隔
request_timeout_seconds = 30          # 请求超时时间
max_retries = 3                       # 最大重试次数
max_internal_retries = 2              # 内部重试次数
health_check_timeout_seconds = 10     # 健康检查超时

# 熔断器设置
circuit_breaker_failure_threshold = 5 # 熔断器失败阈值
circuit_breaker_timeout_seconds = 60  # 熔断器超时时间
recovery_check_interval_seconds = 120 # 恢复检查间隔

# SmartAI 设置（可选）
[settings.smart_ai]
initial_confidence = 0.8              # 初始信心度
min_confidence = 0.05                 # 最小信心度
enable_time_decay = true              # 启用时间衰减
exploration_ratio = 0.2               # 探索流量比例
```

### 👤 用户认证配置 (users)

```toml
# 管理员用户
[users.admin]
name = "Administrator"
token = "berry-admin-token-12345"
allowed_models = []                   # 空数组 = 访问所有模型
enabled = true
tags = ["admin", "unlimited"]

# 普通用户
[users.user1]
name = "Regular User"
token = "berry-user1-token-67890"
allowed_models = ["gpt-3.5-turbo"]   # 限制访问模型
enabled = true
tags = ["user", "basic"]
# 速率限制（可选）
[users.user1.rate_limit]
requests_per_minute = 60
requests_per_hour = 1000

# 高级用户
[users.premium]
name = "Premium User"
token = "berry-premium-token-abcde"
allowed_models = ["gpt-4", "claude-3"]
enabled = true
tags = ["premium", "advanced"]
```

### 🔌 Provider配置 (providers)

```toml
# OpenAI 配置
[providers.openai]
name = "OpenAI"
base_url = "https://api.openai.com/v1"
api_key = "sk-your-openai-key-here"
models = ["gpt-4", "gpt-3.5-turbo"]
enabled = true
timeout_seconds = 30
backend_type = "openai"               # 后端类型

# Azure OpenAI 配置
[providers.azure]
name = "Azure OpenAI"
base_url = "https://your-resource.openai.azure.com"
api_key = "your-azure-key-here"
models = ["gpt-4", "gpt-35-turbo"]
enabled = true
backend_type = "openai"
# 自定义请求头
[providers.azure.headers]
"api-version" = "2024-02-01"

# Anthropic Claude 配置
[providers.anthropic]
name = "Anthropic"
base_url = "https://api.anthropic.com"
api_key = "sk-ant-your-key-here"
models = ["claude-3-opus", "claude-3-sonnet"]
enabled = true
backend_type = "claude"               # Claude格式
```

### 🎯 模型映射配置 (models)

```toml
# 基础模型配置
[models.gpt_4]
name = "gpt-4"                        # 对外暴露的模型名
strategy = "weighted_failover"        # 负载均衡策略
enabled = true

# 后端配置 - 主要服务
[[models.gpt_4.backends]]
provider = "openai"
model = "gpt-4"
weight = 0.7                          # 70% 权重
priority = 1                          # 最高优先级
enabled = true
billing_mode = "per_token"            # 计费模式
tags = ["premium"]

# 后端配置 - 备用服务
[[models.gpt_4.backends]]
provider = "azure"
model = "gpt-4"
weight = 0.3                          # 30% 权重
priority = 2                          # 备用优先级
enabled = true
billing_mode = "per_token"
tags = ["enterprise"]
```

### 📋 配置文件模板

Berry API 提供了多个配置文件模板：

**1. 完整配置示例 (`config-example.toml`)**
- ✅ 包含所有配置选项和详细注释
- ✅ 8种负载均衡策略示例
- ✅ 多种用户权限配置
- ✅ 完整的Provider配置示例
- ✅ 安全和性能优化建议

**2. SmartAI专用配置 (`smart_ai_example.toml`)**
- ✅ SmartAI策略专用配置
- ✅ 成本感知负载均衡
- ✅ 小流量健康检查优化
- ✅ 信心度调整参数

**使用方法**：
```bash
# 使用完整配置模板
cp config-example.toml config.toml

# 使用SmartAI配置模板
cp smart_ai_example.toml config.toml

# 编辑配置文件
vim config.toml
```

## 🔌 API使用指南

Berry API 完全兼容 OpenAI API 格式，可以无缝替换现有的 OpenAI 客户端。

### 🔐 认证与权限管理

**认证方式**
```bash
Authorization: Bearer your-token-here
```

**权限控制**
- **管理员用户**：`allowed_models = []` 可访问所有模型
- **普通用户**：`allowed_models = ["gpt-4"]` 只能访问指定模型
- **用户标签**：支持基于标签的后端过滤

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

## 🔧 负载均衡策略详解

### 策略选择指南

| 策略 | 适用场景 | 优势 | 劣势 |
|------|----------|------|------|
| `weighted_random` | 成本控制、按性能分配 | 灵活的权重分配 | 可能不够均匀 |
| `round_robin` | 简单均衡、相同性能后端 | 完全均匀分配 | 不考虑后端性能差异 |
| `least_latency` | 性能优化、延迟敏感 | 自动选择最快后端 | 需要延迟统计 |
| `failover` | 高可用、主备场景 | 明确的优先级 | 主后端压力大 |
| `random` | 简单场景、测试 | 实现简单 | 无优化策略 |
| `weighted_failover` | 智能负载均衡 | 结合权重和故障转移 | 配置相对复杂 |

### 1. 加权随机 (weighted_random)
根据权重随机选择后端，适合按成本或性能分配流量：
```toml
[models.cost_optimized]
name = "cost-optimized"
strategy = "weighted_random"
enabled = true

[[models.cost_optimized.backends]]
provider = "cheap-provider"
model = "gpt-3.5-turbo"
weight = 0.7  # 70% 流量给便宜的服务
priority = 1
enabled = true

[[models.cost_optimized.backends]]
provider = "premium-provider"
model = "gpt-3.5-turbo"
weight = 0.3  # 30% 流量给高质量服务
priority = 2
enabled = true
```

### 2. 轮询 (round_robin)
依次轮询所有可用后端，适合性能相近的后端：
```toml
[models.balanced]
name = "balanced"
strategy = "round_robin"
enabled = true

[[models.balanced.backends]]
provider = "provider-a"
model = "gpt-4"
weight = 1.0  # 轮询中权重无效
priority = 1
enabled = true

[[models.balanced.backends]]
provider = "provider-b"
model = "gpt-4"
weight = 1.0
priority = 2
enabled = true
```

### 3. 最低延迟 (least_latency)
自动选择响应时间最短的后端：
```toml
[models.fast_response]
name = "fast-response"
strategy = "least_latency"
enabled = true

[[models.fast_response.backends]]
provider = "fast-provider"
model = "gpt-3.5-turbo"
weight = 1.0
priority = 1
enabled = true

[[models.fast_response.backends]]
provider = "slow-provider"
model = "gpt-3.5-turbo"
weight = 1.0
priority = 2
enabled = true
```

### 4. 故障转移 (failover)
按优先级顺序选择，主要用于主备场景：
```toml
[models.high_availability]
name = "high-availability"
strategy = "failover"
enabled = true

[[models.high_availability.backends]]
provider = "primary-provider"
model = "gpt-4"
weight = 1.0
priority = 1  # 最高优先级，优先使用
enabled = true

[[models.high_availability.backends]]
provider = "backup-provider"
model = "gpt-4"
weight = 1.0
priority = 2  # 备用，主服务故障时使用
enabled = true

[[models.high_availability.backends]]
provider = "emergency-provider"
model = "gpt-4"
weight = 1.0
priority = 3  # 应急，前两个都故障时使用
enabled = true
```

### 5. 权重故障转移 (weighted_failover) 🆕
结合权重选择和故障转移的智能策略：

**工作原理**：
1. **正常情况**: 从所有健康的后端中按权重随机选择
2. **故障情况**: 自动屏蔽不健康的后端，只在健康的后端中选择
3. **全部故障**: 如果所有后端都不健康，仍按权重选择（而非优先级）
4. **自动恢复**: 后端恢复健康后自动重新加入负载均衡

```toml
[models.smart_model]
name = "smart-model"
strategy = "weighted_failover"
enabled = true

[[models.smart_model.backends]]
provider = "openai-main"
model = "gpt-4"
weight = 0.6    # 60%权重 - 主要服务
priority = 1    # 最高优先级
enabled = true

[[models.smart_model.backends]]
provider = "openai-backup"
model = "gpt-4"
weight = 0.3    # 30%权重 - 备用服务
priority = 2    # 中等优先级
enabled = true

[[models.smart_model.backends]]
provider = "azure"
model = "gpt-4"
weight = 0.1    # 10%权重 - 应急服务
priority = 3    # 最低优先级
enabled = true
```

### 6. 随机 (random)
完全随机选择，适合简单场景：
```toml
[models.simple_random]
name = "simple-random"
strategy = "random"
enabled = true

[[models.simple_random.backends]]
provider = "provider-a"
model = "gpt-3.5-turbo"
weight = 1.0  # 随机策略中权重无效
priority = 1
enabled = true
```

## 🏥 健康检查与故障处理

### 健康检查配置
```toml
[settings]
health_check_interval_seconds = 30    # 检查间隔（秒）
circuit_breaker_failure_threshold = 5 # 熔断阈值
circuit_breaker_timeout_seconds = 60  # 熔断恢复时间（秒）
```

### 健康检查机制
1. **定期检查**: 每30秒自动检查所有Provider的健康状态
2. **模型列表验证**: 通过调用 `/v1/models` 端点验证服务可用性
3. **聊天请求测试**: 发送简单的聊天请求验证模型功能
4. **自动标记**: 根据检查结果自动标记Provider为健康/不健康

### 故障转移流程
当某个Provider出现故障时：

1. **故障检测**
   - API请求失败
   - 健康检查失败
   - 响应超时

2. **自动处理**
   - 立即标记为不健康
   - 将流量切换到其他健康的Provider
   - 记录故障指标

3. **恢复检测**
   - 定期重试故障的Provider
   - 健康检查通过后自动恢复
   - 用户请求成功也会触发恢复

4. **流量恢复**
   - 恢复后自动重新加入负载均衡
   - 按配置的权重分配流量

### 熔断机制
```
正常状态 ──失败次数达到阈值──▶ 熔断状态
    ▲                           │
    │                           │
    └──超时后自动尝试恢复────────┘
```

- **触发条件**: 连续失败次数达到 `circuit_breaker_failure_threshold`
- **熔断期间**: 不会向该Provider发送请求
- **自动恢复**: 超过 `circuit_breaker_timeout_seconds` 后自动尝试恢复

### 故障处理最佳实践

1. **多Provider配置**: 为每个模型配置多个Provider
2. **合理的权重分配**: 主Provider权重高，备用Provider权重低
3. **适当的超时设置**: 避免过长的等待时间
4. **监控告警**: 定期检查健康状态和指标

## 🧪 测试与调试

### 1. 单元测试
```bash
# 运行所有测试
cargo test

# 运行特定模块测试
cargo test loadbalance
cargo test config
cargo test auth

# 运行集成测试
cargo test --test integration

# 显示测试输出
cargo test -- --nocapture
```

### 2. 功能测试
```bash
# 测试基本功能
./test_auth.sh

# 调试演示
./debug_demo.sh

# 健康检查演示
cargo run --example initial_health_check_demo
```

### 3. 调试日志
启用详细日志进行调试：
```bash
# 启用调试日志
RUST_LOG=debug cargo run

# 只显示特定模块的日志
RUST_LOG=berry_api_api=debug cargo run

# 显示所有日志级别
RUST_LOG=trace cargo run
```

### 4. 配置验证
```bash
# 验证配置文件语法
cargo run -- --check-config

# 使用测试配置
CONFIG_PATH="test_config.toml" cargo run
```

### 5. 性能测试
```bash
# 使用 wrk 进行压力测试
wrk -t12 -c400 -d30s --script=test.lua http://localhost:3000/v1/chat/completions

# 使用 curl 测试延迟
time curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer berry-admin-token-12345" \
  -d '{"model": "gpt-4", "messages": [{"role": "user", "content": "Hello"}]}'
```

## 🛠️ 命令行工具 (berry-cli)

Berry CLI 提供了丰富的运维管理功能：

### 📋 配置管理

**验证配置文件**
```bash
# 验证默认配置
berry-cli validate-config

# 验证指定配置文件
berry-cli validate-config -c /path/to/config.toml

# 输出示例
✅ Configuration is valid
  - 2 providers configured
  - 3 models configured
  - 5 users configured
```

**生成配置文件**
```bash
# 生成基础配置
berry-cli generate-config -o config_example.toml

# 生成高级配置（包含所有功能）
berry-cli generate-config -o advanced_config.toml --advanced
```

### 🏥 健康检查

**检查所有后端**
```bash
berry-cli health-check -c config.toml
# 输出：✅ Health check completed
```

**检查特定Provider**
```bash
berry-cli health-check -c config.toml -p openai
# 输出：✅ Provider openai is healthy
```

### 📊 指标查看

**查看服务指标**
```bash
# 基础指标
berry-cli metrics -c config.toml

# 详细指标
berry-cli metrics -c config.toml --detailed
```

### 🧪 后端测试

**测试后端连接**
```bash
berry-cli test-backend -c config.toml -p openai -m gpt-4
# 输出：✅ Backend openai:gpt-4 connectivity test passed
```

### 🔧 CLI 安装

```bash
# 编译CLI工具
cargo build --release -p berry-cli

# 安装到系统路径
sudo cp target/release/berry-cli /usr/local/bin/

# 验证安装
berry-cli --help
```

## �️ 命令行工具 (berry-cli)

Berry CLI 提供了丰富的运维管理功能：

### 📋 配置管理

**验证配置文件**
```bash
# 验证默认配置
berry-cli validate-config

# 验证指定配置文件
berry-cli validate-config -c /path/to/config.toml

# 输出示例
✅ Configuration is valid
  - 2 providers configured
  - 3 models configured
  - 5 users configured
```

**生成配置文件**
```bash
# 生成基础配置
berry-cli generate-config -o config_example.toml

# 生成高级配置（包含所有功能）
berry-cli generate-config -o advanced_config.toml --advanced
```

### 🏥 健康检查

**检查所有后端**
```bash
berry-cli health-check -c config.toml
# 输出：✅ Health check completed
```

**检查特定Provider**
```bash
berry-cli health-check -c config.toml -p openai
# 输出：✅ Provider openai is healthy
```

### 📊 指标查看

**查看服务指标**
```bash
# 基础指标
berry-cli metrics -c config.toml

# 详细指标
berry-cli metrics -c config.toml --detailed
```

### 🧪 后端测试

**测试后端连接**
```bash
berry-cli test-backend -c config.toml -p openai -m gpt-4
# 输出：✅ Backend openai:gpt-4 connectivity test passed
```

### 🔧 CLI 安装

```bash
# 编译CLI工具
cargo build --release -p berry-cli

# 安装到系统路径
sudo cp target/release/berry-cli /usr/local/bin/

# 验证安装
berry-cli --help
```

## �📈 性能优化与部署

### 性能调优建议

1. **连接池优化**
   ```toml
   [settings]
   request_timeout_seconds = 30      # 根据网络情况调整
   max_retries = 3                   # 避免过多重试
   health_check_interval_seconds = 30 # 平衡检查频率和性能
   ```

2. **权重分配策略**
   - 根据Provider的实际性能和成本调整权重
   - 高性能Provider分配更高权重
   - 备用Provider保持较低权重

3. **超时设置**
   - 设置合理的请求超时时间
   - 避免过长的等待导致用户体验差
   - 考虑不同Provider的响应特性

4. **熔断参数**
   ```toml
   circuit_breaker_failure_threshold = 5  # 根据容错需求调整
   circuit_breaker_timeout_seconds = 60   # 平衡恢复速度和稳定性
   ```

## 📊 监控与可观测性

Berry API 提供完整的可观测性支持，包括指标收集、日志记录和健康监控。

### 🎯 核心指标

**HTTP 请求指标**
- `http_requests_total` - 总请求数（按状态码、方法、路径分类）
- `http_request_duration_seconds` - 请求延迟分布
- `http_requests_in_flight` - 当前处理中的请求数

**后端健康指标**
- `backend_health_status` - 后端健康状态（0=不健康，1=健康）
- `backend_request_count_total` - 后端请求总数
- `backend_error_count_total` - 后端错误总数
- `backend_latency_seconds` - 后端响应延迟

**负载均衡指标**
- `load_balance_selections_total` - 负载均衡选择次数
- `smart_ai_confidence_score` - SmartAI信心度分数
- `circuit_breaker_state` - 熔断器状态

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
   grep "latency" logs/berry-api.log
   ```

### 生产部署

1. **Docker部署**
   ```dockerfile
   FROM rust:1.70 as builder
   WORKDIR /app
   COPY . .
   RUN cargo build --release

   FROM debian:bookworm-slim
   RUN apt-get update && apt-get install -y ca-certificates
   COPY --from=builder /app/target/release/berry-api /usr/local/bin/
   COPY config.toml /etc/berry-api/
   EXPOSE 3000
   CMD ["berry-api"]
   ```

2. **Systemd服务**
   ```ini
   [Unit]
   Description=Berry API Load Balancer
   After=network.target

   [Service]
   Type=simple
   User=berry-api
   WorkingDirectory=/opt/berry-api
   Environment=CONFIG_PATH=/etc/berry-api/config.toml
   Environment=RUST_LOG=info
   ExecStart=/usr/local/bin/berry-api
   Restart=always
   RestartSec=5

   [Install]
   WantedBy=multi-user.target
   ```

3. **负载均衡部署**
   - 使用Nginx或HAProxy进行前端负载均衡
   - 部署多个Berry API实例
   - 配置健康检查和故障转移

4. **安全配置**
   - 使用HTTPS加密传输
   - 定期轮换API密钥
   - 限制网络访问权限
   - 启用访问日志审计

### 扩展性

1. **水平扩展**
   - 支持多实例部署
   - 无状态设计，易于扩展
   - 配置文件共享

2. **动态配置**
   - 支持运行时配置更新
   - 热重载Provider配置
   - 动态添加新模型

3. **插件化架构**
   - 可扩展的认证机制
   - 自定义负载均衡策略
   - 可插拔的监控组件

## 🔧 故障排除

### 🚨 常见问题诊断

**1. 服务启动失败**
```bash
# 检查配置文件语法
berry-cli validate-config -c config.toml

# 检查端口占用
lsof -i :3000
netstat -tulpn | grep :3000

# 查看详细错误信息
RUST_LOG=debug cargo run

# 检查依赖和编译
cargo check
cargo build --release
```

**2. Provider连接失败**
```bash
# 测试网络连接
curl -I https://api.openai.com/v1/models

# 验证API密钥
curl https://api.openai.com/v1/models \
  -H "Authorization: Bearer your-api-key"

# 检查防火墙和代理设置
export https_proxy=http://proxy:8080
```

**3. 认证失败**
```bash
# 验证Token格式
echo "berry-admin-token-12345" | wc -c

# 检查用户配置
berry-cli validate-config | grep users

# 测试认证
curl -H "Authorization: Bearer berry-admin-token-12345" \
     http://localhost:3000/v1/models
```

**4. 负载均衡异常**
```bash
# 检查后端健康状态
curl http://localhost:3000/admin/backend-health

# 查看负载均衡权重
curl http://localhost:3000/admin/model-weights

# 测试特定后端
berry-cli test-backend -p openai -m gpt-4
```

### 📊 日志分析与调试

**日志级别配置**
```bash
# 基础日志
export RUST_LOG=info

# 调试特定模块
export RUST_LOG=berry_loadbalance=debug,berry_relay=debug

# 详细跟踪
export RUST_LOG=trace
```

**关键日志查询**
```bash
# 服务启动日志
grep "Starting Berry API" logs/berry-api.log

# 健康检查状态
grep "health_check" logs/berry-api.log | tail -20

# 认证失败记录
grep "Authentication failed" logs/berry-api.log

# 负载均衡决策
grep "selected backend" logs/berry-api.log | tail -10

# 错误统计
grep "ERROR" logs/berry-api.log | cut -d' ' -f3 | sort | uniq -c

# 性能分析
grep "latency" logs/berry-api.log | jq '.fields.latency_ms' | \
  awk '{sum+=$1; count++} END {print "Average:", sum/count "ms"}'
```

### 🔄 配置热重载

Berry API 支持运行时配置更新，无需重启服务：

**热重载机制**
```bash
# 修改配置文件
vim config.toml

# 发送重载信号（如果支持）
kill -HUP $(pgrep berry-api)

# 或通过API重载（需要实现）
curl -X POST http://localhost:3000/admin/reload-config \
  -H "Authorization: Bearer admin-token"
```

**配置变更监控**
```bash
# 监控配置文件变化
inotifywait -m config.toml -e modify

# 验证新配置
berry-cli validate-config -c config.toml

# 比较配置差异
diff config.toml.backup config.toml
```

### 🛡️ 安全检查

**配置安全审计**
```bash
# 检查敏感信息泄露
grep -r "sk-" config/ --exclude="*.example"

# 验证Token强度
python3 -c "
import secrets
token = 'berry-admin-token-12345'
print(f'Token length: {len(token)}')
print(f'Entropy: {len(set(token))} unique chars')
"

# 检查文件权限
ls -la config.toml
# 应该是 -rw------- (600)
```

### 🔍 性能诊断

**延迟分析**
```bash
# 测试端到端延迟
time curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer token" \
  -d '{"model":"gpt-4","messages":[{"role":"user","content":"hi"}]}'

# 分析响应时间分布
for i in {1..10}; do
  time curl -s http://localhost:3000/health > /dev/null
done
```

**内存和CPU监控**
```bash
# 监控资源使用
top -p $(pgrep berry-api)
htop -p $(pgrep berry-api)

# 内存使用分析
ps aux | grep berry-api
cat /proc/$(pgrep berry-api)/status | grep -E "(VmRSS|VmSize)"
```

## 🚀 生产部署指南

### 🏭 生产环境配置

**系统要求**
```bash
# 推荐配置
CPU: 2核心以上
内存: 2GB以上
磁盘: 10GB以上
网络: 稳定的互联网连接

# 操作系统
Ubuntu 22.04 LTS (推荐)
CentOS 8+
Debian 11+
```

**环境变量配置**
```bash
# /etc/environment
RUST_LOG=info
CONFIG_PATH=/etc/berry-api/config.toml
BIND_ADDRESS=0.0.0.0:3000
MAX_CONNECTIONS=1000
```

### 🐳 Docker 生产部署

**推荐：预编译构建（性能优化）**
```bash
# CI/CD 流水线中的构建步骤
cargo build --workspace --release --features observability --target x86_64-unknown-linux-gnu
mkdir -p ./docker-binaries
cp target/x86_64-unknown-linux-gnu/release/berry-api ./docker-binaries/
cp target/x86_64-unknown-linux-gnu/release/berry-cli ./docker-binaries/
docker build -f Dockerfile.prebuilt -t berry-api:prod .
```

**备选：传统多阶段构建**
```dockerfile
# Dockerfile (已优化)
FROM rust:1.87-slim-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --workspace --release --features observability

FROM gcr.io/distroless/cc-debian12
WORKDIR /app
COPY --from=builder /app/target/release/berry-api /usr/local/bin/
COPY --from=builder /app/target/release/berry-cli /usr/local/bin/
EXPOSE 3000
CMD ["/usr/local/bin/berry-api"]
```

**Docker Compose 生产配置**
```yaml
# docker-compose.prod.yml
version: '3.8'
services:
  berry-api:
    build:
      context: .
      dockerfile: Dockerfile.prod
    ports:
      - "3000:3000"
    environment:
      - RUST_LOG=info
      - CONFIG_PATH=/app/config.toml
    volumes:
      - ./config.toml:/app/config.toml:ro
      - ./logs:/app/logs
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
    deploy:
      resources:
        limits:
          memory: 1G
          cpus: '1.0'
        reservations:
          memory: 512M
          cpus: '0.5'

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
      - ./ssl:/etc/nginx/ssl:ro
    depends_on:
      - berry-api
    restart: unless-stopped
```

### ⚖️ 负载均衡与高可用

**Nginx 配置**
```nginx
# nginx.conf
upstream berry_api {
    server berry-api-1:3000 weight=3;
    server berry-api-2:3000 weight=2;
    server berry-api-3:3000 weight=1 backup;
}

server {
    listen 80;
    listen 443 ssl http2;
    server_name api.yourdomain.com;

    # SSL配置
    ssl_certificate /etc/nginx/ssl/cert.pem;
    ssl_certificate_key /etc/nginx/ssl/key.pem;

    # 安全头
    add_header X-Frame-Options DENY;
    add_header X-Content-Type-Options nosniff;
    add_header X-XSS-Protection "1; mode=block";

    location / {
        proxy_pass http://berry_api;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # 超时配置
        proxy_connect_timeout 30s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;

        # 缓冲配置
        proxy_buffering on;
        proxy_buffer_size 4k;
        proxy_buffers 8 4k;
    }

    # 健康检查
    location /health {
        proxy_pass http://berry_api/health;
        access_log off;
    }
}
```

### 🔒 安全最佳实践

**1. API密钥管理**
```bash
# 使用环境变量或密钥管理服务
export OPENAI_API_KEY=$(vault kv get -field=api_key secret/openai)

# 定期轮换密钥
./scripts/rotate-api-keys.sh

# 密钥强度检查
python3 -c "
import secrets
import string
# 生成强密钥
key = ''.join(secrets.choice(string.ascii_letters + string.digits) for _ in range(32))
print(f'Strong API key: berry-{key}')
"
```

**2. 网络安全**
```bash
# 防火墙配置
ufw allow 22/tcp
ufw allow 80/tcp
ufw allow 443/tcp
ufw deny 3000/tcp  # 只允许内部访问
ufw enable

# 限制访问源
iptables -A INPUT -p tcp --dport 3000 -s 10.0.0.0/8 -j ACCEPT
iptables -A INPUT -p tcp --dport 3000 -j DROP
```

**3. 日志安全**
```toml
# config.toml - 生产配置
[settings]
# 不记录敏感信息
log_request_body = false
log_response_body = false
mask_api_keys = true
```

### 📊 监控与告警

**Prometheus + Grafana 部署**
```yaml
# monitoring/docker-compose.yml
version: '3.8'
services:
  prometheus:
    image: prom/prometheus
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus_data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--web.console.libraries=/etc/prometheus/console_libraries'
      - '--web.console.templates=/etc/prometheus/consoles'

  grafana:
    image: grafana/grafana
    ports:
      - "3001:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin123
    volumes:
      - grafana_data:/var/lib/grafana
      - ./grafana/dashboards:/etc/grafana/provisioning/dashboards
      - ./grafana/datasources:/etc/grafana/provisioning/datasources

volumes:
  prometheus_data:
  grafana_data:
```

### 🔄 CI/CD 流水线

**GitHub Actions 配置**
```yaml
# .github/workflows/deploy.yml
name: Deploy to Production

on:
  push:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo test --all-features

  build:
    needs: test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Build Docker image
        run: |
          docker build -t berry-api:${{ github.sha }} .
          docker tag berry-api:${{ github.sha }} berry-api:latest

      - name: Push to registry
        run: |
          echo ${{ secrets.DOCKER_PASSWORD }} | docker login -u ${{ secrets.DOCKER_USERNAME }} --password-stdin
          docker push berry-api:${{ github.sha }}
          docker push berry-api:latest

  deploy:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - name: Deploy to production
        run: |
          ssh ${{ secrets.PROD_SERVER }} "
            docker pull berry-api:latest
            docker-compose -f docker-compose.prod.yml up -d --no-deps berry-api
          "
```

## 🤝 贡献指南

### 🛠️ 开发环境设置

```bash
# 1. 克隆项目
git clone https://github.com/PPKunOfficial/berry-api.git
cd berry-api

# 2. 安装Rust工具链
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup component add clippy rustfmt

# 3. 安装依赖并编译
cargo build

# 4. 运行测试
cargo test --all-features

# 5. 代码质量检查
cargo fmt --check
cargo clippy -- -D warnings

# 6. 运行开发服务器
RUST_LOG=debug cargo run
```

### 📝 开发规范

**代码风格**
```bash
# 格式化代码
cargo fmt

# 检查代码质量
cargo clippy

# 运行所有检查
./scripts/check.sh
```

**提交规范**
```bash
# 提交格式
git commit -m "feat: add SmartAI load balancing strategy"
git commit -m "fix: resolve authentication timeout issue"
git commit -m "docs: update API documentation"

# 提交类型
feat: 新功能
fix: 修复bug
docs: 文档更新
style: 代码格式
refactor: 重构
test: 测试相关
chore: 构建/工具相关
```

**Pull Request 流程**
1. Fork 项目到个人仓库
2. 创建功能分支：`git checkout -b feature/new-feature`
3. 提交更改：`git commit -am 'Add new feature'`
4. 推送分支：`git push origin feature/new-feature`
5. 创建 Pull Request

### 🧪 测试指南

**单元测试**
```bash
# 运行所有测试
cargo test

# 运行特定模块测试
cargo test loadbalance
cargo test auth

# 生成测试覆盖率报告
cargo tarpaulin --out Html
```

**集成测试**
```bash
# 启动测试环境
docker-compose -f docker-compose.test.yml up -d

# 运行集成测试
cargo test --test integration

# 清理测试环境
docker-compose -f docker-compose.test.yml down
```

欢迎提交Issue和Pull Request！我们重视每一个贡献。

## � 常见用例

### 🏢 企业级AI服务

**场景：大型企业多部门AI服务**
```toml
# 企业配置示例
[settings]
health_check_interval_seconds = 30
max_internal_retries = 3

# 部门用户配置
[users.hr_dept]
name = "HR Department"
token = "hr-dept-token-12345"
allowed_models = ["gpt-3.5-turbo", "claude-3-haiku"]
tags = ["hr", "basic"]

[users.rd_dept]
name = "R&D Department"
token = "rd-dept-token-67890"
allowed_models = ["gpt-4", "claude-3-opus"]
tags = ["rd", "premium"]

# 成本优化配置
[models.cost_effective]
name = "cost-effective"
strategy = "smart_ai"
enabled = true

[[models.cost_effective.backends]]
provider = "cheap_provider"
model = "gpt-3.5-turbo"
weight = 0.8
billing_mode = "per_request"
tags = []

[[models.cost_effective.backends]]
provider = "premium_provider"
model = "gpt-4"
weight = 0.2
billing_mode = "per_token"
tags = ["premium"]
```

### 🚀 初创公司成本控制

**场景：预算有限的初创公司**
```toml
# 成本敏感配置
[models.startup_gpt4]
name = "gpt-4"
strategy = "smart_ai"  # 智能成本控制
enabled = true

[[models.startup_gpt4.backends]]
provider = "budget_provider"
model = "gpt-3.5-turbo"
weight = 1.0
priority = 1
enabled = true

[[models.startup_gpt4.backends]]
provider = "premium_provider"
model = "gpt-4"
weight = 0.1  # 仅作为备选
priority = 2
enabled = true
tags = ["premium"]
```

### 🔄 多云容灾部署

**场景：跨云服务商的高可用部署**
```toml
[models.ha_gpt4]
name = "gpt-4"
strategy = "weighted_failover"
enabled = true

# 主要云服务商
[[models.ha_gpt4.backends]]
provider = "aws_openai"
model = "gpt-4"
weight = 0.5
priority = 1
enabled = true

# 备用云服务商
[[models.ha_gpt4.backends]]
provider = "azure_openai"
model = "gpt-4"
weight = 0.3
priority = 2
enabled = true

# 应急服务商
[[models.ha_gpt4.backends]]
provider = "gcp_openai"
model = "gpt-4"
weight = 0.2
priority = 3
enabled = true
```

### 🧪 开发测试环境

**场景：开发团队测试不同AI模型**
```toml
[users.dev_team]
name = "Development Team"
token = "dev-team-token"
allowed_models = []  # 允许访问所有模型
enabled = true
tags = ["dev", "testing"]

# 测试模型配置
[models.test_model]
name = "test-model"
strategy = "round_robin"  # 轮询测试所有后端
enabled = true

[[models.test_model.backends]]
provider = "openai"
model = "gpt-3.5-turbo"
weight = 1.0
enabled = true

[[models.test_model.backends]]
provider = "anthropic"
model = "claude-3-sonnet"
weight = 1.0
enabled = true
```

## 🎯 最佳实践总结

### ✅ 配置最佳实践

1. **安全配置**
   - 使用强随机Token
   - 定期轮换API密钥
   - 限制用户模型访问权限
   - 启用请求日志审计

2. **性能优化**
   - 根据实际使用情况调整权重
   - 设置合理的超时时间
   - 使用SmartAI策略进行成本优化
   - 启用健康检查和熔断机制

3. **监控告警**
   - 配置Prometheus指标收集
   - 设置关键指标告警
   - 定期检查日志和性能
   - 监控成本和使用情况

### 🚀 部署最佳实践

1. **生产环境**
   - 使用Docker容器化部署
   - 配置负载均衡和高可用
   - 启用HTTPS和安全头
   - 实施备份和恢复策略

2. **扩展性**
   - 设计无状态架构
   - 支持水平扩展
   - 使用配置热重载
   - 实施蓝绿部署

## 📄 许可证

本项目采用 **GNU General Public License v3.0** 许可证。

这意味着：
- ✅ 可以自由使用、修改和分发
- ✅ 可以用于商业用途
- ⚠️ 修改后的代码必须开源
- ⚠️ 必须保留原始许可证声明

详细信息请查看 [LICENSE](LICENSE) 文件。

## 🔗 相关资源

### 📚 官方文档
- [OpenAI API 参考](https://platform.openai.com/docs/api-reference) - OpenAI官方API文档
- [Azure OpenAI 服务](https://docs.microsoft.com/en-us/azure/cognitive-services/openai/) - 微软Azure OpenAI文档
- [Anthropic Claude API](https://docs.anthropic.com/claude/reference/) - Anthropic官方API文档

### 🛠️ 技术栈
- [Rust 编程语言](https://www.rust-lang.org/) - 系统级编程语言
- [Tokio 异步运行时](https://tokio.rs/) - Rust异步编程框架
- [Axum Web框架](https://github.com/tokio-rs/axum) - 现代化Web服务框架
- [Serde 序列化](https://serde.rs/) - Rust序列化/反序列化框架
- [TOML 配置格式](https://toml.io/) - 人性化的配置文件格式

### 🔧 工具与集成
- [Prometheus 监控](https://prometheus.io/) - 开源监控系统
- [Grafana 可视化](https://grafana.com/) - 监控数据可视化平台
- [Docker 容器化](https://www.docker.com/) - 应用容器化平台
- [Nginx 负载均衡](https://nginx.org/) - 高性能Web服务器

### 🌟 社区与支持
- [GitHub 仓库](https://github.com/PPKunOfficial/berry-api) - 源代码和版本管理
- [Issues 问题反馈](https://github.com/PPKunOfficial/berry-api/issues) - Bug报告和功能请求
- [Discussions 讨论区](https://github.com/PPKunOfficial/berry-api/discussions) - 社区讨论和交流
- [Wiki 文档](https://github.com/PPKunOfficial/berry-api/wiki) - 详细文档和教程

### 📈 性能基准
- **并发处理**: 支持1000+并发连接
- **响应延迟**: 平均增加延迟<10ms
- **内存占用**: 基础运行内存<100MB
- **CPU使用**: 单核心可处理500+ QPS

---

<div align="center">

## 🚀 Berry API

**让AI服务负载均衡变得简单高效！**

[![Star on GitHub](https://img.shields.io/github/stars/PPKunOfficial/berry-api?style=social)](https://github.com/PPKunOfficial/berry-api)
[![Fork on GitHub](https://img.shields.io/github/forks/PPKunOfficial/berry-api?style=social)](https://github.com/PPKunOfficial/berry-api/fork)

**[快速开始](#-快速开始)** • **[配置指南](#-配置指南)** • **[API文档](#-api使用指南)** • **[部署指南](#-生产部署指南)**

</div>
