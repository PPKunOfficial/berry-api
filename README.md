# Berry API - 负载均衡AI网关

Berry API 是一个高性能的AI服务负载均衡网关，支持多种AI服务提供商的智能负载均衡、故障转移和健康检查。

## 🚀 特性

### 核心功能
- **多Provider支持**: 支持OpenAI、Azure OpenAI、Anthropic等多种AI服务提供商
- **智能负载均衡**: 支持加权随机、轮询、最低延迟、故障转移等多种负载均衡策略
- **健康检查**: 自动监控后端服务健康状态，实现故障自动切换
- **配置热重载**: 支持运行时配置更新，无需重启服务
- **OpenAI兼容**: 完全兼容OpenAI API格式，无缝替换

### 负载均衡策略
- **加权随机 (weighted_random)**: 根据权重随机选择后端
- **轮询 (round_robin)**: 依次轮询所有可用后端
- **最低延迟 (least_latency)**: 选择响应时间最短的后端
- **故障转移 (failover)**: 按优先级顺序选择，主要用于备份场景
- **随机 (random)**: 完全随机选择后端

### 监控与指标
- **实时健康状态**: 提供详细的服务健康状态信息
- **性能指标**: 记录请求延迟、成功率等关键指标
- **服务发现**: 自动发现和管理可用的模型服务

## 📋 系统架构

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   客户端请求     │───▶│  Berry API网关   │───▶│   AI服务提供商   │
│                │    │                  │    │                │
│ - OpenAI格式    │    │ - 负载均衡        │    │ - OpenAI        │
│ - 流式/非流式   │    │ - 健康检查        │    │ - Azure OpenAI  │
│ - 模型选择      │    │ - 故障转移        │    │ - Anthropic     │
└─────────────────┘    │ - 指标收集        │    │ - 其他代理服务   │
                       └──────────────────┘    └─────────────────┘
```

## 🛠️ 安装与配置

### 1. 环境要求
- Rust 1.70+
- Tokio异步运行时

### 2. 克隆项目
```bash
git clone https://github.com/your-repo/berry-api.git
cd berry-api
```

### 3. 配置文件
复制示例配置文件并根据需要修改：
```bash
cp config_example.toml config.toml
```

### 4. 配置API密钥
直接在配置文件中设置API密钥：
```toml
[providers.openai-primary]
name = "OpenAI Primary Account"
base_url = "https://api.openai.com/v1"
api_key = "sk-your-openai-key-here"  # 直接在配置文件中设置
models = ["gpt-4", "gpt-3.5-turbo"]
enabled = true
```

可选：设置配置文件路径环境变量
```bash
export CONFIG_PATH="config.toml"
```

### 5. 启动服务
```bash
cargo run
```

## 📝 配置说明

### Provider配置
```toml
[providers.openai-primary]
name = "OpenAI Primary Account"
base_url = "https://api.openai.com/v1"
api_key = "sk-your-openai-key-here"
models = ["gpt-4", "gpt-4-turbo", "gpt-3.5-turbo"]
enabled = true
timeout_seconds = 30
max_retries = 3
```

### 模型映射配置
```toml
[models.gpt_4]
name = "gpt-4"  # 对外暴露的模型名
strategy = "weighted_random"
enabled = true

[[models.gpt_4.backends]]
provider = "openai-primary"
model = "gpt-4"
weight = 0.5      # 权重
priority = 1      # 优先级
enabled = true
tags = ["premium"]
```

### 全局设置
```toml
[settings]
health_check_interval_seconds = 30
request_timeout_seconds = 30
max_retries = 3
circuit_breaker_failure_threshold = 5
circuit_breaker_timeout_seconds = 60
```

## 🔌 API使用

### 聊天完成 (兼容OpenAI)
```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{
    "model": "gpt-4",
    "messages": [
      {"role": "user", "content": "Hello, world!"}
    ],
    "stream": false
  }'
```

### 获取可用模型
```bash
curl http://localhost:3000/v1/models \
  -H "Authorization: Bearer your-api-key"
```

### 健康检查
```bash
curl http://localhost:3000/health
```

### 服务指标
```bash
curl http://localhost:3000/metrics
```

## 📊 监控端点

| 端点 | 方法 | 描述 |
|------|------|------|
| `/health` | GET | 服务健康状态 |
| `/metrics` | GET | 详细性能指标 |
| `/models` | GET | 可用模型列表 |
| `/v1/health` | GET | OpenAI兼容健康检查 |

## 🔧 高级配置

### 负载均衡策略选择

1. **高可用场景**: 使用`failover`策略，设置主备服务
2. **性能优化**: 使用`least_latency`策略，自动选择最快的服务
3. **成本控制**: 使用`weighted_random`策略，按成本分配权重
4. **简单均衡**: 使用`round_robin`策略，平均分配请求

### 健康检查配置
```toml
[settings]
health_check_interval_seconds = 30    # 检查间隔
circuit_breaker_failure_threshold = 5 # 熔断阈值
circuit_breaker_timeout_seconds = 60  # 熔断恢复时间
```

## 🚦 故障处理

### 自动故障转移
当某个provider出现故障时，系统会：
1. 自动标记为不健康
2. 将流量切换到其他健康的provider
3. 定期重试故障的provider
4. 恢复后自动重新加入负载均衡

### 熔断机制
- 连续失败达到阈值时触发熔断
- 熔断期间不会向该provider发送请求
- 超时后自动尝试恢复

## 🧪 测试

```bash
# 运行所有测试
cargo test

# 运行特定模块测试
cargo test loadbalance

# 运行集成测试
cargo test --test integration
```

## 📈 性能优化

### 建议配置
- 根据实际使用情况调整权重分配
- 设置合适的超时时间和重试次数
- 定期监控健康状态和性能指标
- 使用缓存减少配置加载开销

### 扩展性
- 支持动态添加新的provider
- 支持运行时配置更新
- 支持水平扩展部署

## 🤝 贡献

欢迎提交Issue和Pull Request！

## 📄 许可证

GNU GENERAL PUBLIC LICENSE Version 3

## 🔗 相关链接

- [OpenAI API文档](https://platform.openai.com/docs/api-reference)
- [Azure OpenAI文档](https://docs.microsoft.com/en-us/azure/cognitive-services/openai/)
- [Anthropic API文档](https://docs.anthropic.com/claude/reference/)
