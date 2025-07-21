# 🛠️ 命令行工具 (berry-cli)

Berry CLI 提供了丰富的运维管理功能：

### 📋 配置管理

**验证配置文件**

用于验证 Berry API 配置文件的语法和逻辑是否正确。

```bash
# 验证默认配置文件 (config.toml)
berry-cli validate-config

# 验证指定配置文件
berry-cli validate-config --config /path/to/your_config.toml

# 输出示例
# ✅ Configuration is valid
#   - 2 providers configured
#   - 3 models configured
#   - 5 users configured
```

**生成配置文件**

用于生成 Berry API 的示例配置文件，可以生成基础版或包含所有高级功能的版本。

```bash
# 生成基础配置文件到 config_example.toml
berry-cli generate-config --output config_example.toml

# 生成包含所有高级功能的配置文件到 advanced_config.toml
berry-cli generate-config --output advanced_config.toml --advanced
```

### 🏥 健康检查

用于检查 Berry API 后端服务的健康状态，可以检查所有配置的提供商，或指定特定提供商。

```bash
# 检查所有提供商的健康状态 (使用默认配置文件 config.toml)
berry-cli health-check

# 检查所有提供商的健康状态 (使用指定配置文件)
berry-cli health-check --config /path/to/your_config.toml

# 检查特定提供商的健康状态，例如 'openai'
berry-cli health-check --config config.toml --provider openai

# 输出示例
# ✅ Health check completed
# ✅ Provider openai is healthy
# ❌ Provider anthropic health check failed: ...
```

### 📊 指标查看

用于显示 Berry API 服务的运行时指标和统计信息，可以查看基础指标或包含详细后端统计信息的详细指标。

```bash
# 查看基础服务指标 (使用默认配置文件 config.toml)
berry-cli metrics

# 查看基础服务指标 (使用指定配置文件)
berry-cli metrics --config /path/to/your_config.toml

# 查看详细的服务指标，包括每个后端（Provider:Model）的请求数、失败数和延迟
berry-cli metrics --config config.toml --detailed

# 输出示例 (基础指标)
# 📊 Service Metrics
# ==================
# Service Status: 🟢 Running
# Total Requests: 1000
# Successful Requests: 980
# Success Rate: 98.00%
#
# 🏥 Health Summary
# =================
# Total Providers: 2
# Healthy Providers: 2
# Total Models: 3
# Healthy Models: 3
# Provider Health Ratio: 100.00%
# Model Health Ratio: 100.00%

# 输出示例 (详细指标)
# 📈 Detailed Backend Statistics
# ==============================
# Backend: openai:gpt-3.5-turbo
#   Status: 🟢 Healthy
#   Requests: 500
#   Failures: 10
#   Latency: 150ms
#
# Backend: anthropic:claude-3-sonnet-20240229
#   Status: 🔴 Unhealthy
#   Requests: 200
#   Failures: 50
#   Latency: 300ms
```

### 🧪 后端测试

用于测试特定提供商和模型的连接性，包括对 `/v1/models` 和 `/v1/chat/completions` API 的测试。

```bash
# 测试 OpenAI 的 gpt-4 模型连接性
berry-cli test-backend --config config.toml --provider openai --model gpt-4

# 输出示例
# 🔍 Testing connectivity to openai:gpt-4
# Base URL: https://api.openai.com
#
# Testing models API: https://api.openai.com/v1/models
# Models API Status: 200 OK
# ✅ Models API test passed
#
# Testing chat completions API: https://api.openai.com/v1/chat/completions
# Chat API Status: 200 OK
# ✅ Chat API test passed
# 🎉 Backend openai:gpt-4 is fully functional!
```

### 🔧 CLI 安装

```bash
# 编译CLI工具
cargo build --release -p berry-cli

# 安装到系统路径 (可能需要管理员权限)
sudo cp target/release/berry-cli /usr/local/bin/

# 验证安装
berry-cli --help
