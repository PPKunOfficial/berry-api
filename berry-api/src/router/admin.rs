use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::app::AppState;

/// 查询参数
#[derive(Debug, Deserialize)]
pub struct ModelWeightsQuery {
    /// 模型名称（可选，不指定则返回所有模型）
    pub model: Option<String>,
}

/// 后端权重信息
#[derive(Debug, Serialize)]
pub struct BackendWeight {
    pub provider: String,
    pub model: String,
    pub original_weight: f64,
    pub effective_weight: f64,
    pub is_healthy: bool,
    pub is_enabled: bool,
    pub priority: u8,
    pub tags: Vec<String>,
    pub billing_mode: String,
    pub failure_count: u32,
}

/// 模型权重信息
#[derive(Debug, Serialize)]
pub struct ModelWeights {
    pub model_name: String,
    pub display_name: String,
    pub strategy: String,
    pub enabled: bool,
    pub backends: Vec<BackendWeight>,
    pub total_effective_weight: f64,
}

/// 获取模型权重信息
pub async fn get_model_weights(
    State(state): State<AppState>,
    Query(query): Query<ModelWeightsQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let config = state.load_balancer.get_metrics();
    let app_config = &state.config;

    let mut result = HashMap::new();

    // 如果指定了特定模型
    if let Some(model_name) = &query.model {
        if let Some(model_mapping) = app_config.models.get(model_name) {
            let model_weights = build_model_weights(model_name, model_mapping, &config);
            result.insert(model_name.clone(), model_weights);
        } else {
            return Err((
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": {
                        "type": "model_not_found",
                        "message": format!("Model '{}' not found", model_name),
                        "code": 404
                    }
                })),
            ));
        }
    } else {
        // 返回所有模型的权重信息
        for (model_key, model_mapping) in &app_config.models {
            let model_weights = build_model_weights(model_key, model_mapping, &config);
            result.insert(model_key.clone(), model_weights);
        }
    }

    Ok(Json(json!({
        "models": result,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "total_models": result.len()
    })))
}

/// 构建模型权重信息
fn build_model_weights(
    model_key: &str,
    model_mapping: &berry_core::config::model::ModelMapping,
    metrics: &berry_loadbalance::MetricsCollector,
) -> ModelWeights {
    let mut backends = Vec::new();
    let mut total_effective_weight = 0.0;

    for backend in &model_mapping.backends {
        let backend_key = format!("{}:{}", backend.provider, backend.model);
        let effective_weight = metrics.get_effective_weight(&backend_key, backend.weight);
        let is_healthy = metrics.is_healthy(&backend.provider, &backend.model);
        let failure_count = metrics.get_failure_count(&backend.provider, &backend.model);

        if backend.enabled {
            total_effective_weight += effective_weight;
        }

        backends.push(BackendWeight {
            provider: backend.provider.clone(),
            model: backend.model.clone(),
            original_weight: backend.weight,
            effective_weight,
            is_healthy,
            is_enabled: backend.enabled,
            priority: backend.priority,
            tags: backend.tags.clone(),
            billing_mode: format!("{:?}", backend.billing_mode),
            failure_count,
        });
    }

    ModelWeights {
        model_name: model_key.to_string(),
        display_name: model_mapping.name.clone(),
        strategy: format!("{:?}", model_mapping.strategy),
        enabled: model_mapping.enabled,
        backends,
        total_effective_weight,
    }
}

/// 获取用户速率限制使用情况
pub async fn get_rate_limit_usage(
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let user_id = match query.get("user_id") {
        Some(id) => id,
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": {
                        "type": "missing_parameter",
                        "message": "Missing user_id parameter",
                        "code": 400
                    }
                })),
            ));
        }
    };

    match state.rate_limiter.get_usage(user_id).await {
        Some(usage) => Ok(Json(json!({
            "user_id": user_id,
            "usage": {
                "requests_this_minute": usage.requests_this_minute,
                "requests_this_hour": usage.requests_this_hour,
                "requests_this_day": usage.requests_this_day
            },
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))),
        None => Ok(Json(json!({
            "user_id": user_id,
            "usage": {
                "requests_this_minute": 0,
                "requests_this_hour": 0,
                "requests_this_day": 0
            },
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "note": "No usage data found for this user"
        }))),
    }
}

/// 获取后端健康状态
pub async fn get_backend_health(State(state): State<AppState>) -> Json<Value> {
    let metrics = state.load_balancer.get_metrics();
    let config = &state.config;

    let mut backend_health = HashMap::new();

    for (model_key, model_mapping) in &config.models {
        let mut model_backends = Vec::new();

        for backend in &model_mapping.backends {
            let backend_key = format!("{}:{}", backend.provider, backend.model);
            let is_healthy = metrics.is_healthy(&backend.provider, &backend.model);
            let failure_count = metrics.get_failure_count(&backend.provider, &backend.model);
            let is_in_unhealthy_list = metrics.is_in_unhealthy_list(&backend_key);

            model_backends.push(json!({
                "provider": backend.provider,
                "model": backend.model,
                "enabled": backend.enabled,
                "is_healthy": is_healthy,
                "failure_count": failure_count,
                "is_in_unhealthy_list": is_in_unhealthy_list,
                "tags": backend.tags,
                "priority": backend.priority
            }));
        }

        backend_health.insert(
            model_key,
            json!({
                "model_name": model_mapping.name,
                "enabled": model_mapping.enabled,
                "backends": model_backends
            }),
        );
    }

    Json(json!({
        "backend_health": backend_health,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// 获取系统统计信息
pub async fn get_system_stats(State(state): State<AppState>) -> Json<Value> {
    let service_health = state.load_balancer.get_service_health().await;
    let available_models = state.load_balancer.get_available_models();

    Json(json!({
        "system": {
            "is_running": service_health.is_running,
            "total_models": available_models.len(),
            "available_models": available_models
        },
        "health_summary": service_health.health_summary,
        "model_stats": service_health.model_stats,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
