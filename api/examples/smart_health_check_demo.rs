use berry_api_api::config::model::{Config, Provider, ModelMapping, Backend, LoadBalanceStrategy, GlobalSettings, BillingMode};
use berry_api_api::loadbalance::LoadBalanceService;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

fn create_smart_demo_config() -> Config {
    let mut providers = HashMap::new();
    
    // 按token计费的provider（执行主动健康检查）
    providers.insert("per_token_provider".to_string(), Provider {
        name: "Per-Token Provider (Active Health Check)".to_string(),
        base_url: "https://httpbin.org".to_string(),
        api_key: "per-token-key".to_string(),
        models: vec!["token-model".to_string()],
        headers: HashMap::new(),
        enabled: true,
        timeout_seconds: 10,
        max_retries: 2,
        billing_mode: BillingMode::PerToken,
    });

    // 按请求计费的provider（跳过主动检查，使用被动验证）
    providers.insert("per_request_provider".to_string(), Provider {
        name: "Per-Request Provider (Passive Validation)".to_string(),
        base_url: "https://httpbin.org".to_string(),
        api_key: "per-request-key".to_string(),
        models: vec!["request-model".to_string()],
        headers: HashMap::new(),
        enabled: true,
        timeout_seconds: 10,
        max_retries: 2,
        billing_mode: BillingMode::PerRequest,
    });

    // 另一个按请求计费的provider
    providers.insert("per_request_backup".to_string(), Provider {
        name: "Per-Request Backup Provider".to_string(),
        base_url: "https://httpbin.org".to_string(),
        api_key: "backup-key".to_string(),
        models: vec!["backup-model".to_string()],
        headers: HashMap::new(),
        enabled: true,
        timeout_seconds: 10,
        max_retries: 2,
        billing_mode: BillingMode::PerRequest,
    });

    let mut models = HashMap::new();
    models.insert("smart-model".to_string(), ModelMapping {
        name: "smart-model".to_string(),
        backends: vec![
            Backend {
                provider: "per_token_provider".to_string(),
                model: "token-model".to_string(),
                weight: 0.5,  // 50%权重
                priority: 1,
                enabled: true,
                tags: vec!["per-token".to_string()],
            },
            Backend {
                provider: "per_request_provider".to_string(),
                model: "request-model".to_string(),
                weight: 0.3,  // 30%权重，不健康时降至10%
                priority: 2,
                enabled: true,
                tags: vec!["per-request".to_string()],
            },
            Backend {
                provider: "per_request_backup".to_string(),
                model: "backup-model".to_string(),
                weight: 0.2,  // 20%权重，不健康时降至10%
                priority: 3,
                enabled: true,
                tags: vec!["per-request".to_string(), "backup".to_string()],
            },
        ],
        strategy: LoadBalanceStrategy::SmartWeightedFailover,
        enabled: true,
    });

    Config {
        providers,
        models,
        users: HashMap::new(),
        settings: GlobalSettings {
            health_check_interval_seconds: 15,
            request_timeout_seconds: 10,
            max_retries: 2,
            circuit_breaker_failure_threshold: 3,
            circuit_breaker_timeout_seconds: 30,
            recovery_check_interval_seconds: 30,
            max_internal_retries: 2,
            health_check_timeout_seconds: 10,
        },
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    info!("🚀 启动智能健康检查演示");
    info!("本演示展示按计费模式区分的健康检查机制：");
    info!("  - 按token计费：执行主动健康检查（chat请求）");
    info!("  - 按请求计费：跳过主动检查，使用被动验证和权重恢复");

    let config = create_smart_demo_config();
    let service = LoadBalanceService::new(config)?;

    info!("📋 配置加载完成：");
    info!("  - per_token_provider: 按token计费，执行主动健康检查");
    info!("  - per_request_provider: 按请求计费，使用被动验证");
    info!("  - per_request_backup: 按请求计费，备用provider");

    // 启动服务
    info!("🔄 启动负载均衡服务...");
    service.start().await?;

    // 等待初始健康检查完成
    info!("⏳ 等待初始健康检查完成...");
    sleep(Duration::from_secs(3)).await;

    let metrics = service.get_metrics();

    info!("📊 初始健康检查结果：");
    info!("  - per_token_provider:token-model = {}", 
          metrics.is_healthy("per_token_provider", "token-model"));
    info!("  - per_request_provider:request-model = {}", 
          metrics.is_healthy("per_request_provider", "request-model"));
    info!("  - per_request_backup:backup-model = {}", 
          metrics.is_healthy("per_request_backup", "backup-model"));

    // 演示1: 模拟按请求计费provider失败
    info!("\n=== 演示1: 按请求计费provider失败 ===");
    info!("🔥 模拟per_request_provider失败...");
    metrics.record_failure("per_request_provider:request-model");

    info!("📊 失败后状态：");
    info!("  - per_request_provider:request-model = {}", 
          metrics.is_healthy("per_request_provider", "request-model"));
    
    // 检查权重
    let effective_weight = metrics.get_effective_weight("per_request_provider:request-model", 0.3);
    info!("  - 有效权重: {:.3} (原始权重: 0.3)", effective_weight);

    // 演示2: 被动验证和权重恢复
    info!("\n=== 演示2: 被动验证和权重恢复 ===");
    info!("💬 模拟成功请求（被动验证）...");
    
    // 第一次成功 - 应该进入30%权重阶段
    metrics.record_passive_success("per_request_provider:request-model", 0.3);
    let weight_after_1st = metrics.get_effective_weight("per_request_provider:request-model", 0.3);
    info!("  - 第1次成功后权重: {:.3}", weight_after_1st);

    // 第二次成功 - 仍在30%权重阶段
    metrics.record_passive_success("per_request_provider:request-model", 0.3);
    let weight_after_2nd = metrics.get_effective_weight("per_request_provider:request-model", 0.3);
    info!("  - 第2次成功后权重: {:.3}", weight_after_2nd);

    // 第三次成功 - 应该进入50%权重阶段
    metrics.record_passive_success("per_request_provider:request-model", 0.3);
    let weight_after_3rd = metrics.get_effective_weight("per_request_provider:request-model", 0.3);
    info!("  - 第3次成功后权重: {:.3}", weight_after_3rd);

    // 第四次成功 - 仍在50%权重阶段
    metrics.record_passive_success("per_request_provider:request-model", 0.3);
    let weight_after_4th = metrics.get_effective_weight("per_request_provider:request-model", 0.3);
    info!("  - 第4次成功后权重: {:.3}", weight_after_4th);

    // 第五次成功 - 应该完全恢复到100%权重
    metrics.record_passive_success("per_request_provider:request-model", 0.3);
    let weight_after_5th = metrics.get_effective_weight("per_request_provider:request-model", 0.3);
    info!("  - 第5次成功后权重: {:.3}", weight_after_5th);
    info!("  - 健康状态: {}", metrics.is_healthy("per_request_provider", "request-model"));

    // 演示3: 智能权重故障转移
    info!("\n=== 演示3: 智能权重故障转移 ===");
    info!("🎯 测试智能backend选择...");
    
    for i in 1..=5 {
        match service.select_backend("smart-model").await {
            Ok(selected) => {
                let backend_key = format!("{}:{}", selected.backend.provider, selected.backend.model);
                let effective_weight = metrics.get_effective_weight(&backend_key, selected.backend.weight);
                info!("  选择 #{}: {} (权重: {:.3})", 
                      i, backend_key, effective_weight);
            }
            Err(e) => {
                warn!("  选择 #{} 失败: {}", i, e);
            }
        }
    }

    // 演示4: 健康检查区分
    info!("\n=== 演示4: 健康检查区分 ===");
    info!("🔍 触发健康检查...");
    service.trigger_health_check().await?;
    sleep(Duration::from_secs(2)).await;

    info!("📝 健康检查说明：");
    info!("  - per_token_provider: 执行了主动API检查");
    info!("  - per_request_provider: 跳过了主动检查（依赖被动验证）");
    info!("  - per_request_backup: 跳过了主动检查（依赖被动验证）");

    // 获取最终状态
    info!("\n=== 最终状态 ===");
    let health = service.get_service_health().await;
    info!("🏥 服务健康状态:");
    info!("  - 运行状态: {}", health.is_running);
    info!("  - 健康providers: {}/{}", health.health_summary.healthy_providers, health.health_summary.total_providers);
    info!("  - 健康models: {}/{}", health.health_summary.healthy_models, health.health_summary.total_models);
    info!("  - 系统健康: {}", if health.is_healthy() { "✅" } else { "❌" });

    // 停止服务
    info!("\n🛑 停止服务...");
    service.stop().await;

    info!("\n🎉 演示完成！");
    info!("📋 总结：");
    info!("  1. ✅ 按计费模式区分健康检查策略");
    info!("  2. ✅ 按请求计费provider的被动验证机制");
    info!("  3. ✅ 权重恢复机制 (10% → 30% → 50% → 100%)");
    info!("  4. ✅ 智能权重故障转移策略");
    info!("  5. ✅ 混合计费模式的负载均衡");

    Ok(())
}
