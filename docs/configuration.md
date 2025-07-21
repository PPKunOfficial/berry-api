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
health_check_interval_seconds = 30    # 健康检查间隔（秒），默认30秒
request_timeout_seconds = 30          # 请求超时时间（秒），默认30秒
max_retries = 3                       # 最大重试次数，默认3次
max_internal_retries = 2              # 内部重试次数，默认2次
health_check_timeout_seconds = 10     # 健康检查超时（秒），默认10秒

# 熔断器设置
circuit_breaker_failure_threshold = 5 # 熔断器失败阈值，默认5次
circuit_breaker_timeout_seconds = 60  # 熔断器超时时间（秒），默认60秒
recovery_check_interval_seconds = 120 # 恢复检查间隔（秒），默认120秒

# SmartAI 设置
[settings.smart_ai]
initial_confidence = 0.8              # 初始信心度，默认0.8
min_confidence = 0.05                 # 最小信心度（保留恢复机会），默认0.05
enable_time_decay = true              # 启用时间衰减，默认true
lightweight_check_interval_seconds = 600 # 轻量级检查间隔（秒），默认600秒 (10分钟)
exploration_ratio = 0.2               # 探索流量比例（用于测试其他后端），默认0.2
non_premium_stability_bonus = 1.1     # 非premium后端稳定性加成，默认1.1

# SmartAI 信心度调整参数
[settings.smart_ai.confidence_adjustments]
success_boost = 0.1                   # 成功请求信心度提升，默认0.1
network_error_penalty = 0.3           # 网络错误信心度惩罚，默认0.3
auth_error_penalty = 0.8              # 认证错误信心度惩罚，默认0.8
rate_limit_penalty = 0.1              # 速率限制错误信心度惩罚，默认0.1
server_error_penalty = 0.2            # 服务器错误信心度惩罚，默认0.2
model_error_penalty = 0.3             # 模型错误信心度惩罚，默认0.3
timeout_penalty = 0.2                 # 超时错误信心度惩罚，默认0.2

# 批量指标系统设置 (可选)
[settings.batch_metrics]
batch_size = 100                      # 批量大小，默认100
flush_interval_seconds = 5            # 刷新间隔（秒），默认5秒
buffer_size = 10000                   # 缓冲区大小，默认10000
enable_compression = false            # 是否启用压缩，默认false
```

### 👤 用户认证配置 (users)

```toml
# 管理员用户
[users.admin]
name = "Administrator"
token = "berry-admin-token-12345"
allowed_models = []                   # 空数组 = 访问所有模型
enabled = true
tags = ["admin", "unlimited"]         # 用户标签，用于路由选择器和权限控制

# 普通用户
[users.user1]
name = "Regular User"
token = "berry-user1-token-67890"
allowed_models = ["gpt-3.5-turbo"]   # 限制访问模型
enabled = true
tags = ["user", "basic"]
# 速率限制（可选）
[users.user1.rate_limit]
requests_per_minute = 60              # 每分钟请求数限制，默认无限制
requests_per_hour = 1000              # 每小时请求数限制，默认无限制
requests_per_day = 10000              # 每天请求数限制，默认无限制

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
enabled = true                        # 是否启用此Provider，默认true
timeout_seconds = 30                  # 请求超时时间（秒），默认30秒
max_retries = 3                       # 最大重试次数，默认3次
backend_type = "openai"               # 后端类型：openai, claude, gemini，默认openai
headers = {}                          # 自定义请求头，默认空

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

# Google Gemini 配置
[providers.gemini]
name = "Google Gemini"
base_url = "https://generativelanguage.googleapis.com/v1beta"
api_key = "your-gemini-key-here"
models = ["gemini-pro"]
enabled = true
backend_type = "gemini"               # Gemini格式
```

### 🎯 模型映射配置 (models)

```toml
# 基础模型配置
[models.gpt_4]
name = "gpt-4"                        # 对外暴露的模型名
strategy = "smart_ai"                 # 负载均衡策略：目前仅支持 "smart_ai"，默认 "smart_ai"
enabled = true                        # 是否启用此模型，默认true

# 后端配置 - 主要服务
[[models.gpt_4.backends]]
provider = "openai"                   # 引用Provider配置中的ID
model = "gpt-4"                       # Provider中实际的模型名
weight = 0.7                          # 权重，用于加权轮询，默认1.0
priority = 1                          # 优先级，数字越小优先级越高，默认0
enabled = true                        # 是否启用此后端，默认true
billing_mode = "per_token"            # 计费模式：per_token (按token计费，主动健康检查), per_request (按请求计费，被动验证)，默认per_token
tags = ["premium"]                    # 后端标签，用于路由选择器

# 后端配置 - 备用服务
[[models.gpt_4.backends]]
provider = "azure"
model = "gpt-4"
weight = 0.3
priority = 2
enabled = true
billing_mode = "per_token"
tags = ["enterprise"]
```

### 🚦 路由选择器配置 (Route Selector)

Berry API 的路由选择器允许根据用户、模型和后端标签动态选择最佳后端。

- **用户标签**: 在 `[users.*]` 配置中定义，例如 `tags = ["admin", "unlimited"]`。
- **后端标签**: 在 `[[models.YOUR_MODEL.backends]]` 配置中定义，例如 `tags = ["premium"]`。

当用户发起请求时，系统会根据用户的标签和后端标签进行匹配，优先选择具有共同标签的后端。如果用户没有标签，或者没有匹配的后端标签，则所有后端都可供选择。

### 🛡️ 配置验证规则和错误处理

Berry API 在启动时会对配置文件进行严格的验证，以确保配置的有效性和一致性。如果配置不符合以下规则，服务将无法启动并会报告详细的错误信息。

**全局设置 (settings) 验证:**
- `health_check_interval_seconds`: 必须大于0。
- `request_timeout_seconds`: 必须大于0，且小于等于300。
- `max_retries`: 必须小于等于10。
- `circuit_breaker_failure_threshold`: 必须大于0。
- `circuit_breaker_timeout_seconds`: 必须大于0。
- `recovery_check_interval_seconds`: 必须大于0。
- `max_internal_retries`: 必须大于0。
- `health_check_timeout_seconds`: 必须大于0。

**Provider 配置 (providers) 验证:**
- `name`: 不能为空。
- `base_url`: 不能为空，且必须以 `http://` 或 `https://` 开头。
- `api_key`: 不能为空，且长度至少为10个字符。
- `models`: 不能为空，且列表中的每个模型名称不能为空。
- `timeout_seconds`: 必须大于0，且小于等于300。
- `max_retries`: 必须小于等于10。
- `headers`: 自定义请求头名称和值不能为空。

**Model 映射配置 (models) 验证:**
- `name`: 不能为空，且不能包含空格、制表符或换行符。
- `backends`: 不能为空。
- 每个启用的后端必须具有正的 `weight`。

**Backend 配置验证:**
- `provider`: 必须引用一个已存在的Provider。
- `model`: 必须是对应Provider中定义的模型。
- `weight`: 必须大于0，且小于等于100。
- `priority`: 必须小于等于10。
- `tags`: 列表中的每个标签不能为空，且不能包含空格。

**用户认证配置 (users) 验证:**
- `name`: 不能为空。
- `token`: 不能为空，长度至少为16个字符，且不能包含空格、制表符或换行符。
- `allowed_models`: 列表中的每个模型名称不能为空，且必须引用一个已存在的模型。
- `tags`: 列表中的每个标签不能为空，且不能包含空格。
- **速率限制 (rate_limit) 验证 (如果存在):**
    - `requests_per_minute`, `requests_per_hour`, `requests_per_day`: 必须大于0。
    - 逻辑一致性：`requests_per_minute` <= `requests_per_hour` <= `requests_per_day`。
    - 合理性检查：`requests_per_minute` 最大1000，`requests_per_hour` 最大60000，`requests_per_day` 最大1440000。

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
