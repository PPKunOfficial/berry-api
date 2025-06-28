# ⚙️ 配置指南

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
