# Docker 构建说明

本项目提供了两种 Docker 构建方式，以满足不同的使用场景和性能需求。

## 🚀 方式一：预编译构建（推荐，用于 CI/CD）

### 特点
- ✅ **性能优化**：在宿主机上编译，避免 Docker 内编译的性能损失
- ✅ **缓存友好**：利用 GitHub Actions 的 Rust 缓存，大幅减少构建时间
- ✅ **资源节省**：Docker 构建阶段只需复制文件，无需编译环境
- ✅ **并行构建**：可以同时编译多个目标平台

### 使用场景
- GitHub Actions CI/CD 流水线
- 有预编译环境的生产部署
- 需要优化构建性能的场景

### 构建流程
```bash
# 1. 在宿主机编译二进制文件
cargo build --workspace --release --features observability --target x86_64-unknown-linux-gnu

# 2. 复制二进制文件到临时目录
mkdir -p ./docker-binaries
cp target/x86_64-unknown-linux-gnu/release/berry-api ./docker-binaries/
cp target/x86_64-unknown-linux-gnu/release/berry-cli ./docker-binaries/

# 3. 使用预编译 Dockerfile 构建镜像
docker build -f Dockerfile.prebuilt -t berry-api:latest .
```

### GitHub Actions 自动化
项目的 `.github/workflows/docker_release.yml` 已配置为使用此方式，并自动发布到 GitHub Release：

```yaml
- name: 编译 Rust 二进制文件
  run: |
    cargo build --workspace --release --features observability --target x86_64-unknown-linux-gnu
    mkdir -p ./docker-binaries
    cp target/x86_64-unknown-linux-gnu/release/berry-api ./docker-binaries/
    cp target/x86_64-unknown-linux-gnu/release/berry-cli ./docker-binaries/

- name: 准备 Release 文件
  run: |
    mkdir -p ./release-assets
    cp target/x86_64-unknown-linux-gnu/release/berry-api ./release-assets/berry-api-linux-x86_64
    cp target/x86_64-unknown-linux-gnu/release/berry-cli ./release-assets/berry-cli-linux-x86_64
    cd release-assets
    tar -czf berry-api-${{ github.ref_name }}-linux-x86_64.tar.gz berry-api-linux-x86_64 berry-cli-linux-x86_64
    sha256sum berry-api-${{ github.ref_name }}-linux-x86_64.tar.gz > berry-api-${{ github.ref_name }}-linux-x86_64.tar.gz.sha256

- name: 构建并推送 Docker 镜像
  uses: docker/build-push-action@v5
  with:
    file: ./Dockerfile.prebuilt

- name: 创建 GitHub Release
  uses: softprops/action-gh-release@v1
  with:
    files: |
      release-assets/berry-api-${{ github.ref_name }}-linux-x86_64.tar.gz
      release-assets/berry-api-${{ github.ref_name }}-linux-x86_64.tar.gz.sha256
```

**自动化流程包括：**
1. ✅ 预编译二进制文件（启用 observability 功能）
2. ✅ 构建并推送 Docker 镜像到 Docker Hub
3. ✅ 创建 GitHub Release 并上传二进制文件包
4. ✅ 生成 SHA256 校验和文件

## 🔧 方式二：传统多阶段构建（备选方案）

### 特点
- ✅ **简单易用**：一条命令完成所有构建
- ✅ **环境隔离**：完全在容器内编译，无需本地 Rust 环境
- ❌ **性能较慢**：每次都需要在容器内重新编译
- ❌ **资源消耗**：需要完整的编译环境镜像

### 使用场景
- 本地开发和测试
- 无法预编译的环境
- 简单的一次性构建

### 构建命令
```bash
# 使用传统 Dockerfile 构建
docker build -f Dockerfile -t berry-api:latest .

# 或者使用默认 Dockerfile（如果重命名）
docker build -t berry-api:latest .
```

## 📊 性能对比

| 构建方式 | 首次构建时间 | 增量构建时间 | 镜像大小 | 资源消耗 |
|----------|--------------|--------------|----------|----------|
| 预编译构建 | ~5-8分钟 | ~2-3分钟 | ~50MB | 低 |
| 传统构建 | ~15-20分钟 | ~10-15分钟 | ~50MB | 高 |

## 🛠️ 本地开发建议

### 开发阶段
```bash
# 本地快速测试，使用传统构建
docker build -f Dockerfile -t berry-api:dev .
docker run -p 3000:3000 berry-api:dev
```

### 生产部署
```bash
# 模拟 CI/CD 流程，使用预编译构建
cargo build --workspace --release --features observability
mkdir -p ./docker-binaries
cp target/release/berry-api ./docker-binaries/
cp target/release/berry-cli ./docker-binaries/
docker build -f Dockerfile.prebuilt -t berry-api:prod .
```

## 🔍 故障排除

### 预编译构建问题
1. **二进制文件不存在**
   ```bash
   # 检查编译是否成功
   ls -la target/release/
   ls -la ./docker-binaries/
   ```

2. **架构不匹配**
   ```bash
   # 确保目标架构正确
   cargo build --target x86_64-unknown-linux-gnu --release
   ```

3. **权限问题**
   ```bash
   # 检查文件权限
   chmod +x ./docker-binaries/berry-api
   chmod +x ./docker-binaries/berry-cli
   ```

### 传统构建问题
1. **编译失败**
   - 检查 Rust 版本兼容性
   - 确保所有依赖可用
   - 查看 Docker 构建日志

2. **内存不足**
   - 增加 Docker 内存限制
   - 使用 `--no-default-features` 减少编译负担

## 📦 GitHub Release 自动发布

### 🚀 自动化发布流程

当推送版本标签（如 `v1.0.0`）时，GitHub Actions 会自动：

1. **编译二进制文件**：使用预编译方式构建 Linux x86_64 二进制文件
2. **创建压缩包**：将 `berry-api` 和 `berry-cli` 打包为 `.tar.gz` 文件
3. **生成校验和**：创建 SHA256 校验和文件
4. **发布 Release**：自动创建 GitHub Release 并上传文件
5. **推送 Docker 镜像**：同时推送到 Docker Hub

### 📋 Release 文件说明

每个 Release 包含以下文件：

| 文件名 | 说明 |
|--------|------|
| `berry-api-{version}-linux-x86_64.tar.gz` | 包含 `berry-api` 和 `berry-cli` 的二进制文件包 |
| `berry-api-{version}-linux-x86_64.tar.gz.sha256` | SHA256 校验和文件 |

### 🧪 本地测试 Release 构建

```bash
# 测试 Release 构建流程
./scripts/test-release-build.sh v1.0.0-test

# 验证生成的文件
ls -la release-assets/
```

### 📥 下载和使用 Release

```bash
# 1. 下载最新版本
wget https://github.com/PPKunOfficial/berry-api/releases/latest/download/berry-api-v1.0.0-linux-x86_64.tar.gz

# 2. 验证校验和（可选）
wget https://github.com/PPKunOfficial/berry-api/releases/latest/download/berry-api-v1.0.0-linux-x86_64.tar.gz.sha256
sha256sum -c berry-api-v1.0.0-linux-x86_64.tar.gz.sha256

# 3. 解压并运行
tar -xzf berry-api-v1.0.0-linux-x86_64.tar.gz
chmod +x berry-api-linux-x86_64 berry-cli-linux-x86_64
./berry-api-linux-x86_64 --version
```

## 📝 注意事项

1. **功能特性**：两种构建方式都默认启用 `observability` 功能
2. **二进制文件**：预编译方式会同时构建 `berry-api` 和 `berry-cli`
3. **缓存策略**：GitHub Actions 使用 `Swatinem/rust-cache` 优化编译缓存
4. **安全性**：两种方式都使用 `gcr.io/distroless/cc-debian12` 作为运行时镜像
5. **Release 触发**：只有推送符合 `v*.*.*` 格式的标签才会触发 Release 构建
6. **权限要求**：GitHub Actions 需要 `GITHUB_TOKEN` 权限来创建 Release
