use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex};
use tokio::time::interval;
use tracing::{debug, error, info, warn};
/// 指标事件类型
#[derive(Debug, Clone)]
pub enum MetricEvent {
    /// HTTP请求指标
    HttpRequest {
        method: String,
        path: String,
        status_code: u16,
        duration_ms: f64,
        timestamp: Instant,
    },
    /// 后端请求指标
    BackendRequest {
        provider: String,
        model: String,
        success: bool,
        latency_ms: f64,
        error_type: Option<String>,
        timestamp: Instant,
    },
    /// 健康检查指标
    HealthCheck {
        backend_key: String,
        healthy: bool,
        check_duration_ms: f64,
        timestamp: Instant,
    },
    /// 缓存指标
    CacheMetric {
        cache_type: String,
        operation: String, // hit, miss, eviction
        timestamp: Instant,
    },
    /// 自定义计数器
    Counter {
        name: String,
        labels: HashMap<String, String>,
        value: f64,
        timestamp: Instant,
    },
    /// 自定义直方图
    Histogram {
        name: String,
        labels: HashMap<String, String>,
        value: f64,
        timestamp: Instant,
    },
}

/// 批量指标收集器配置
#[derive(Debug, Clone)]
pub struct BatchMetricsConfig {
    /// 批量大小
    pub batch_size: usize,
    /// 刷新间隔
    pub flush_interval: Duration,
    /// 缓冲区大小
    pub buffer_size: usize,
    /// 是否启用压缩
    pub enable_compression: bool,
}

impl Default for BatchMetricsConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            flush_interval: Duration::from_secs(5),
            buffer_size: 10000,
            enable_compression: false,
        }
    }
}

/// 批量指标收集器
pub struct BatchMetricsCollector {
    sender: mpsc::UnboundedSender<MetricEvent>,
    config: BatchMetricsConfig,
    // 统计信息
    total_events: Arc<AtomicU64>,
    processed_events: Arc<AtomicU64>,
    dropped_events: Arc<AtomicU64>,
    batch_count: Arc<AtomicU64>,
    last_flush_time: Arc<Mutex<Instant>>,
}

impl BatchMetricsCollector {
    /// 创建新的批量指标收集器
    pub fn new(config: BatchMetricsConfig) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        let collector = Self {
            sender,
            config: config.clone(),
            total_events: Arc::new(AtomicU64::new(0)),
            processed_events: Arc::new(AtomicU64::new(0)),
            dropped_events: Arc::new(AtomicU64::new(0)),
            batch_count: Arc::new(AtomicU64::new(0)),
            last_flush_time: Arc::new(Mutex::new(Instant::now())),
        };

        // 启动后台处理器
        collector.start_background_processor(receiver);

        collector
    }

    /// 创建默认配置的收集器
    pub fn with_default_config() -> Self {
        Self::new(BatchMetricsConfig::default())
    }

    /// 记录指标事件（非阻塞）
    pub fn record_event(&self, event: MetricEvent) {
        self.total_events.fetch_add(1, Ordering::Relaxed);

        if self.sender.send(event).is_err() {
            self.dropped_events.fetch_add(1, Ordering::Relaxed);
            warn!("Failed to send metric event: channel closed");
        }
    }

    /// 记录HTTP请求指标
    pub fn record_http_request(
        &self,
        method: &str,
        path: &str,
        status_code: u16,
        duration: Duration,
    ) {
        let event = MetricEvent::HttpRequest {
            method: method.to_string(),
            path: path.to_string(),
            status_code,
            duration_ms: duration.as_secs_f64() * 1000.0,
            timestamp: Instant::now(),
        };
        self.record_event(event);
    }

    /// 记录后端请求指标
    pub fn record_backend_request(
        &self,
        provider: &str,
        model: &str,
        success: bool,
        latency: Duration,
        error_type: Option<&str>,
    ) {
        let event = MetricEvent::BackendRequest {
            provider: provider.to_string(),
            model: model.to_string(),
            success,
            latency_ms: latency.as_secs_f64() * 1000.0,
            error_type: error_type.map(|s| s.to_string()),
            timestamp: Instant::now(),
        };
        self.record_event(event);
    }

    /// 记录健康检查指标
    pub fn record_health_check(&self, backend_key: &str, healthy: bool, check_duration: Duration) {
        let event = MetricEvent::HealthCheck {
            backend_key: backend_key.to_string(),
            healthy,
            check_duration_ms: check_duration.as_secs_f64() * 1000.0,
            timestamp: Instant::now(),
        };
        self.record_event(event);
    }

    /// 记录缓存指标
    pub fn record_cache_metric(&self, cache_type: &str, operation: &str) {
        let event = MetricEvent::CacheMetric {
            cache_type: cache_type.to_string(),
            operation: operation.to_string(),
            timestamp: Instant::now(),
        };
        self.record_event(event);
    }

    /// 记录自定义计数器
    pub fn record_counter(&self, name: &str, labels: HashMap<String, String>, value: f64) {
        let event = MetricEvent::Counter {
            name: name.to_string(),
            labels,
            value,
            timestamp: Instant::now(),
        };
        self.record_event(event);
    }

    /// 记录自定义直方图
    pub fn record_histogram(&self, name: &str, labels: HashMap<String, String>, value: f64) {
        let event = MetricEvent::Histogram {
            name: name.to_string(),
            labels,
            value,
            timestamp: Instant::now(),
        };
        self.record_event(event);
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> BatchMetricsStats {
        let last_flush = *self.last_flush_time.lock().await;

        BatchMetricsStats {
            total_events: self.total_events.load(Ordering::Relaxed),
            processed_events: self.processed_events.load(Ordering::Relaxed),
            dropped_events: self.dropped_events.load(Ordering::Relaxed),
            batch_count: self.batch_count.load(Ordering::Relaxed),
            last_flush_time: last_flush,
            time_since_last_flush: last_flush.elapsed(),
        }
    }

    /// 启动后台处理器
    fn start_background_processor(&self, mut receiver: mpsc::UnboundedReceiver<MetricEvent>) {
        let config = self.config.clone();
        let processed_events = self.processed_events.clone();
        let batch_count = self.batch_count.clone();
        let last_flush_time = self.last_flush_time.clone();

        tokio::spawn(async move {
            let mut buffer = Vec::with_capacity(config.batch_size);
            let mut flush_interval = interval(config.flush_interval);

            info!(
                "Started batch metrics processor with batch_size={}, flush_interval={:?}",
                config.batch_size, config.flush_interval
            );

            loop {
                tokio::select! {
                    // 接收新的指标事件
                    event = receiver.recv() => {
                        match event {
                            Some(event) => {
                                buffer.push(event);

                                // 如果缓冲区满了，立即刷新
                                if buffer.len() >= config.batch_size {
                                    Self::flush_batch(&mut buffer, &processed_events, &batch_count, &last_flush_time).await;
                                }
                            }
                            None => {
                                // 通道关闭，刷新剩余数据并退出
                                if !buffer.is_empty() {
                                    Self::flush_batch(&mut buffer, &processed_events, &batch_count, &last_flush_time).await;
                                }
                                info!("Batch metrics processor stopped");
                                break;
                            }
                        }
                    }

                    // 定时刷新
                    _ = flush_interval.tick() => {
                        if !buffer.is_empty() {
                            Self::flush_batch(&mut buffer, &processed_events, &batch_count, &last_flush_time).await;
                        }
                    }
                }
            }
        });
    }

    /// 刷新批量数据
    async fn flush_batch(
        buffer: &mut Vec<MetricEvent>,
        processed_events: &Arc<AtomicU64>,
        batch_count: &Arc<AtomicU64>,
        last_flush_time: &Arc<Mutex<Instant>>,
    ) {
        if buffer.is_empty() {
            return;
        }

        let batch_size = buffer.len();
        let start_time = Instant::now();

        // 处理批量数据
        match Self::process_batch(buffer).await {
            Ok(_) => {
                processed_events.fetch_add(batch_size as u64, Ordering::Relaxed);
                batch_count.fetch_add(1, Ordering::Relaxed);

                debug!(
                    "Processed batch of {} events in {:?}",
                    batch_size,
                    start_time.elapsed()
                );
            }
            Err(e) => {
                error!("Failed to process batch of {} events: {}", batch_size, e);
            }
        }

        // 更新最后刷新时间
        *last_flush_time.lock().await = Instant::now();

        // 清空缓冲区
        buffer.clear();
    }

    /// 处理批量数据
    async fn process_batch(
        events: &[MetricEvent],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 这里可以集成实际的指标后端，如 Prometheus、InfluxDB 等
        // 目前只是记录日志作为示例

        let mut http_requests = 0;
        let mut backend_requests = 0;
        let mut health_checks = 0;
        let mut cache_metrics = 0;
        let mut counters = 0;
        let mut histograms = 0;

        for event in events {
            match event {
                MetricEvent::HttpRequest { .. } => http_requests += 1,
                MetricEvent::BackendRequest { .. } => backend_requests += 1,
                MetricEvent::HealthCheck { .. } => health_checks += 1,
                MetricEvent::CacheMetric { .. } => cache_metrics += 1,
                MetricEvent::Counter { .. } => counters += 1,
                MetricEvent::Histogram { .. } => histograms += 1,
            }
        }

        debug!(
            "Batch metrics summary: {} HTTP requests, {} backend requests, {} health checks, {} cache metrics, {} counters, {} histograms",
            http_requests, backend_requests, health_checks, cache_metrics, counters, histograms
        );

        // TODO: 实际的指标上报逻辑
        // 例如：发送到 Prometheus pushgateway、写入 InfluxDB 等

        Ok(())
    }
}

/// 批量指标统计信息
#[derive(Debug, Clone)]
pub struct BatchMetricsStats {
    pub total_events: u64,
    pub processed_events: u64,
    pub dropped_events: u64,
    pub batch_count: u64,
    pub last_flush_time: Instant,
    pub time_since_last_flush: Duration,
}

impl std::fmt::Display for BatchMetricsStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Batch Metrics Stats: {} total, {} processed, {} dropped, {} batches, last flush: {:?} ago",
            self.total_events,
            self.processed_events,
            self.dropped_events,
            self.batch_count,
            self.time_since_last_flush
        )
    }
}
