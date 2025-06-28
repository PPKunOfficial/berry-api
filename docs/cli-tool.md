# 🛠️ 命令行工具 (berry-cli)

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
