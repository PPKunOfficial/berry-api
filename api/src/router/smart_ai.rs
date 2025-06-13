use crate::app::AppState;
use crate::config::model::LoadBalanceStrategy;
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

/// SmartAI 权重查询参数
#[derive(Deserialize)]
pub struct SmartAiQuery {
    /// 是否包含详细信息
    #[serde(default)]
    pub detailed: bool,
    /// 是否只显示启用的后端
    #[serde(default = "default_true")]
    pub enabled_only: bool,
}

fn default_true() -> bool {
    true
}

/// 后端权重信息
#[derive(Serialize)]
pub struct BackendWeightInfo {
    /// 提供商名称
    pub provider: String,
    /// 模型名称
    pub model: String,
    /// 原始权重
    pub original_weight: f64,
    /// 当前有效权重
    pub effective_weight: f64,
    /// 信心度
    pub confidence: f64,
    /// 是否为premium后端
    pub is_premium: bool,
    /// 是否启用
    pub enabled: bool,
    /// 标签
    pub tags: Vec<String>,
    /// 计费模式
    pub billing_mode: Option<String>,
    /// 健康状态详情（仅在detailed=true时包含）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_details: Option<BackendHealthDetails>,
}

/// 后端健康状态详情
#[derive(Serialize)]
pub struct BackendHealthDetails {
    /// 总请求数
    pub total_requests: u32,
    /// 连续成功次数
    pub consecutive_successes: u32,
    /// 连续失败次数
    pub consecutive_failures: u32,
    /// 最后请求时间
    pub last_request_time: Option<String>,
    /// 最后成功时间
    pub last_success_time: Option<String>,
    /// 最后失败时间
    pub last_failure_time: Option<String>,
    /// 错误统计
    pub error_counts: HashMap<String, u32>,
    /// 连通性状态
    pub connectivity_ok: bool,
    /// 最后连通性检查时间
    pub last_connectivity_check: Option<String>,
}

/// 模型权重信息
#[derive(Serialize)]
pub struct ModelWeightInfo {
    /// 模型名称
    pub name: String,
    /// 负载均衡策略
    pub strategy: String,
    /// 是否启用
    pub enabled: bool,
    /// 后端列表
    pub backends: Vec<BackendWeightInfo>,
    /// 统计信息
    pub stats: ModelStats,
}

/// 模型统计信息
#[derive(Serialize)]
pub struct ModelStats {
    /// 总后端数
    pub total_backends: usize,
    /// 启用的后端数
    pub enabled_backends: usize,
    /// 健康的后端数
    pub healthy_backends: usize,
    /// Premium后端数
    pub premium_backends: usize,
    /// 平均信心度
    pub average_confidence: f64,
    /// 权重分布
    pub weight_distribution: HashMap<String, f64>,
}

/// 获取所有模型的SmartAI权重信息
pub async fn get_smart_ai_weights(
    State(state): State<AppState>,
    Query(query): Query<SmartAiQuery>,
) -> impl IntoResponse {
    let config = &state.config;
    let mut models_info = Vec::new();

    for (model_key, model_mapping) in &config.models {
        // 只处理使用SmartAI策略的模型
        if model_mapping.strategy != LoadBalanceStrategy::SmartAi {
            continue;
        }

        let model_info = build_model_weight_info(
            model_key,
            model_mapping,
            &state,
            query.enabled_only,
            query.detailed,
        ).await;

        models_info.push(model_info);
    }

    // 收集可用的模型名称（用于API使用提示）
    let available_models: Vec<_> = config.models.iter()
        .filter(|(_, mapping)| mapping.strategy == LoadBalanceStrategy::SmartAi)
        .map(|(key, mapping)| json!({
            "key": key,
            "name": mapping.name,
            "enabled": mapping.enabled
        }))
        .collect();

    Json(json!({
        "models": models_info,
        "total_smart_ai_models": models_info.len(),
        "available_models": available_models,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "settings": {
            "detailed": query.detailed,
            "enabled_only": query.enabled_only
        }
    }))
}

/// 获取特定模型的SmartAI权重信息
pub async fn get_model_smart_ai_weights(
    State(state): State<AppState>,
    Path(model_name): Path<String>,
    Query(query): Query<SmartAiQuery>,
) -> impl IntoResponse {
    let config = &state.config;

    // 查找模型 - 支持通过键名或显示名称查找
    let (_found_key, model_mapping) = match find_model_by_name(&config, &model_name) {
        Some((key, mapping)) => (key, mapping),
        None => {
            return Json(json!({
                "error": "Model not found",
                "model": model_name,
                "hint": "Use model key from config (e.g., 'gpt_4o') or display name (e.g., 'gpt-4o')"
            }));
        }
    };

    // 检查是否使用SmartAI策略
    if model_mapping.strategy != LoadBalanceStrategy::SmartAi {
        return Json(json!({
            "error": "Model does not use SmartAI strategy",
            "model": model_name,
            "current_strategy": format!("{:?}", model_mapping.strategy)
        }));
    }

    let model_info = build_model_weight_info(
        &model_name,
        model_mapping,
        &state,
        query.enabled_only,
        query.detailed,
    ).await;

    Json(json!({
        "model": model_info,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// 构建模型权重信息
async fn build_model_weight_info(
    _model_key: &str,
    model_mapping: &crate::config::model::ModelMapping,
    state: &AppState,
    enabled_only: bool,
    detailed: bool,
) -> ModelWeightInfo {
    let mut backends_info = Vec::new();
    let mut total_confidence = 0.0;
    let mut healthy_count = 0;
    let mut premium_count = 0;
    let mut enabled_count = 0;
    let mut weight_distribution = HashMap::new();

    for backend in &model_mapping.backends {
        if enabled_only && !backend.enabled {
            continue;
        }

        if backend.enabled {
            enabled_count += 1;
        }

        let backend_key = format!("{}:{}", backend.provider, backend.model);
        let confidence = state.load_balancer.get_metrics().get_smart_ai_confidence(&backend_key);
        
        // 计算有效权重（使用与selector.rs相同的逻辑）
        let effective_weight = calculate_effective_weight(backend, confidence);
        
        let is_premium = backend.tags.contains(&"premium".to_string());
        if is_premium {
            premium_count += 1;
        }

        if confidence > 0.6 {
            healthy_count += 1;
        }

        total_confidence += confidence;

        // 构建健康状态详情
        let health_details = if detailed {
            build_health_details(&backend_key, state).await
        } else {
            None
        };

        let backend_info = BackendWeightInfo {
            provider: backend.provider.clone(),
            model: backend.model.clone(),
            original_weight: backend.weight,
            effective_weight,
            confidence,
            is_premium,
            enabled: backend.enabled,
            tags: backend.tags.clone(),
            billing_mode: Some(format!("{:?}", backend.billing_mode)),
            health_details,
        };

        // 记录权重分布
        let provider_key = backend.provider.clone();
        *weight_distribution.entry(provider_key).or_insert(0.0) += effective_weight;

        backends_info.push(backend_info);
    }

    let total_backends = if enabled_only {
        enabled_count
    } else {
        model_mapping.backends.len()
    };

    let average_confidence = if total_backends > 0 {
        total_confidence / total_backends as f64
    } else {
        0.0
    };

    let stats = ModelStats {
        total_backends,
        enabled_backends: enabled_count,
        healthy_backends: healthy_count,
        premium_backends: premium_count,
        average_confidence,
        weight_distribution,
    };

    ModelWeightInfo {
        name: model_mapping.name.clone(),
        strategy: format!("{:?}", model_mapping.strategy),
        enabled: model_mapping.enabled,
        backends: backends_info,
        stats,
    }
}

/// 计算有效权重（与selector.rs中的逻辑保持一致）
fn calculate_effective_weight(
    backend: &crate::config::model::Backend,
    confidence: f64,
) -> f64 {
    let base_weight = backend.weight;
    
    // 检查是否为premium后端
    let is_premium = backend.tags.contains(&"premium".to_string());
    
    // 信心度到权重的映射
    let confidence_weight = match confidence {
        c if c >= 0.8 => c,           // 高信心度：按比例
        c if c >= 0.6 => c * 0.8,     // 中等信心度：适度降权
        c if c >= 0.3 => c * 0.5,     // 低信心度：大幅降权
        _ => 0.05,                    // 极低信心度：保留恢复机会
    };
    
    // 只有非premium后端才能获得稳定性加成
    let stability_bonus = if !is_premium && confidence > 0.9 { 
        1.1  // 非premium后端稳定时给予10%加成
    } else { 
        1.0  // premium后端不给加成，凭原始权重竞争
    };
    
    base_weight * confidence_weight * stability_bonus
}

/// 构建健康状态详情
async fn build_health_details(
    backend_key: &str,
    state: &AppState,
) -> Option<BackendHealthDetails> {
    let metrics = state.load_balancer.get_metrics();

    if let Some(health) = metrics.get_smart_ai_health_details(backend_key) {
        // 转换错误类型映射
        let error_counts: HashMap<String, u32> = health.error_counts
            .into_iter()
            .map(|(error_type, count)| (format!("{:?}", error_type), count))
            .collect();

        Some(BackendHealthDetails {
            total_requests: health.total_requests,
            consecutive_successes: health.consecutive_successes,
            consecutive_failures: health.consecutive_failures,
            last_request_time: Some(health.last_request_time.elapsed().as_secs().to_string() + " seconds ago"),
            last_success_time: health.last_success_time.map(|t| t.elapsed().as_secs().to_string() + " seconds ago"),
            last_failure_time: health.last_failure_time.map(|t| t.elapsed().as_secs().to_string() + " seconds ago"),
            error_counts,
            connectivity_ok: health.connectivity_ok,
            last_connectivity_check: health.last_connectivity_check.map(|t| t.elapsed().as_secs().to_string() + " seconds ago"),
        })
    } else {
        // 如果没有SmartAI健康数据，返回基本信息
        let confidence = metrics.get_smart_ai_confidence(backend_key);

        Some(BackendHealthDetails {
            total_requests: 0,
            consecutive_successes: 0,
            consecutive_failures: 0,
            last_request_time: None,
            last_success_time: None,
            last_failure_time: None,
            error_counts: HashMap::new(),
            connectivity_ok: confidence > 0.3,
            last_connectivity_check: None,
        })
    }
}

/// 通过模型名称查找模型（支持键名和显示名称）
fn find_model_by_name<'a>(
    config: &'a crate::config::model::Config,
    model_name: &str,
) -> Option<(String, &'a crate::config::model::ModelMapping)> {
    // 首先尝试直接通过键名查找
    if let Some(mapping) = config.models.get(model_name) {
        return Some((model_name.to_string(), mapping));
    }

    // 然后尝试通过显示名称查找
    for (key, mapping) in &config.models {
        if mapping.name == model_name {
            return Some((key.clone(), mapping));
        }
    }

    None
}
