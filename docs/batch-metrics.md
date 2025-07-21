# 批量指标收集系统（Batch Metrics Collector）

本文档详细介绍 Berry API 中的批量指标收集系统的设计与实现，涵盖关键组件、工作流程及扩展方法。

---

## 设计目标

- 高效异步收集多种类型的指标事件
- 支持批量处理，减少处理开销
- 提供灵活配置，满足不同场景需求
- 易于扩展，支持多种指标后端集成

---

## 关键组件

### MetricEvent

指标事件枚举，定义了系统支持的多种指标类型：

- `HttpRequest`：HTTP 请求指标，包含方法、路径、状态码、耗时等信息
- `BackendRequest`：后端请求指标，包含提供商、模型、成功状态、延迟、错误类型等
- `HealthCheck`：健康检查指标，包含后端标识、健康状态、检查耗时
- `CacheMetric`：缓存指标，包含缓存类型和操作（命中、未命中、驱逐）
- `Counter`：自定义计数器，支持带标签的计数
- `Histogram`：自定义直方图，支持带标签的分布数据

### BatchMetricsConfig

批量指标收集器配置结构体，主要配置项：

- `batch_size`：每批处理的指标事件数量，默认100
- `flush_interval`：批量刷新时间间隔，默认5秒
- `buffer_size`：内部缓冲区大小，默认10000
- `enable_compression`：是否启用压缩，默认关闭

### BatchMetricsCollector

批量指标收集器核心结构体，负责：

- 接收指标事件（通过异步无界通道）
- 缓冲指标事件，按批量大小或时间间隔触发批量处理
- 统计指标事件总数、处理数、丢弃数及批次数
- 启动后台异步任务处理批量指标
- 提供多种指标事件记录接口（HTTP请求、后端请求、健康检查、缓存、自定义计数器和直方图）
- 提供异步接口获取统计信息

---

## 工作流程

1. 应用调用 `BatchMetricsCollector` 的记录接口，非阻塞地将指标事件发送到内部通道。
2. 后台异步任务持续接收事件，缓冲到一定数量（`batch_size`）或达到刷新时间间隔（`flush_interval`）时，触发批量处理。
3. 批量处理函数对指标事件进行分类统计，并可集成实际的指标后端（如 Prometheus、InfluxDB 等）进行上报。
4. 处理完成后更新统计信息，清空缓冲区，继续接收新事件。

---

## 使用示例

```rust
use std::time::Duration;
use std::collections::HashMap;

let config = BatchMetricsConfig {
    batch_size: 200,
    flush_interval: Duration::from_secs(10),
    buffer_size: 5000,
    enable_compression: false,
};

let collector = BatchMetricsCollector::new(config);

// 记录 HTTP 请求指标
collector.record_http_request("GET", "/api/data", 200, Duration::from_millis(150));

// 记录后端请求指标
collector.record_backend_request("openai", "gpt-4", true, Duration::from_millis(850), None);

// 记录健康检查指标
collector.record_health_check("backend-1", true, Duration::from_millis(50));

// 记录缓存指标
collector.record_cache_metric("redis", "hit");

// 记录自定义计数器
let mut labels = HashMap::new();
labels.insert("region".to_string(), "us-east".to_string());
collector.record_counter("custom_counter", labels.clone(), 42.0);

// 记录自定义直方图
collector.record_histogram("response_time", labels, 123.4);

// 异步获取统计信息
let stats = tokio::runtime::Runtime::new().unwrap().block_on(async {
    collector.get_stats().await
});
println!("{}", stats);
```

---

## 扩展与集成

- **指标后端集成**：`process_batch` 函数为批量处理入口，当前示例仅打印日志，实际可集成 Prometheus Pushgateway、InfluxDB、Elasticsearch 等。
- **压缩支持**：配置项 `enable_compression` 预留接口，未来可实现批量数据压缩传输。
- **自定义指标类型**：可扩展 `MetricEvent` 枚举，添加更多指标事件类型。
- **多实例支持**：可创建多个 `BatchMetricsCollector` 实例，分别处理不同来源或类型的指标。

---

## 统计信息结构体

`BatchMetricsStats` 提供当前收集器的统计数据，包括：

- 总事件数
- 已处理事件数
- 丢弃事件数
- 批次数量
- 最后刷新时间及距今时长

---

## 代码结构概览

- `MetricEvent`：指标事件定义
- `BatchMetricsConfig`：配置结构体
- `BatchMetricsCollector`：核心收集器，包含事件发送、后台处理、事件记录接口
- `BatchMetricsStats`：统计信息结构体及显示实现

---

## 参考

代码实现详见 `berry-api/src/observability/batch_metrics.rs`。

---