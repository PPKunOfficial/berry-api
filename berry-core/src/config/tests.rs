#[cfg(test)]
mod tests {
    use crate::config::model::*;
    use std::collections::HashMap;

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
            backend_type: ProviderBackendType::OpenAI,
        }
    }

    fn create_test_backend() -> Backend {
        Backend {
            provider: "test-provider".to_string(),
            model: "test-model".to_string(),
            weight: 1.0,
            priority: 1,
            enabled: true,
            tags: vec!["test".to_string()],
            billing_mode: BillingMode::PerToken,
        }
    }

    fn create_test_model_mapping() -> ModelMapping {
        ModelMapping {
            name: "test-model".to_string(),
            backends: vec![create_test_backend()],
            strategy: LoadBalanceStrategy::WeightedRandom,
            enabled: true,
        }
    }

    fn create_test_user_token() -> UserToken {
        UserToken {
            name: "Test User".to_string(),
            token: "test-token-123456789".to_string(), // 增加长度到16+字符
            allowed_models: vec!["test-model".to_string()],
            enabled: true,
            rate_limit: Some(RateLimit {
                requests_per_minute: 60,
                requests_per_hour: 1000,
                requests_per_day: 10000,
            }),
            tags: vec!["test".to_string()],
        }
    }

    fn create_test_config() -> Config {
        let mut providers = HashMap::new();
        providers.insert("test-provider".to_string(), create_test_provider());

        let mut models = HashMap::new();
        models.insert("test-model".to_string(), create_test_model_mapping());

        let mut users = HashMap::new();
        users.insert("test-user".to_string(), create_test_user_token());

        Config {
            providers,
            models,
            users,
            settings: GlobalSettings::default(),
        }
    }

    #[test]
    fn test_config_validation_success() {
        let config = create_test_config();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_empty_provider_name() {
        let mut config = create_test_config();
        config.providers.get_mut("test-provider").unwrap().name = "".to_string();

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty name"));
    }

    #[test]
    fn test_config_validation_empty_provider_base_url() {
        let mut config = create_test_config();
        config.providers.get_mut("test-provider").unwrap().base_url = "".to_string();

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty base_url"));
    }

    #[test]
    fn test_config_validation_empty_provider_api_key() {
        let mut config = create_test_config();
        config.providers.get_mut("test-provider").unwrap().api_key = "".to_string();

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty api_key"));
    }

    #[test]
    fn test_config_validation_no_provider_models() {
        let mut config = create_test_config();
        config.providers.get_mut("test-provider").unwrap().models = vec![];

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("no models defined"));
    }

    #[test]
    fn test_config_validation_empty_model_name() {
        let mut config = create_test_config();
        config.models.get_mut("test-model").unwrap().name = "".to_string();

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty name"));
    }

    #[test]
    fn test_config_validation_no_model_backends() {
        let mut config = create_test_config();
        config.models.get_mut("test-model").unwrap().backends = vec![];

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("no backends defined"));
    }

    #[test]
    fn test_config_validation_invalid_backend_provider() {
        let mut config = create_test_config();
        config.models.get_mut("test-model").unwrap().backends[0].provider =
            "nonexistent-provider".to_string();

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("references unknown provider"));
    }

    #[test]
    fn test_config_validation_empty_user_name() {
        let mut config = create_test_config();
        config.users.get_mut("test-user").unwrap().name = "".to_string();

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty name"));
    }

    #[test]
    fn test_config_validation_empty_user_token() {
        let mut config = create_test_config();
        config.users.get_mut("test-user").unwrap().token = "".to_string();

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty token"));
    }

    #[test]
    fn test_config_validation_invalid_user_model() {
        let mut config = create_test_config();
        config.users.get_mut("test-user").unwrap().allowed_models =
            vec!["nonexistent-model".to_string()];

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("references unknown model"));
    }

    #[test]
    fn test_get_available_backends() {
        let config = create_test_config();

        let backends = config.get_available_backends("test-model");
        assert!(backends.is_some());
        assert_eq!(backends.unwrap().len(), 1);

        let no_backends = config.get_available_backends("nonexistent-model");
        assert!(no_backends.is_none());
    }

    #[test]
    fn test_get_available_backends_disabled_provider() {
        let mut config = create_test_config();
        config.providers.get_mut("test-provider").unwrap().enabled = false;

        let backends = config.get_available_backends("test-model");
        assert!(backends.is_some());
        assert_eq!(backends.unwrap().len(), 0); // 应该过滤掉禁用的provider
    }

    #[test]
    fn test_get_available_backends_disabled_backend() {
        let mut config = create_test_config();
        config.models.get_mut("test-model").unwrap().backends[0].enabled = false;

        let backends = config.get_available_backends("test-model");
        assert!(backends.is_some());
        assert_eq!(backends.unwrap().len(), 0); // 应该过滤掉禁用的backend
    }

    #[test]
    fn test_get_provider() {
        let config = create_test_config();

        let provider = config.get_provider("test-provider");
        assert!(provider.is_some());
        assert_eq!(provider.unwrap().name, "Test Provider");

        let no_provider = config.get_provider("nonexistent-provider");
        assert!(no_provider.is_none());
    }

    #[test]
    fn test_get_model() {
        let config = create_test_config();

        let model = config.get_model("test-model");
        assert!(model.is_some());
        assert_eq!(model.unwrap().name, "test-model");

        let no_model = config.get_model("nonexistent-model");
        assert!(no_model.is_none());
    }

    #[test]
    fn test_get_available_models() {
        let config = create_test_config();

        let models = config.get_available_models();
        assert_eq!(models.len(), 1);
        assert!(models.contains(&"test-model".to_string()));
    }

    #[test]
    fn test_get_available_models_disabled() {
        let mut config = create_test_config();
        config.models.get_mut("test-model").unwrap().enabled = false;

        let models = config.get_available_models();
        assert_eq!(models.len(), 0); // 禁用的模型不应该出现在列表中
    }

    #[test]
    fn test_validate_user_token() {
        let config = create_test_config();

        let user = config.validate_user_token("test-token-123456789");
        assert!(user.is_some());
        assert_eq!(user.unwrap().name, "Test User");

        let no_user = config.validate_user_token("invalid-token");
        assert!(no_user.is_none());
    }

    #[test]
    fn test_validate_user_token_disabled_user() {
        let mut config = create_test_config();
        config.users.get_mut("test-user").unwrap().enabled = false;

        let user = config.validate_user_token("test-token-123456789");
        assert!(user.is_none()); // 禁用的用户不应该通过验证
    }

    #[test]
    fn test_user_can_access_model_allowed() {
        let config = create_test_config();
        let user = config.validate_user_token("test-token-123456789").unwrap();

        assert!(config.user_can_access_model(user, "test-model"));
    }

    #[test]
    fn test_user_can_access_model_denied() {
        let config = create_test_config();
        let user = config.validate_user_token("test-token-123456789").unwrap();

        assert!(!config.user_can_access_model(user, "nonexistent-model"));
    }

    #[test]
    fn test_user_can_access_model_admin_user() {
        let mut config = create_test_config();

        // 添加管理员用户（allowed_models为空）
        config.users.insert(
            "admin-user".to_string(),
            UserToken {
                name: "Admin User".to_string(),
                token: "admin-token-456789012".to_string(), // 增加长度到16+字符
                allowed_models: vec![],                     // 空表示允许所有模型
                enabled: true,
                rate_limit: None,
                tags: vec!["admin".to_string()],
            },
        );

        let admin_user = config.validate_user_token("admin-token-456789012").unwrap();
        assert!(config.user_can_access_model(admin_user, "test-model"));
        assert!(config.user_can_access_model(admin_user, "any-model")); // 管理员可以访问任何模型名称
    }

    #[test]
    fn test_get_user_available_models() {
        let config = create_test_config();
        let user = config.validate_user_token("test-token-123456789").unwrap();

        let models = config.get_user_available_models(user);
        assert_eq!(models.len(), 1);
        assert!(models.contains(&"test-model".to_string()));
    }

    #[test]
    fn test_get_user_available_models_admin() {
        let mut config = create_test_config();

        // 添加管理员用户
        config.users.insert(
            "admin-user".to_string(),
            UserToken {
                name: "Admin User".to_string(),
                token: "admin-token-456789012".to_string(), // 增加长度到16+字符
                allowed_models: vec![],                     // 空表示允许所有模型
                enabled: true,
                rate_limit: None,
                tags: vec!["admin".to_string()],
            },
        );

        let admin_user = config.validate_user_token("admin-token-456789012").unwrap();
        let models = config.get_user_available_models(admin_user);
        assert_eq!(models.len(), 1); // 应该返回所有可用模型
        assert!(models.contains(&"test-model".to_string()));
    }
}
