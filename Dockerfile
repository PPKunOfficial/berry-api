# Berry API Docker 镜像
# 多阶段构建优化镜像大小

# 构建阶段
FROM rust:1.75-slim as builder

# 安装构建依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# 创建应用目录
WORKDIR /app

# 先复制 Cargo 文件以便缓存依赖
COPY Cargo.toml Cargo.lock ./
COPY api/Cargo.toml ./api/

# 创建虚拟源文件来构建依赖
RUN mkdir src api/src && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub fn start_server() -> Result<(), Box<dyn std::error::Error>> { Ok(()) }" > api/src/lib.rs

# 构建依赖（这一层会被缓存）
RUN cargo build --release && \
    rm -rf src api/src target/release/deps/berry*

# 复制实际源代码
COPY src ./src
COPY api/src ./api/src

# 构建应用程序
RUN cargo build --release

# 运行时阶段
FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# 设置工作目录
WORKDIR /app

# 从构建阶段复制二进制文件
COPY --from=builder /app/target/release/berry-api /usr/local/bin/berry-api

# 暴露端口
EXPOSE 3000

# 健康检查
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

# 设置环境变量
ENV RUST_LOG=info
ENV BIND_ADDRESS=0.0.0.0:3000
ENV CONFIG_PATH=/app/config.toml

# 启动命令
CMD ["/usr/local/bin/berry-api"]
