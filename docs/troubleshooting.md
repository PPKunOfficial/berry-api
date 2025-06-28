# 🔧 故障排除

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