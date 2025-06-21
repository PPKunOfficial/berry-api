# GitHub Actions 故障排除指南

本文档记录了 GitHub Actions 工作流程中常见的问题和解决方案。

## 🔐 权限问题

### 问题：Release 创建失败 (403 错误)

**错误信息：**
```
⚠️ GitHub release failed with status: 403
undefined
retrying... (2 retries remaining)
❌ Too many retries. Aborting...
Error: Too many retries.
```

**原因：**
GitHub Actions 默认的 `GITHUB_TOKEN` 权限不足以创建 Release。

**解决方案：**
在 workflow 文件中添加必要的权限配置：

```yaml
name: Docker Release

on:
  push:
    tags:
      - 'v*.*.*'

permissions:
  contents: write  # 允许创建 Release
  packages: write  # 允许推送到 GitHub Packages (可选)

jobs:
  build-and-docker:
    runs-on: ubuntu-latest
    # ... 其他步骤
```

**权限说明：**
- `contents: write` - 允许创建、编辑和删除仓库内容，包括 Release
- `packages: write` - 允许推送到 GitHub Packages（如果需要）

### 问题：Docker Hub 推送失败

**错误信息：**
```
Error: buildx failed with: ERROR: failed to solve: failed to push
```

**原因：**
Docker Hub 认证失败或权限不足。

**解决方案：**
1. 确保在 GitHub 仓库设置中配置了正确的 Secrets：
   - `DOCKERHUB_USERNAME`: Docker Hub 用户名
   - `DOCKERHUB_TOKEN`: Docker Hub 访问令牌

2. 检查 Docker Hub 访问令牌权限：
   - 登录 Docker Hub
   - 进入 Account Settings > Security
   - 创建新的访问令牌，确保有推送权限

## 🏗️ 构建问题

### 问题：Rust 编译失败

**常见原因和解决方案：**

1. **依赖版本冲突**
   ```bash
   # 清理缓存
   cargo clean
   # 更新依赖
   cargo update
   ```

2. **目标平台不支持**
   ```yaml
   # 确保安装了正确的目标平台
   - name: 设置 Rust 工具链
     uses: dtolnay/rust-toolchain@stable
     with:
       toolchain: stable
       targets: x86_64-unknown-linux-gnu
   ```

3. **功能特性问题**
   ```bash
   # 检查功能特性是否存在
   cargo build --bin berry-api --release --target x86_64-unknown-linux-gnu
   ```

### 问题：二进制文件不存在

**错误信息：**
```
cp: cannot stat 'target/x86_64-unknown-linux-gnu/release/berry-api': No such file or directory
```

**解决方案：**
1. 检查编译命令是否正确
2. 验证目标平台是否正确
3. 确保编译成功完成

```bash
# 验证编译结果
ls -la target/x86_64-unknown-linux-gnu/release/
file target/x86_64-unknown-linux-gnu/release/berry-api
```

## 🐳 Docker 问题

### 问题：Dockerfile 构建失败

**常见原因：**
1. 基础镜像不可用
2. 复制的文件路径错误
3. 权限问题

**解决方案：**
```dockerfile
# 确保使用稳定的基础镜像
FROM gcr.io/distroless/cc-debian12

# 检查文件路径
COPY ./docker-binaries/berry-api /usr/local/bin/berry-api

# 设置正确的权限
RUN chmod +x /usr/local/bin/berry-api  # 注意：distroless 镜像没有 shell
```

## 📋 最佳实践

### 1. 权限配置
```yaml
permissions:
  contents: write    # Release 创建
  packages: write    # 包推送
  actions: read      # 读取 Actions
  security-events: write  # 安全扫描（可选）
```

### 2. 错误处理
```yaml
- name: 编译检查
  run: |
    # 添加错误检查
    if [ ! -f "target/x86_64-unknown-linux-gnu/release/berry-api" ]; then
      echo "❌ 二进制文件不存在"
      exit 1
    fi
    
    # 验证文件类型
    file target/x86_64-unknown-linux-gnu/release/berry-api
```

### 3. 调试信息
```yaml
- name: 调试信息
  run: |
    echo "当前目录: $(pwd)"
    echo "文件列表:"
    find . -name "berry-api" -type f
    echo "环境变量:"
    env | grep -E "(GITHUB_|RUNNER_)"
```

### 4. 缓存优化
```yaml
- name: 设置 Rust 缓存
  uses: Swatinem/rust-cache@v2
  with:
    workspaces: "."
    cache-on-failure: true
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
```

## 🔍 调试技巧

### 1. 启用调试日志
```yaml
env:
  ACTIONS_STEP_DEBUG: true
  ACTIONS_RUNNER_DEBUG: true
```

### 2. 保留构建产物
```yaml
- name: 上传构建产物
  uses: actions/upload-artifact@v3
  if: failure()  # 只在失败时上传
  with:
    name: build-artifacts
    path: |
      target/
      docker-binaries/
      release-assets/
```

### 3. 条件执行
```yaml
- name: 创建 Release
  if: startsWith(github.ref, 'refs/tags/')
  uses: softprops/action-gh-release@v1
```

## 📞 获取帮助

如果遇到其他问题：

1. 检查 [GitHub Actions 文档](https://docs.github.com/en/actions)
2. 查看 [softprops/action-gh-release 文档](https://github.com/softprops/action-gh-release)
3. 在项目 Issues 中搜索相关问题
4. 创建新的 Issue 并提供详细的错误日志
