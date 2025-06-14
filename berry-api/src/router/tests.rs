#[cfg(test)]
mod tests {
    use berry_core::config::model::*;
    use std::collections::HashMap;

    fn create_test_config() -> Config {
        let mut providers = HashMap::new();
        providers.insert("test-provider".to_string(), Provider {
            name: "Test Provider".to_string(),
            base_url: "https://api.test.com".to_string(),
            api_key: "test-api-key".to_string(),
            models: vec!["test-model".to_string()],
            headers: HashMap::new(),
            enabled: true,
            timeout_seconds: 30,
            max_retries: 3,
            backend_type: berry_core::config::model::ProviderBackendType::OpenAI,
        });

        let mut models = HashMap::new();
        models.insert("test-model".to_string(), ModelMapping {
            name: "test-model".to_string(),
            backends: vec![Backend {
                provider: "test-provider".to_string(),
                model: "test-model".to_string(),
                weight: 1.0,
                priority: 1,
                enabled: true,
                tags: vec![],
                billing_mode: BillingMode::PerToken,
            }],
            strategy: LoadBalanceStrategy::WeightedRandom,
            enabled: true,
        });

        let mut users = HashMap::new();
        users.insert("test-user".to_string(), UserToken {
            name: "Test User".to_string(),
            token: "test-token-123".to_string(),
            allowed_models: vec!["test-model".to_string()],
            enabled: true,
            rate_limit: Some(RateLimit {
                requests_per_minute: 60,
                requests_per_hour: 1000,
                requests_per_day: 10000,
            }),
            tags: vec!["test".to_string()],
        });

        users.insert("admin-user".to_string(), UserToken {
            name: "Admin User".to_string(),
            token: "admin-token-456".to_string(),
            allowed_models: vec![], // 允许所有模型
            enabled: true,
            rate_limit: None,
            tags: vec!["admin".to_string()],
        });

        Config {
            providers,
            models,
            users,
            settings: GlobalSettings::default(),
        }
    }

    #[test]
    fn test_config_creation() {
        let config = create_test_config();

        // 测试providers
        assert_eq!(config.providers.len(), 1);
        assert!(config.providers.contains_key("test-provider"));

        let provider = config.providers.get("test-provider").unwrap();
        assert_eq!(provider.name, "Test Provider");
        assert_eq!(provider.base_url, "https://api.test.com");
        assert_eq!(provider.api_key, "test-api-key");
        assert!(provider.enabled);

        // 测试models
        assert_eq!(config.models.len(), 1);
        assert!(config.models.contains_key("test-model"));

        let model = config.models.get("test-model").unwrap();
        assert_eq!(model.name, "test-model");
        assert!(model.enabled);
        assert_eq!(model.backends.len(), 1);

        // 测试users
        assert_eq!(config.users.len(), 2);
        assert!(config.users.contains_key("test-user"));
        assert!(config.users.contains_key("admin-user"));

        let test_user = config.users.get("test-user").unwrap();
        assert_eq!(test_user.name, "Test User");
        assert_eq!(test_user.token, "test-token-123");
        assert!(test_user.enabled);
        assert_eq!(test_user.allowed_models.len(), 1);

        let admin_user = config.users.get("admin-user").unwrap();
        assert_eq!(admin_user.name, "Admin User");
        assert_eq!(admin_user.token, "admin-token-456");
        assert!(admin_user.enabled);
        assert!(admin_user.allowed_models.is_empty()); // 管理员允许所有模型
    }

    #[test]
    fn test_config_validation() {
        let config = create_test_config();

        // 配置应该是有效的
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_user_model_access() {
        let config = create_test_config();

        let test_user = config.users.get("test-user").unwrap();
        let admin_user = config.users.get("admin-user").unwrap();

        // 测试用户只能访问允许的模型
        assert!(config.user_can_access_model(test_user, "test-model"));
        assert!(!config.user_can_access_model(test_user, "nonexistent-model"));

        // 管理员用户可以访问任何模型
        assert!(config.user_can_access_model(admin_user, "test-model"));
        assert!(config.user_can_access_model(admin_user, "any-model"));
    }

    #[test]
    fn test_available_models() {
        let config = create_test_config();

        let models = config.get_available_models();
        assert_eq!(models.len(), 1);
        assert!(models.contains(&"test-model".to_string()));
    }

    #[test]
    fn test_user_available_models() {
        let config = create_test_config();

        let test_user = config.users.get("test-user").unwrap();
        let admin_user = config.users.get("admin-user").unwrap();

        let test_user_models = config.get_user_available_models(test_user);
        assert_eq!(test_user_models.len(), 1);
        assert!(test_user_models.contains(&"test-model".to_string()));

        let admin_user_models = config.get_user_available_models(admin_user);
        assert_eq!(admin_user_models.len(), 1); // 应该返回所有可用模型
        assert!(admin_user_models.contains(&"test-model".to_string()));
    }

    #[test]
    fn test_backend_configuration() {
        let config = create_test_config();

        let backends = config.get_available_backends("test-model");
        assert!(backends.is_some());

        let backends = backends.unwrap();
        assert_eq!(backends.len(), 1);

        let backend = backends[0];
        assert_eq!(backend.provider, "test-provider");
        assert_eq!(backend.model, "test-model");
        assert_eq!(backend.weight, 1.0);
        assert_eq!(backend.priority, 1);
        assert!(backend.enabled);
        assert_eq!(backend.billing_mode, BillingMode::PerToken);
    }

    #[test]
    fn test_provider_configuration() {
        let config = create_test_config();

        let provider = config.get_provider("test-provider");
        assert!(provider.is_some());

        let provider = provider.unwrap();
        assert_eq!(provider.name, "Test Provider");
        assert_eq!(provider.base_url, "https://api.test.com");
        assert_eq!(provider.timeout_seconds, 30);
        assert_eq!(provider.max_retries, 3);
        assert!(provider.headers.is_empty());
    }

    #[test]
    fn test_global_settings() {
        let config = create_test_config();

        let settings = &config.settings;
        // 测试默认设置是否合理
        assert!(settings.health_check_interval_seconds > 0);
        assert!(settings.request_timeout_seconds > 0);
        assert!(settings.max_retries > 0);
        assert!(settings.circuit_breaker_failure_threshold > 0);
        assert!(settings.circuit_breaker_timeout_seconds > 0);
    }
}
