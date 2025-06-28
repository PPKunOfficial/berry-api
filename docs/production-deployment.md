# 🚀 生产部署指南

### 🏭 生产环境配置

**系统要求**

```bash
# 推荐配置
CPU: 2核心以上
内存: 2GB以上
磁盘: 10GB以上
网络: 稳定的互联网连接

# 操作系统
Ubuntu 22.04 LTS (推荐)
CentOS 8+
Debian 11+
```

**环境变量配置**

```bash
# /etc/environment
RUST_LOG=info
CONFIG_PATH=/etc/berry-api/config.toml
BIND_ADDRESS=0.0.0.0:3000
MAX_CONNECTIONS=1000
```

### 🐳 Docker 生产部署

**推荐：预编译构建（性能优化）**

```bash
# CI/CD 流水线中的构建步骤
cargo build --workspace --release --features observability --target x86_64-unknown-linux-gnu
mkdir -p ./docker-binaries
cp target/x86_64-unknown-linux-gnu/release/berry-api ./docker-binaries/
cp target/x86_64-unknown-linux-gnu/release/berry-cli ./docker-binaries/
docker build -f Dockerfile.prebuilt -t berry-api:prod .
```

**备选：传统多阶段构建**

```dockerfile
# Dockerfile (已优化)
FROM rust:1.87-slim-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --workspace --release --features observability

FROM gcr.io/distroless/cc-debian12
WORKDIR /app
COPY --from=builder /app/target/release/berry-api /usr/local/bin/
COPY --from=builder /app/target/release/berry-cli /usr/local/bin/
EXPOSE 3000
CMD ["/usr/local/bin/berry-api"]
```

**Docker Compose 生产配置**

```yaml
# docker-compose.prod.yml
version: '3.8'
services:
  berry-api:
    build:
      context: .
      dockerfile: Dockerfile.prod
    ports:
      - "3000:3000"
    environment:
      - RUST_LOG=info
      - CONFIG_PATH=/app/config.toml
    volumes:
      - ./config.toml:/app/config.toml:ro
      - ./logs:/app/logs
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
    deploy:
      resources:
        limits:
          memory: 1G
          cpus: '1.0'
        reservations:
          memory: 512M
          cpus: '0.5'

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
      - ./ssl:/etc/nginx/ssl:ro
    depends_on:
      - berry-api
    restart: unless-stopped
```

### ⚖️ 负载均衡与高可用

**Nginx 配置**

```nginx
# nginx.conf
upstream berry_api {
    server berry-api-1:3000 weight=3;
    server berry-api-2:3000 weight=2;
    server berry-api-3:3000 weight=1 backup;
}

server {
    listen 80;
    listen 443 ssl http2;
    server_name api.yourdomain.com;

    # SSL配置
    ssl_certificate /etc/nginx/ssl/cert.pem;
    ssl_certificate_key /etc/nginx/ssl/key.pem;

    # 安全头
    add_header X-Frame-Options DENY;
    add_header X-Content-Type-Options nosniff;
    add_header X-XSS-Protection "1; mode=block";

    location / {
        proxy_pass http://berry_api;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # 超时配置
        proxy_connect_timeout 30s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;

        # 缓冲配置
        proxy_buffering on;
        proxy_buffer_size 4k;
        proxy_buffers 8 4k;
    }

    # 健康检查
    location /health {
        proxy_pass http://berry_api/health;
        access_log off;
    }
}
```

### 🔒 安全最佳实践

**1. API密钥管理**

```bash
# 使用环境变量或密钥管理服务
export OPENAI_API_KEY=$(vault kv get -field=api_key secret/openai)

# 定期轮换密钥
./scripts/rotate-api-keys.sh

# 密钥强度检查
python3 -c "
import secrets
import string
# 生成强密钥
key = ''.join(secrets.choice(string.ascii_letters + string.digits) for _ in range(32))
print(f'Strong API key: berry-{key}')
"
```

**2. 网络安全**

```bash
# 防火墙配置
ufw allow 22/tcp
ufw allow 80/tcp
ufw allow 443/tcp
ufw deny 3000/tcp  # 只允许内部访问
ufw enable

# 限制访问源
iptables -A INPUT -p tcp --dport 3000 -s 10.0.0.0/8 -j ACCEPT
iptables -A INPUT -p tcp --dport 3000 -j DROP
```

**3. 日志安全**

```toml
# config.toml - 生产配置
[settings]
# 不记录敏感信息
log_request_body = false
log_response_body = false
mask_api_keys = true
```

### 📊 监控与告警

**Prometheus + Grafana 部署**

```yaml
# monitoring/docker-compose.yml
version: '3.8'
services:
  prometheus:
    image: prom/prometheus
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus_data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--web.console.libraries=/etc/prometheus/console_libraries'
      - '--web.console.templates=/etc/prometheus/consoles'

  grafana:
    image: grafana/grafana
    ports:
      - "3001:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin123
    volumes:
      - grafana_data:/var/lib/grafana
      - ./grafana/dashboards:/etc/grafana/provisioning/dashboards
      - ./grafana/datasources:/etc/grafana/provisioning/datasources

volumes:
  prometheus_data:
  grafana_data:
```

### 🔄 CI/CD 流水线

**GitHub Actions 配置**

```yaml
# .github/workflows/deploy.yml
name: Deploy to Production

on:
  push:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo test --all-features

  build:
    needs: test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Build Docker image
        run: |
          docker build -t berry-api:${{ github.sha }} .
          docker tag berry-api:${{ github.sha }} berry-api:latest

      - name: Push to registry
        run: |
          echo ${{ secrets.DOCKER_PASSWORD }} | docker login -u ${{ secrets.DOCKER_USERNAME }} --password-stdin
          docker push berry-api:${{ github.sha }}
          docker push berry-api:latest

  deploy:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - name: Deploy to production
        run: |
          ssh ${{ secrets.PROD_SERVER }} "
            docker pull berry-api:latest
            docker-compose -f docker-compose.prod.yml up -d --no-deps berry-api
          "
```
