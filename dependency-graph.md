# Berry API 依赖关系图

## 模块依赖关系

```mermaid
graph TD
    %% 核心依赖库
    External[外部依赖库]

    %% Berry模块
    berry-core[berry-core<br/>核心库]
    berry-loadbalance[berry-loadbalance<br/>负载均衡]
    berry-relay[berry-relay<br/>请求中继]
    berry-api[berry-api<br/>API服务器]
    berry-cli[berry-cli<br/>命令行工具]

    %% 依赖关系
    berry-core --> External

    berry-loadbalance --> berry-core
    berry-loadbalance --> External

    berry-relay --> berry-core
    berry-relay --> berry-loadbalance
    berry-relay --> External

    berry-api --> berry-core
    berry-api --> berry-loadbalance
    berry-api --> berry-relay
    berry-api --> External

    berry-cli --> berry-core
    berry-cli --> berry-loadbalance
    berry-cli --> External

    %% 样式设置
    classDef coreModule fill:#e1f5fe,stroke:#01579b,stroke-width:2px
    classDef serviceModule fill:#f3e5f5,stroke:#4a148c,stroke-width:2px
    classDef externalModule fill:#fff3e0,stroke:#e65100,stroke-width:1px

    class berry-core coreModule
    class berry-loadbalance,berry-relay,berry-api,berry-cli serviceModule
    class External externalModule
```

## 外部依赖详细分析

### berry-core 依赖

- anyhow - 错误处理
- async-trait - 异步 trait 支持
- serde/serde_json - 序列化
- thiserror - 自定义错误类型
- tokio - 异步运行时
- toml - 配置文件解析
- tracing - 日志跟踪
- chrono - 时间处理
- axum/axum-extra - Web 框架
- headers - HTTP 头处理
- reqwest - HTTP 客户端

### berry-loadbalance 依赖

- **内部依赖**: berry-core
- anyhow, async-trait, serde, thiserror, tokio, tracing, chrono, reqwest
- rand - 随机数生成
- futures - 异步流处理
- once_cell - 全局变量
- parking_lot - 并发原语

### berry-relay 依赖

- **内部依赖**: berry-core, berry-loadbalance
- anyhow, async-trait, axum, axum-extra, bytes, chrono, futures, headers, reqwest, serde, thiserror, tokio, tracing
- eventsource-stream - SSE 流处理
- tokio-stream, tokio-util - 流处理工具

### berry-api 依赖

- **内部依赖**: berry-core, berry-loadbalance, berry-relay
- anyhow, axum, axum-extra, chrono, headers, serde, serde_json, tokio, tower-http, tracing, tracing-subscriber
- include_dir - 静态文件嵌入
- mime_guess - MIME 类型猜测
- prometheus, axum-prometheus - 监控指标（可选）

### berry-cli 依赖

- **内部依赖**: berry-core, berry-loadbalance
- anyhow, clap, reqwest, serde, serde_json, tokio, tracing, tracing-subscriber

## 依赖层级结构

```mermaid
graph TD
    Level0[外部依赖]
    Level1[berry-core<br/>核心抽象]
    Level2[berry-loadbalance<br/>负载均衡]
    Level3[berry-relay<br/>请求中继]
    Level4[berry-api<br/>Web服务]
    Level4_CLI[berry-cli<br/>命令行]

    Level0 --> Level1
    Level1 --> Level2
    Level2 --> Level3
    Level3 --> Level4
    Level2 --> Level4_CLI

    style Level1 fill:#ffebee
    style Level2 fill:#e8f5e9
    style Level3 fill:#e3f2fd
    style Level4 fill:#fff3e0
    style Level4_CLI fill:#f3e5f5
```
