# Berry API Workspace 结构

## 📁 项目重构说明

项目已成功重构为多个workspace，以便更好地组织代码结构，避免臃肿的单体架构。

## 🏗️ Workspace 结构

```
berry-api/
├── Cargo.toml                 # Workspace 根配置
├── berry-core/                # 核心库
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── config/            # 配置管理
│       └── auth/              # 认证系统
├── berry-loadbalance/         # 负载均衡库
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       └── loadbalance/       # 负载均衡逻辑
├── berry-relay/               # 请求转发库
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       └── relay/             # 请求转发处理
├── berry-api/                 # API服务器
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── app.rs
│       ├── router/            # 路由处理
│       └── static_files.rs
└── berry-cli/                 # 命令行工具
    ├── Cargo.toml
    └── src/
        └── main.rs
```

## 📦 各Workspace功能

### 1. **berry-core** - 核心库
- **配置管理**: 配置文件加载、验证、数据结构定义
- **认证系统**: 用户认证、权限控制、中间件
- **共享类型**: 所有workspace共用的基础类型和工具

**主要导出**:
```rust
pub use berry_core::{Config, Provider, Model, UserToken, Backend, LoadBalanceStrategy, BillingMode};
pub use berry_core::{AuthenticatedUser, AuthError, AuthMiddleware};
```

### 2. **berry-loadbalance** - 负载均衡库
- **负载均衡策略**: 权重随机、轮询、最低延迟等
- **健康检查**: 自动检测后端健康状态
- **指标收集**: 性能指标和健康状态统计
- **故障转移**: 智能故障检测和恢复

**主要导出**:
```rust
pub use berry_loadbalance::{LoadBalanceService, LoadBalanceManager, BackendSelector, HealthChecker};
```

### 3. **berry-relay** - 请求转发库
- **请求处理器**: 负载均衡的请求处理
- **客户端实现**: OpenAI兼容的HTTP客户端
- **协议适配**: 不同AI服务提供商的协议转换

**主要导出**:
```rust
pub use berry_relay::LoadBalancedHandler;
```

### 4. **berry-api** - API服务器
- **路由管理**: 所有API端点的路由配置
- **应用状态**: 全局应用状态管理
- **静态文件**: 监控界面的静态文件服务
- **主程序**: 服务器启动和生命周期管理

**可执行文件**: `berry-api`

### 5. **berry-cli** - 命令行工具
- **配置验证**: 验证配置文件的正确性
- **健康检查**: 手动触发健康检查
- **管理命令**: 各种管理和调试功能

**可执行文件**: `berry-cli`

## 🔧 编译和运行

### 编译整个workspace
```bash
cargo build --workspace
```

### 编译特定组件
```bash
cargo build -p berry-api
cargo build -p berry-cli
```

### 运行API服务器
```bash
cargo run -p berry-api
```

### 运行CLI工具
```bash
# 验证配置
cargo run -p berry-cli -- validate-config --config config.toml

# 健康检查
cargo run -p berry-cli -- health-check --config config.toml
```

## 📋 依赖关系

```
berry-api     → berry-core, berry-loadbalance, berry-relay
berry-relay   → berry-core, berry-loadbalance  
berry-loadbalance → berry-core
berry-cli     → berry-core, berry-loadbalance
berry-core    → (无内部依赖)
```

## ✅ 重构优势

1. **模块化**: 每个workspace专注于特定功能
2. **可复用**: 核心库可以被其他项目使用
3. **独立开发**: 不同团队可以独立开发不同模块
4. **测试隔离**: 每个模块可以独立测试
5. **编译优化**: 只编译需要的组件
6. **依赖清晰**: 明确的依赖关系，避免循环依赖

## 🚧 TODO

1. **完善健康检查**: 重新实现真实的AI provider健康检查
2. **完善请求转发**: 优化OpenAI客户端实现
3. **添加更多CLI功能**: 配置生成、性能监控等
4. **文档完善**: 为每个workspace添加详细文档
5. **测试覆盖**: 为每个模块添加完整的单元测试

## 📝 注意事项

- 当前健康检查使用临时实现，需要在重构完成后实现真正的健康检查逻辑
- 某些功能可能需要在workspace之间重新设计接口
- 配置文件格式保持不变，向后兼容
