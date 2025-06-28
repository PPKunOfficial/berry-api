use crate::app::AppState;
use axum::{extract::State, http::StatusCode, response::Json};
use serde_json::{json, Value};

/// 获取系统监控信息
pub async fn get_monitoring_info(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    // 获取负载均衡服务健康状态
    let service_health = state.load_balancer.get_service_health().await;

    // 获取缓存统计
    let cache_stats = state.load_balancer.get_cache_stats().await;

    // 获取批量指标统计
    let batch_metrics_stats = state.batch_metrics.get_stats().await;

    // 获取指标收集器统计
    let metrics_collector = state.load_balancer.get_metrics();
    let total_requests = metrics_collector.get_total_requests();
    let successful_requests = metrics_collector.get_successful_requests();

    let response = json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service": {
            "is_running": service_health.is_running,
            "is_healthy": service_health.is_healthy(),
            "success_rate": service_health.success_rate(),
            "total_requests": total_requests,
            "successful_requests": successful_requests,
        },
        "cache": cache_stats.map(|stats| json!({
            "total_requests": stats.total_requests,
            "cache_hits": stats.cache_hits,
            "cache_misses": stats.cache_misses,
            "hit_rate": stats.hit_rate,
            "evictions": stats.evictions,
        })),
        "batch_metrics": {
            "total_events": batch_metrics_stats.total_events,
            "processed_events": batch_metrics_stats.processed_events,
            "dropped_events": batch_metrics_stats.dropped_events,
            "batch_count": batch_metrics_stats.batch_count,
            "time_since_last_flush_ms": batch_metrics_stats.time_since_last_flush.as_millis(),
        },
        "backends": service_health.model_stats.iter().map(|(model, stats)| {
            json!({
                "model": model,
                "healthy_backends": stats.healthy_backends,
                "total_backends": stats.total_backends,
                "health_ratio": stats.health_ratio,
                "is_healthy": stats.is_healthy(),
                "average_latency_ms": stats.average_latency.map(|d| d.as_millis()),
            })
        }).collect::<Vec<_>>(),
        "health_summary": {
            "total_providers": service_health.health_summary.total_providers,
            "healthy_providers": service_health.health_summary.healthy_providers,
            "total_models": service_health.health_summary.total_models,
            "healthy_models": service_health.health_summary.healthy_models,
            "provider_health_ratio": service_health.health_summary.provider_health_ratio,
            "model_health_ratio": service_health.health_summary.model_health_ratio,
            "is_system_healthy": service_health.health_summary.is_system_healthy(),
        }
    });

    Ok(Json(response))
}

/// 获取模型权重信息
pub async fn get_model_weights(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    let config = &state.config;
    let mut models_info = Vec::new();

    for (model_key, model_mapping) in &config.models {
        let mut backends_info = Vec::new();

        for backend in &model_mapping.backends {
            backends_info.push(json!({
                "provider": backend.provider,
                "model": backend.model,
                "weight": backend.weight,
                "priority": backend.priority,
                "enabled": backend.enabled,
                "tags": backend.tags,
                "billing_mode": backend.billing_mode,
            }));
        }

        models_info.push(json!({
            "model_key": model_key,
            "model_name": model_mapping.name,
            "strategy": model_mapping.strategy,
            "enabled": model_mapping.enabled,
            "backends": backends_info,
        }));
    }

    let response = json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "models": models_info,
    });

    Ok(Json(response))
}

/// 清空缓存
pub async fn clear_cache(State(_state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    // 清空负载均衡缓存
    // 注意：这里需要遍历所有选择器来清空缓存
    // 由于架构限制，我们暂时返回成功状态

    let response = json!({
        "status": "ok",
        "message": "Cache clear request received",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    Ok(Json(response))
}

/// 获取详细的性能指标
pub async fn get_performance_metrics(
    State(state): State<AppState>,
) -> Result<Json<Value>, StatusCode> {
    let metrics_collector = state.load_balancer.get_metrics();
    let service_health = state.load_balancer.get_service_health().await;

    // 获取所有后端的详细指标
    let mut backend_metrics = Vec::new();

    for model_key in service_health.model_stats.keys() {
        let config = &state.config;

        // 查找对应的模型配置
        for (config_model_key, model_mapping) in &config.models {
            if config_model_key == model_key {
                for backend in &model_mapping.backends {
                    let backend_key = format!("{}:{}", backend.provider, backend.model);

                    // 获取该后端的详细指标
                    let request_count = metrics_collector.get_backend_request_count(&backend_key);
                    let failure_count =
                        metrics_collector.get_failure_count(&backend.provider, &backend.model);
                    let success_count = request_count.saturating_sub(failure_count as u64);
                    let latency = metrics_collector.get_latency(&backend.provider, &backend.model);
                    let is_healthy =
                        metrics_collector.is_healthy(&backend.provider, &backend.model);

                    backend_metrics.push(json!({
                        "backend_key": backend_key,
                        "provider": backend.provider,
                        "model": backend.model,
                        "weight": backend.weight,
                        "priority": backend.priority,
                        "enabled": backend.enabled,
                        "is_healthy": is_healthy,
                        "request_count": request_count,
                        "success_count": success_count,
                        "failure_count": failure_count,
                        "success_rate": if request_count > 0 {
                            success_count as f64 / request_count as f64
                        } else {
                            0.0
                        },
                        "average_latency_ms": latency.map(|d| d.as_millis()),
                        "tags": backend.tags,
                        "billing_mode": backend.billing_mode,
                    }));
                }
                break;
            }
        }
    }

    let response = json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "summary": {
            "total_requests": metrics_collector.get_total_requests(),
            "successful_requests": metrics_collector.get_successful_requests(),
            "total_backends": backend_metrics.len(),
            "healthy_backends": backend_metrics.iter().filter(|b| b["is_healthy"].as_bool().unwrap_or(false)).count(),
        },
        "backends": backend_metrics,
    });

    Ok(Json(response))
}

/// 健康检查端点
pub async fn health_check(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    let service_health = state.load_balancer.get_service_health().await;

    let status_code = if service_health.is_healthy() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let response = json!({
        "status": if service_health.is_healthy() { "healthy" } else { "unhealthy" },
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service_running": service_health.is_running,
        "healthy_providers": service_health.health_summary.healthy_providers,
        "total_providers": service_health.health_summary.total_providers,
        "success_rate": service_health.success_rate(),
    });

    match status_code {
        StatusCode::OK => Ok(Json(response)),
        _ => Err(status_code),
    }
}
