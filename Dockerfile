# =================================================================
# 传统多阶段构建 Dockerfile (备选方案)
# 在容器内编译 Rust 应用，适用于本地开发或无法预编译的场景
#
# 使用方法: docker build -f Dockerfile .
# =================================================================

# =================================================================
# Stage 1: Builder - 编译你的 Rust 应用
# 使用 slim 镜像并固定版本以保证构建的稳定性
# =================================================================
FROM rust:1.87-slim-bookworm AS builder

# 创建应用目录
WORKDIR /app

# 复制整个项目，确保所有必要的文件都存在
COPY . .

# 直接构建 berry-api 项目
RUN cargo build --bin berry-api --release

# =================================================================
# Stage 2: Runner - 运行你的应用
# 使用 Google 的 Distroless 镜像，它极小且安全，不包含 shell 和包管理器
# =================================================================
FROM gcr.io/distroless/cc-debian12

# 设置工作目录
WORKDIR /app

# 从 builder 阶段复制编译好的二进制文件
COPY --from=builder /app/target/release/berry-api /usr/local/bin/berry-api

# 暴露端口
EXPOSE 3000

# 设置环境变量
ENV RUST_LOG=info
ENV BIND_ADDRESS=0.0.0.0:3000
ENV CONFIG_PATH=/app/config.toml

# 启动命令
CMD ["/usr/local/bin/berry-api"]


