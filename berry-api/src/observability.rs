//! Observability module for Berry API
//! 
//! This module provides Prometheus metrics and monitoring capabilities.
//! It's only available when the 'observability' feature is enabled.

#[cfg(feature = "observability")]
pub mod prometheus_metrics {
    use axum::{
        extract::State,
        response::IntoResponse,
        http::StatusCode,
    };
    use ::prometheus::{
        CounterVec, HistogramVec, GaugeVec, Registry, TextEncoder,
        HistogramOpts, Opts,
    };
    use std::sync::Arc;
    use crate::app::AppState;

    /// Prometheus metrics collector
    #[derive(Clone)]
    pub struct PrometheusMetrics {
        pub registry: Arc<Registry>,
        pub http_requests_total: CounterVec,
        pub http_request_duration_seconds: HistogramVec,
        pub http_requests_in_flight: GaugeVec,
        pub backend_health_status: GaugeVec,
        pub backend_request_count_total: CounterVec,
        pub backend_error_count_total: CounterVec,
        pub backend_latency_seconds: HistogramVec,
    }

    impl PrometheusMetrics {
        pub fn new() -> Result<Self, ::prometheus::Error> {
            let registry = Arc::new(Registry::new());

            // HTTP metrics
            let http_requests_total = CounterVec::new(
                Opts::new("http_requests_total", "Total number of HTTP requests")
                    .namespace("berry_api"),
                &["method", "endpoint", "status"]
            )?;

            let http_request_duration_seconds = HistogramVec::new(
                HistogramOpts::new("http_request_duration_seconds", "HTTP request duration in seconds")
                    .namespace("berry_api"),
                &["method", "endpoint"]
            )?;

            let http_requests_in_flight = GaugeVec::new(
                Opts::new("http_requests_in_flight", "Number of HTTP requests currently being processed")
                    .namespace("berry_api"),
                &["method", "endpoint"]
            )?;

            // Backend metrics
            let backend_health_status = GaugeVec::new(
                Opts::new("backend_health_status", "Health status of backends (1 = healthy, 0 = unhealthy)")
                    .namespace("berry_api"),
                &["backend"]
            )?;

            let backend_request_count_total = CounterVec::new(
                Opts::new("backend_request_count_total", "Total number of requests sent to backends")
                    .namespace("berry_api"),
                &["backend"]
            )?;

            let backend_error_count_total = CounterVec::new(
                Opts::new("backend_error_count_total", "Total number of errors from backends")
                    .namespace("berry_api"),
                &["backend"]
            )?;

            let backend_latency_seconds = HistogramVec::new(
                HistogramOpts::new("backend_latency_seconds", "Backend response latency in seconds")
                    .namespace("berry_api"),
                &["backend"]
            )?;

            // Register all metrics
            registry.register(Box::new(http_requests_total.clone()))?;
            registry.register(Box::new(http_request_duration_seconds.clone()))?;
            registry.register(Box::new(http_requests_in_flight.clone()))?;
            registry.register(Box::new(backend_health_status.clone()))?;
            registry.register(Box::new(backend_request_count_total.clone()))?;
            registry.register(Box::new(backend_error_count_total.clone()))?;
            registry.register(Box::new(backend_latency_seconds.clone()))?;

            Ok(Self {
                registry,
                http_requests_total,
                http_request_duration_seconds,
                http_requests_in_flight,
                backend_health_status,
                backend_request_count_total,
                backend_error_count_total,
                backend_latency_seconds,
            })
        }

        /// Update backend health metrics
        pub fn update_backend_health(&self, backend: &str, is_healthy: bool) {
            self.backend_health_status
                .with_label_values(&[backend])
                .set(if is_healthy { 1.0 } else { 0.0 });
        }

        /// Record backend request
        pub fn record_backend_request(&self, backend: &str) {
            self.backend_request_count_total
                .with_label_values(&[backend])
                .inc();
        }

        /// Record backend error
        pub fn record_backend_error(&self, backend: &str) {
            self.backend_error_count_total
                .with_label_values(&[backend])
                .inc();
        }

        /// Record backend latency
        pub fn record_backend_latency(&self, backend: &str, latency_seconds: f64) {
            self.backend_latency_seconds
                .with_label_values(&[backend])
                .observe(latency_seconds);
        }

        /// Update metrics from load balancer state
        pub async fn update_from_load_balancer(&self, state: &AppState) {
            let health = state.load_balancer.get_service_health().await;
            let metrics = state.load_balancer.get_metrics();

            // Update backend health status
            for (backend_key, stats) in &health.model_stats {
                self.update_backend_health(backend_key, stats.is_healthy());

                // Update latency if available
                if let Some(latency) = metrics.get_latency_by_key(backend_key) {
                    self.record_backend_latency(backend_key, latency.as_secs_f64());
                }
            }

            // Update request counts
            let request_counts = metrics.get_all_request_counts();
            for (_backend_key, _count) in request_counts {
                // Note: Prometheus counters are cumulative, so we don't set absolute values
                // The actual incrementing should happen in the request handlers
            }
        }

        /// Initialize metrics with default values to ensure they appear in Prometheus
        pub async fn initialize_metrics(&self, state: &AppState) {
            // Initialize backend health metrics for all configured backends
            let config = &state.config;
            for (_model_key, model_mapping) in &config.models {
                for backend in &model_mapping.backends {
                    let backend_key = format!("{}:{}", backend.provider, backend.model);

                    // Initialize health status (will be updated by health checker)
                    self.backend_health_status
                        .with_label_values(&[&backend_key])
                        .set(0.0); // Start as unhealthy until first check

                    // Initialize counters with 0 to make them visible
                    self.backend_request_count_total
                        .with_label_values(&[&backend_key])
                        .inc_by(0.0);

                    self.backend_error_count_total
                        .with_label_values(&[&backend_key])
                        .inc_by(0.0);

                    // Initialize histogram with a sample to make it visible
                    self.backend_latency_seconds
                        .with_label_values(&[&backend_key])
                        .observe(0.0);
                }
            }

            // Initialize HTTP metrics
            let common_endpoints = ["/v1/chat/completions", "/health", "/metrics"];
            let methods = ["GET", "POST", "PUT", "DELETE"];
            let status_codes = ["200", "400", "401", "403", "404", "500", "502", "503"];

            for endpoint in &common_endpoints {
                for method in &methods {
                    for status in &status_codes {
                        // Initialize HTTP request counters
                        self.http_requests_total
                            .with_label_values(&[method, endpoint, status])
                            .inc_by(0.0);
                    }

                    // Initialize HTTP duration histograms
                    self.http_request_duration_seconds
                        .with_label_values(&[method, endpoint])
                        .observe(0.0);

                    // Initialize in-flight gauges
                    self.http_requests_in_flight
                        .with_label_values(&[method, endpoint])
                        .set(0.0);
                }
            }

            tracing::info!("Prometheus metrics initialized with default values");
        }

        /// Start background metrics updater
        pub fn start_background_updater(&self, state: AppState) {
            let metrics = self.clone();

            tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));

                loop {
                    interval.tick().await;

                    // Update metrics from load balancer
                    metrics.update_from_load_balancer(&state).await;

                    // Update cache metrics if available
                    if let Some(cache_stats) = state.load_balancer.get_cache_stats().await {
                        // Add cache-specific metrics here
                        tracing::debug!("Cache stats: {}", cache_stats);
                    }
                }
            });
        }
    }

    /// Prometheus metrics endpoint handler
    pub async fn prometheus_metrics_handler(
        State(state): State<AppState>,
    ) -> impl IntoResponse {
        if let Some(ref metrics) = state.prometheus_metrics {
            // Update metrics from current state
            metrics.update_from_load_balancer(&state).await;

            // Encode metrics
            let encoder = TextEncoder::new();
            let metric_families = metrics.registry.gather();
            
            match encoder.encode_to_string(&metric_families) {
                Ok(output) => (
                    StatusCode::OK,
                    [("content-type", "text/plain; version=0.0.4")],
                    output
                ).into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to encode metrics: {}", e)
                ).into_response(),
            }
        } else {
            (
                StatusCode::NOT_FOUND,
                "Observability feature not enabled"
            ).into_response()
        }
    }
}

#[cfg(not(feature = "observability"))]
pub mod prometheus_metrics {
    use axum::{
        extract::State,
        response::IntoResponse,
        http::StatusCode,
    };
    use crate::app::AppState;

    /// Placeholder for when observability is disabled
    pub async fn prometheus_metrics_handler(
        _state: State<AppState>,
    ) -> impl IntoResponse {
        (
            StatusCode::NOT_FOUND,
            "Observability feature not enabled. Compile with --features observability to enable Prometheus metrics."
        )
    }
}

pub mod batch_metrics;
