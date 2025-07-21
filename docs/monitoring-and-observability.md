# 📊 监控与可观测性

Berry API 提供基于批量指标收集系统的监控与可观测性支持，包括指标事件的高效收集、批量处理和统计信息展示。

### 🎯 核心指标

**服务健康指标**

- **服务运行状态** - 服务是否正常运行
- **总请求数** - 累计处理的请求总数
- **成功请求数** - 成功处理的请求数
- **成功率** - 成功请求占总请求的百分比
- **时间戳** - 指标更新时间

**提供商健康指标**

- **总提供商数** - 配置的提供商总数
- **健康提供商数** - 当前健康的提供商数
- **健康比例** - 健康提供商占总数的比例

**模型健康指标**

- **总模型数** - 配置的模型总数
- **健康模型数** - 当前健康的模型数
- **健康比例** - 健康模型占总数的比例
- **模型详情** - 每个模型的详细健康状态

### 📈 批量指标系统（BatchMetricsCollector）

Berry API 采用批量指标收集器 `BatchMetricsCollector` 来高效处理各种指标事件。该系统通过异步通道接收指标事件，缓冲一定数量后批量处理，支持多种指标类型，包括 HTTP 请求、后端请求、健康检查、缓存指标、自定义计数器和直方图。

#### 工作原理

- 指标事件通过非阻塞接口发送到内部异步通道。
- 收集器维护一个缓冲区，按配置的批量大小和刷新间隔异步批量处理指标。
- 批量处理逻辑可扩展，当前实现以日志记录为示例，支持集成 Prometheus、InfluxDB 等后端。
- 统计信息包括总事件数、处理事件数、丢弃事件数、批次数量及最后刷新时间。

#### 配置

`BatchMetricsConfig` 配置项包括：

- `batch_size`：每批处理的指标事件数量，默认100。
- `flush_interval`：批量刷新时间间隔，默认5秒。
- `buffer_size`：内部缓冲区大小，默认10000。
- `enable_compression`：是否启用压缩，默认关闭。

#### 使用方法

1. 创建收集器实例：

```rust
let config = BatchMetricsConfig {
    batch_size: 200,
    flush_interval: Duration::from_secs(10),
    buffer_size: 5000,
    enable_compression: false,
};
let collector = BatchMetricsCollector::new(config);
```

或使用默认配置：

```rust
let collector = BatchMetricsCollector::with_default_config();
```

2. 记录指标事件：

```rust
collector.record_http_request("GET", "/api/data", 200, Duration::from_millis(150));
collector.record_backend_request("openai", "gpt-4", true, Duration::from_millis(850), None);
collector.record_health_check("backend-1", true, Duration::from_millis(50));
collector.record_cache_metric("redis", "hit");
collector.record_counter("custom_counter", labels_map, 42.0);
collector.record_histogram("response_time", labels_map, 123.4);
```

3. 获取统计信息（异步）：

```rust
let stats = collector.get_stats().await;
println!("{}", stats);
```

### 📝 日志管理

（保持原有日志管理内容不变）

### 🔍 健康检查监控

（保持原有健康检查监控内容不变）

### 🖥️ 管理接口

（保持原有管理接口内容不变）
