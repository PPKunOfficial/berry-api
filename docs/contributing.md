# 🤝 贡献指南

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

1.  Fork 项目到个人仓库
2.  创建功能分支：`git checkout -b feature/new-feature`
3.  提交更改：`git commit -am 'Add new feature'`
4.  推送分支：`git push origin feature/new-feature`
5.  创建 Pull Request
