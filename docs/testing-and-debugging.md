# 🧪 测试与调试

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

```