# Berry API 快速开始指南

## 🚀 5分钟快速部署

### 前置要求

- Docker 和 Docker Compose
- 至少一个AI服务提供商的API密钥（OpenAI、Azure OpenAI、Anthropic等）

### 步骤1：克隆项目

```bash
git clone https://github.com/PPKunOfficial/berry-api.git
cd berry-api
```

### 步骤2：配置服务

```bash
# 方式1：使用完整配置模板（推荐新手）
cp config-example.toml config.toml

# 编辑配置文件
vim config.toml
```

**最小配置示例**：
```toml
# 基础设置
[settings]
health_check_interval_seconds = 30
request_timeout_seconds = 30

# 用户配置
[users.admin]
name = "Administrator"
token = "berry-admin-token-12345"
allowed_models = []
enabled = true

# AI服务提供商
[providers.openai]
name = "OpenAI"
base_url = "https://api.openai.com/v1"
api_key = "sk-your-openai-key-here"  # 替换为你的API密钥
models = ["gpt-4", "gpt-3.5-turbo"]
enabled = true

# 模型映射
[models.gpt_4]
name = "gpt-4"
strategy = "weighted_failover"
enabled = true

[[models.gpt_4.backends]]
provider = "openai"
model = "gpt-4"
weight = 1.0
priority = 1
enabled = true
```

### 步骤3：启动服务

```bash
# 使用Docker Compose启动
docker-compose up -d

# 查看服务状态
docker-compose ps

# 查看日志
docker-compose logs -f berry-api
```

### 步骤4：验证部署

```bash
# 检查服务健康状态
curl http://localhost:3000/health

# 获取可用模型
curl -H "Authorization: Bearer berry-admin-token-12345" \
     http://localhost:3000/v1/models

# 发送测试请求
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer berry-admin-token-12345" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello!"}],
    "stream": false
  }'
```

## 🔧 常见配置场景

### 场景1：多Provider负载均衡

```toml
# 配置多个OpenAI账户
[providers.openai_primary]
name = "OpenAI Primary"
base_url = "https://api.openai.com/v1"
api_key = "sk-primary-key"
models = ["gpt-4"]
enabled = true

[providers.openai_backup]
name = "OpenAI Backup"
base_url = "https://api.openai.com/v1"
api_key = "sk-backup-key"
models = ["gpt-4"]
enabled = true

# 负载均衡配置
[models.gpt_4]
name = "gpt-4"
strategy = "weighted_failover"
enabled = true

[[models.gpt_4.backends]]
provider = "openai_primary"
model = "gpt-4"
weight = 0.7  # 70%流量
priority = 1
enabled = true

[[models.gpt_4.backends]]
provider = "openai_backup"
model = "gpt-4"
weight = 0.3  # 30%流量
priority = 2
enabled = true
```

### 场景2：成本优化配置

```toml
# 使用SmartAI策略进行成本控制
[models.cost_optimized]
name = "gpt-4"
strategy = "smart_ai"
enabled = true

[[models.cost_optimized.backends]]
provider = "cheap_provider"
model = "gpt-3.5-turbo"
weight = 1.0
enabled = true
tags = []  # 非premium，获得稳定性加成

[[models.cost_optimized.backends]]
provider = "premium_provider"
model = "gpt-4"
weight = 0.5
enabled = true
tags = ["premium"]  # premium标签
```

### 场景3：多用户权限管理

```toml
# 管理员用户
[users.admin]
name = "Administrator"
token = "admin-token-secure-123"
allowed_models = []  # 访问所有模型
enabled = true
tags = ["admin"]

# 开发团队
[users.dev_team]
name = "Development Team"
token = "dev-team-token-456"
allowed_models = ["gpt-3.5-turbo", "claude-3-haiku"]
enabled = true
tags = ["dev"]

# 高级用户
[users.premium_user]
name = "Premium User"
token = "premium-user-token-789"
allowed_models = ["gpt-4", "claude-3-opus"]
enabled = true
tags = ["premium"]
# 速率限制
[users.premium_user.rate_limit]
requests_per_minute = 100
requests_per_hour = 2000
```

## 🛠️ 开发环境设置

### 本地开发

```bash
# 安装Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 克隆项目
git clone https://github.com/PPKunOfficial/berry-api.git
cd berry-api

# 编译项目
cargo build

# 运行开发服务器
RUST_LOG=debug cargo run

# 运行测试
cargo test
```

### 使用CLI工具

```bash
# 编译CLI工具
cargo build --release -p berry-cli

# 验证配置
./target/release/berry-cli validate-config -c config.toml

# 检查后端健康
./target/release/berry-cli health-check -c config.toml

# 测试特定后端
./target/release/berry-cli test-backend -c config.toml -p openai -m gpt-4
```

## 📊 监控设置

### 基础监控

```bash
# 查看服务状态
curl http://localhost:3000/health

# 查看详细指标
curl http://localhost:3000/metrics

# 查看Prometheus格式指标
curl http://localhost:3000/prometheus
```

### Grafana仪表板

```bash
# 启动监控栈
cd monitoring
docker-compose up -d

# 访问Grafana
open http://localhost:3001
# 用户名: admin, 密码: admin123
```

## 🔍 故障排除

### 常见问题

**1. 服务启动失败**
```bash
# 检查配置
berry-cli validate-config -c config.toml

# 检查端口占用
lsof -i :3000

# 查看详细日志
RUST_LOG=debug docker-compose up
```

**2. API密钥错误**
```bash
# 测试API密钥
curl https://api.openai.com/v1/models \
  -H "Authorization: Bearer your-api-key"

# 检查配置文件中的密钥
grep "api_key" config.toml
```

**3. 认证失败**
```bash
# 验证Token
curl -H "Authorization: Bearer your-token" \
     http://localhost:3000/v1/models

# 检查用户配置
grep -A 5 "users\." config.toml
```

### 调试技巧

```bash
# 启用详细日志
export RUST_LOG=debug

# 查看特定模块日志
export RUST_LOG=berry_loadbalance=debug,berry_relay=debug

# 监控请求
tail -f logs/berry-api.log | grep "selected backend"

# 性能分析
time curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer token" \
  -d '{"model":"gpt-4","messages":[{"role":"user","content":"hi"}]}'
```

## 📚 下一步

- 阅读完整的 [README.md](README.md) 了解所有功能
- 查看 [ARCHITECTURE.md](ARCHITECTURE.md) 了解系统架构
- 浏览 `smart_ai_example.toml` 学习高级配置
- 加入 [GitHub Discussions](https://github.com/PPKunOfficial/berry-api/discussions) 参与社区讨论

## 🆘 获取帮助

- 📖 [完整文档](README.md)
- 🐛 [问题反馈](https://github.com/PPKunOfficial/berry-api/issues)
- 💬 [社区讨论](https://github.com/PPKunOfficial/berry-api/discussions)
- 📧 联系维护者

---

**恭喜！** 你已经成功部署了Berry API。现在可以开始使用智能AI负载均衡服务了！ 🎉
