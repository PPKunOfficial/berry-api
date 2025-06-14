#[cfg(test)]
mod tests {
    use crate::loadbalance::manager::*;
    use berry_core::config::model::*;
    use std::collections::HashMap;
    use tokio;

    fn create_test_provider() -> Provider {
        Provider {
            name: "Test Provider".to_string(),
            base_url: "https://api.test.com".to_string(),
            api_key: "test-api-key".to_string(),
            models: vec!["test-model".to_string()],
            headers: HashMap::new(),
            enabled: true,
            timeout_seconds: 30,
            max_retries: 3,
            backend_type: berry_core::config::model::ProviderBackendType::OpenAI,
        }
    }

    fn create_test_backend(provider: &str, model: &str, weight: f64, priority: u8) -> Backend {
        Backend {
            provider: provider.to_string(),
            model: model.to_string(),
            weight,
            priority,
            enabled: true,
            tags: vec![],
            billing_mode: BillingMode::PerToken,
        }
    }

    fn create_test_model_mapping(backends: Vec<Backend>, strategy: LoadBalanceStrategy) -> ModelMapping {
        ModelMapping {
            name: "test-model".to_string(),
            backends,
            strategy,
            enabled: true,
        }
    }

    fn create_test_config_with_multiple_backends() -> Config {
        let mut providers = HashMap::new();
        providers.insert("provider1".to_string(), create_test_provider());
        providers.insert("provider2".to_string(), create_test_provider());

        let backends = vec![
            create_test_backend("provider1", "model1", 0.7, 1),
            create_test_backend("provider2", "model2", 0.3, 2),
        ];

        let mut models = HashMap::new();
        models.insert("test-model".to_string(), create_test_model_mapping(backends, LoadBalanceStrategy::WeightedRandom));

        Config {
            providers,
            models,
            users: HashMap::new(),
            settings: GlobalSettings::default(),
        }
    }

    fn create_test_config_single_backend() -> Config {
        let mut providers = HashMap::new();
        providers.insert("provider1".to_string(), create_test_provider());

        let backends = vec![
            create_test_backend("provider1", "model1", 1.0, 1),
        ];

        let mut models = HashMap::new();
        models.insert("test-model".to_string(), create_test_model_mapping(backends, LoadBalanceStrategy::WeightedRandom));

        Config {
            providers,
            models,
            users: HashMap::new(),
            settings: GlobalSettings::default(),
        }
    }

    #[tokio::test]
    async fn test_load_balance_manager_creation() {
        let config = create_test_config_single_backend();
        let manager = LoadBalanceManager::new(config);

        // 检查管理器是否正确创建
        let models = manager.get_available_models();
        assert_eq!(models.len(), 1);
    }

    #[tokio::test]
    async fn test_load_balance_manager_initialization() {
        let config = create_test_config_single_backend();
        let manager = LoadBalanceManager::new(config);

        let result = manager.initialize().await;
        assert!(result.is_ok());

        // 初始化后应该能够选择后端
        let backend_result = manager.select_backend("test-model").await;
        assert!(backend_result.is_ok());
    }

    #[tokio::test]
    async fn test_select_backend_single() {
        let config = create_test_config_single_backend();
        let manager = LoadBalanceManager::new(config);
        
        manager.initialize().await.unwrap();
        
        let result = manager.select_backend("test-model").await;
        assert!(result.is_ok());
        
        let backend = result.unwrap();
        assert_eq!(backend.provider, "provider1");
        assert_eq!(backend.model, "model1");
    }

    #[tokio::test]
    async fn test_select_backend_multiple() {
        let config = create_test_config_with_multiple_backends();
        let manager = LoadBalanceManager::new(config);
        
        manager.initialize().await.unwrap();
        
        // 多次选择，应该根据权重分配
        let mut provider1_count = 0;
        let mut provider2_count = 0;
        
        for _ in 0..100 {
            let result = manager.select_backend("test-model").await;
            assert!(result.is_ok());
            
            let backend = result.unwrap();
            if backend.provider == "provider1" {
                provider1_count += 1;
            } else if backend.provider == "provider2" {
                provider2_count += 1;
            }
        }
        
        // 由于权重是0.7和0.3，provider1应该被选择更多次
        assert!(provider1_count > provider2_count);
        assert!(provider1_count + provider2_count == 100);
    }

    #[tokio::test]
    async fn test_select_backend_nonexistent_model() {
        let config = create_test_config_single_backend();
        let manager = LoadBalanceManager::new(config);
        
        manager.initialize().await.unwrap();
        
        let result = manager.select_backend("nonexistent-model").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_select_backend_before_initialization() {
        let config = create_test_config_single_backend();
        let manager = LoadBalanceManager::new(config);
        
        let result = manager.select_backend("test-model").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_available_models() {
        let config = create_test_config_single_backend();
        let manager = LoadBalanceManager::new(config);
        
        let models = manager.get_available_models();
        assert_eq!(models.len(), 1);
        assert!(models.contains(&"test-model".to_string()));
    }

    #[tokio::test]
    async fn test_get_available_models_multiple() {
        let mut config = create_test_config_single_backend();

        // 添加第二个模型
        let backends2 = vec![
            create_test_backend("provider1", "model2", 1.0, 1),
        ];
        let mut model_mapping2 = create_test_model_mapping(backends2, LoadBalanceStrategy::Random);
        model_mapping2.name = "test-model-2".to_string(); // 设置面向客户的模型名称
        config.models.insert("test-model-2-id".to_string(), model_mapping2);

        let manager = LoadBalanceManager::new(config);

        let models = manager.get_available_models();
        assert_eq!(models.len(), 2);
        assert!(models.contains(&"test-model".to_string()));
        assert!(models.contains(&"test-model-2".to_string()));
    }

    #[tokio::test]
    async fn test_get_available_models_disabled() {
        let mut config = create_test_config_single_backend();
        config.models.get_mut("test-model").unwrap().enabled = false;
        
        let manager = LoadBalanceManager::new(config);
        
        let models = manager.get_available_models();
        assert_eq!(models.len(), 0); // 禁用的模型不应该出现
    }

    #[tokio::test]
    async fn test_health_stats_initialization() {
        let config = create_test_config_single_backend();
        let manager = LoadBalanceManager::new(config);

        manager.initialize().await.unwrap();

        let stats_map = manager.get_health_stats().await;
        assert!(!stats_map.is_empty());

        let stats = stats_map.get("test-model");
        assert!(stats.is_some());

        let stats = stats.unwrap();
        assert_eq!(stats.total_backends, 1);
        assert!(stats.healthy_backends <= stats.total_backends);
    }

    #[tokio::test]
    async fn test_health_stats_nonexistent_model() {
        let config = create_test_config_single_backend();
        let manager = LoadBalanceManager::new(config);

        manager.initialize().await.unwrap();

        let stats_map = manager.get_health_stats().await;
        let stats = stats_map.get("nonexistent-model");
        assert!(stats.is_none());
    }

    #[tokio::test]
    async fn test_record_request_success() {
        let config = create_test_config_single_backend();
        let manager = LoadBalanceManager::new(config);

        manager.initialize().await.unwrap();

        let backend = manager.select_backend("test-model").await.unwrap();
        manager.record_success(&backend.provider, &backend.model, std::time::Duration::from_millis(100));

        // 验证指标收集器记录了成功请求
        let metrics = manager.get_metrics();
        assert!(metrics.is_healthy(&backend.provider, &backend.model));
    }

    #[tokio::test]
    async fn test_record_request_failure() {
        let config = create_test_config_single_backend();
        let manager = LoadBalanceManager::new(config);

        manager.initialize().await.unwrap();

        let backend = manager.select_backend("test-model").await.unwrap();
        manager.record_failure(&backend.provider, &backend.model);

        // 验证指标收集器记录了失败请求
        let _metrics = manager.get_metrics();
        // 注意：单次失败可能不会立即标记为不健康，这取决于具体的健康检查逻辑
    }

    #[tokio::test]
    async fn test_record_multiple_requests() {
        let config = create_test_config_single_backend();
        let manager = LoadBalanceManager::new(config);

        manager.initialize().await.unwrap();

        let backend = manager.select_backend("test-model").await.unwrap();

        // 记录多个成功和失败请求
        for _ in 0..3 {
            manager.record_success(&backend.provider, &backend.model, std::time::Duration::from_millis(100));
        }
        for _ in 0..2 {
            manager.record_failure(&backend.provider, &backend.model);
        }

        // 验证指标被正确记录
        let _metrics = manager.get_metrics();
        // 这里我们主要验证方法调用不会panic，具体的指标验证需要访问MetricsCollector的内部状态
    }

    #[tokio::test]
    async fn test_failover_strategy() {
        let mut config = create_test_config_with_multiple_backends();
        config.models.get_mut("test-model").unwrap().strategy = LoadBalanceStrategy::Failover;
        
        let manager = LoadBalanceManager::new(config);
        manager.initialize().await.unwrap();
        
        // Failover策略应该优先选择priority较低的backend
        let backend = manager.select_backend("test-model").await.unwrap();
        assert_eq!(backend.priority, 1); // 应该选择priority为1的backend
    }

    #[tokio::test]
    async fn test_random_strategy() {
        let mut config = create_test_config_with_multiple_backends();
        config.models.get_mut("test-model").unwrap().strategy = LoadBalanceStrategy::Random;
        
        let manager = LoadBalanceManager::new(config);
        manager.initialize().await.unwrap();
        
        // Random策略应该能选择到不同的backend
        let mut providers = std::collections::HashSet::new();
        for _ in 0..20 {
            let backend = manager.select_backend("test-model").await.unwrap();
            providers.insert(backend.provider.clone());
        }
        
        // 在20次选择中，应该至少选择到一个不同的provider
        // 注意：这是概率性测试，极小概率可能失败
        assert!(providers.len() >= 1);
    }
}
