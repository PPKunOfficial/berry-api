#[cfg(test)]
mod tests {
    use crate::auth::types::*;
    use crate::config::model::*;
    use std::collections::HashMap;

    fn create_test_user_token() -> UserToken {
        UserToken {
            name: "Test User".to_string(),
            token: "test-token-123".to_string(),
            allowed_models: vec!["gpt-4".to_string(), "gpt-3.5-turbo".to_string()],
            enabled: true,
            rate_limit: Some(RateLimit {
                requests_per_minute: 60,
                requests_per_hour: 1000,
                requests_per_day: 10000,
            }),
            tags: vec!["test".to_string(), "user".to_string()],
        }
    }

    fn create_admin_user_token() -> UserToken {
        UserToken {
            name: "Admin User".to_string(),
            token: "admin-token-456".to_string(),
            allowed_models: vec![], // 空表示允许所有模型
            enabled: true,
            rate_limit: None,
            tags: vec!["admin".to_string()],
        }
    }

    #[test]
    fn test_authenticated_user_creation() {
        let user_token = create_test_user_token();
        let auth_user = AuthenticatedUser::new("test-user".to_string(), user_token.clone());

        assert_eq!(auth_user.user_id, "test-user");
        assert_eq!(auth_user.user_token.name, "Test User");
        assert_eq!(auth_user.user_token.token, "test-token-123");
    }

    #[test]
    fn test_can_access_model_allowed() {
        let user_token = create_test_user_token();
        let auth_user = AuthenticatedUser::new("test-user".to_string(), user_token);

        assert!(auth_user.can_access_model("gpt-4"));
        assert!(auth_user.can_access_model("gpt-3.5-turbo"));
    }

    #[test]
    fn test_can_access_model_denied() {
        let user_token = create_test_user_token();
        let auth_user = AuthenticatedUser::new("test-user".to_string(), user_token);

        assert!(!auth_user.can_access_model("claude-3"));
        assert!(!auth_user.can_access_model("nonexistent-model"));
    }

    #[test]
    fn test_can_access_model_admin_user() {
        let admin_token = create_admin_user_token();
        let admin_user = AuthenticatedUser::new("admin-user".to_string(), admin_token);

        // 管理员用户可以访问任何模型
        assert!(admin_user.can_access_model("gpt-4"));
        assert!(admin_user.can_access_model("claude-3"));
        assert!(admin_user.can_access_model("any-model"));
    }

    #[test]
    fn test_get_name() {
        let user_token = create_test_user_token();
        let auth_user = AuthenticatedUser::new("test-user".to_string(), user_token);

        assert_eq!(auth_user.get_name(), "Test User");
    }

    #[test]
    fn test_get_tags() {
        let user_token = create_test_user_token();
        let auth_user = AuthenticatedUser::new("test-user".to_string(), user_token);

        let tags = auth_user.get_tags();
        assert_eq!(tags.len(), 2);
        assert!(tags.contains(&"test".to_string()));
        assert!(tags.contains(&"user".to_string()));
    }

    #[test]
    fn test_has_tag() {
        let user_token = create_test_user_token();
        let auth_user = AuthenticatedUser::new("test-user".to_string(), user_token);

        assert!(auth_user.has_tag("test"));
        assert!(auth_user.has_tag("user"));
        assert!(!auth_user.has_tag("admin"));
        assert!(!auth_user.has_tag("nonexistent"));
    }

    #[test]
    fn test_auth_error_missing_token() {
        let error = AuthError::missing_token();

        assert_eq!(error.error, "missing_authorization");
        assert_eq!(error.message, "Authorization header is missing or invalid");
        assert_eq!(error.status, 401);
    }

    #[test]
    fn test_auth_error_invalid_token() {
        let error = AuthError::invalid_token();

        assert_eq!(error.error, "invalid_token");
        assert_eq!(error.message, "The provided API key is invalid");
        assert_eq!(error.status, 401);
    }

    #[test]
    fn test_auth_error_disabled_user() {
        let error = AuthError::disabled_user();

        assert_eq!(error.error, "disabled_user");
        assert_eq!(error.message, "User account is disabled");
        assert_eq!(error.status, 403);
    }

    #[test]
    fn test_auth_error_model_access_denied() {
        let error = AuthError::model_access_denied("gpt-4");

        assert_eq!(error.error, "model_access_denied");
        assert_eq!(error.message, "Access denied for model: gpt-4");
        assert_eq!(error.status, 403);
    }

    #[test]
    fn test_auth_error_rate_limit_exceeded() {
        let error = AuthError::rate_limit_exceeded();

        assert_eq!(error.error, "rate_limit_exceeded");
        assert_eq!(error.message, "Rate limit exceeded. Please try again later");
        assert_eq!(error.status, 429);
    }

    #[test]
    fn test_billing_mode_default() {
        let default_mode = BillingMode::default();
        assert_eq!(default_mode, BillingMode::PerToken);
    }

    #[test]
    fn test_billing_mode_per_token() {
        let mode = BillingMode::PerToken;
        assert_eq!(mode, BillingMode::PerToken);
        assert_ne!(mode, BillingMode::PerRequest);
    }

    #[test]
    fn test_billing_mode_per_request() {
        let mode = BillingMode::PerRequest;
        assert_eq!(mode, BillingMode::PerRequest);
        assert_ne!(mode, BillingMode::PerToken);
    }

    #[test]
    fn test_load_balance_strategy_default() {
        let default_strategy = LoadBalanceStrategy::default();
        assert_eq!(default_strategy, LoadBalanceStrategy::WeightedRandom);
    }

    #[test]
    fn test_rate_limit_structure() {
        let rate_limit = RateLimit {
            requests_per_minute: 60,
            requests_per_hour: 1000,
            requests_per_day: 10000,
        };

        assert_eq!(rate_limit.requests_per_minute, 60);
        assert_eq!(rate_limit.requests_per_hour, 1000);
        assert_eq!(rate_limit.requests_per_day, 10000);
    }

    #[test]
    fn test_user_token_with_rate_limit() {
        let user_token = create_test_user_token();

        assert!(user_token.rate_limit.is_some());
        let rate_limit = user_token.rate_limit.unwrap();
        assert_eq!(rate_limit.requests_per_minute, 60);
        assert_eq!(rate_limit.requests_per_hour, 1000);
        assert_eq!(rate_limit.requests_per_day, 10000);
    }

    #[test]
    fn test_user_token_without_rate_limit() {
        let admin_token = create_admin_user_token();

        assert!(admin_token.rate_limit.is_none());
    }

    #[test]
    fn test_user_token_enabled_disabled() {
        let mut user_token = create_test_user_token();
        assert!(user_token.enabled);

        user_token.enabled = false;
        assert!(!user_token.enabled);
    }

    #[test]
    fn test_user_token_allowed_models_empty() {
        let admin_token = create_admin_user_token();
        assert!(admin_token.allowed_models.is_empty());
    }

    #[test]
    fn test_user_token_allowed_models_specific() {
        let user_token = create_test_user_token();
        assert!(!user_token.allowed_models.is_empty());
        assert_eq!(user_token.allowed_models.len(), 2);
        assert!(user_token.allowed_models.contains(&"gpt-4".to_string()));
        assert!(user_token
            .allowed_models
            .contains(&"gpt-3.5-turbo".to_string()));
    }

    #[test]
    fn test_user_token_tags() {
        let user_token = create_test_user_token();
        assert_eq!(user_token.tags.len(), 2);
        assert!(user_token.tags.contains(&"test".to_string()));
        assert!(user_token.tags.contains(&"user".to_string()));

        let admin_token = create_admin_user_token();
        assert_eq!(admin_token.tags.len(), 1);
        assert!(admin_token.tags.contains(&"admin".to_string()));
    }

    #[test]
    fn test_backend_structure() {
        let backend = Backend {
            provider: "test-provider".to_string(),
            model: "test-model".to_string(),
            weight: 1.5,
            priority: 2,
            enabled: true,
            tags: vec!["test".to_string(), "backend".to_string()],
            billing_mode: BillingMode::PerRequest,
        };

        assert_eq!(backend.provider, "test-provider");
        assert_eq!(backend.model, "test-model");
        assert_eq!(backend.weight, 1.5);
        assert_eq!(backend.priority, 2);
        assert!(backend.enabled);
        assert_eq!(backend.tags.len(), 2);
        assert_eq!(backend.billing_mode, BillingMode::PerRequest);
    }

    #[test]
    fn test_backend_default_values() {
        // 测试默认值是否正确应用
        let backend = Backend {
            provider: "test-provider".to_string(),
            model: "test-model".to_string(),
            weight: 1.0, // 默认权重
            priority: 1,
            enabled: true, // 默认启用
            tags: vec![],
            billing_mode: BillingMode::default(), // 默认计费模式
        };

        assert_eq!(backend.weight, 1.0);
        assert!(backend.enabled);
        assert_eq!(backend.billing_mode, BillingMode::PerToken);
    }

    #[test]
    fn test_provider_structure() {
        let mut headers = HashMap::new();
        headers.insert("X-Custom-Header".to_string(), "custom-value".to_string());

        let provider = Provider {
            name: "Test Provider".to_string(),
            base_url: "https://api.test.com".to_string(),
            api_key: "test-api-key".to_string(),
            models: vec!["model1".to_string(), "model2".to_string()],
            headers,
            enabled: true,
            timeout_seconds: 30,
            max_retries: 3,
            backend_type: crate::config::model::ProviderBackendType::OpenAI,
        };

        assert_eq!(provider.name, "Test Provider");
        assert_eq!(provider.base_url, "https://api.test.com");
        assert_eq!(provider.api_key, "test-api-key");
        assert_eq!(provider.models.len(), 2);
        assert!(provider.enabled);
        assert_eq!(provider.timeout_seconds, 30);
        assert_eq!(provider.max_retries, 3);
        assert_eq!(provider.headers.len(), 1);
        assert_eq!(
            provider.headers.get("X-Custom-Header").unwrap(),
            "custom-value"
        );
    }

    #[test]
    fn test_model_mapping_structure() {
        let backend = Backend {
            provider: "test-provider".to_string(),
            model: "test-model".to_string(),
            weight: 1.0,
            priority: 1,
            enabled: true,
            tags: vec![],
            billing_mode: BillingMode::PerToken,
        };

        let model_mapping = ModelMapping {
            name: "test-model".to_string(),
            backends: vec![backend],
            strategy: LoadBalanceStrategy::Failover,
            enabled: true,
        };

        assert_eq!(model_mapping.name, "test-model");
        assert_eq!(model_mapping.backends.len(), 1);
        assert_eq!(model_mapping.strategy, LoadBalanceStrategy::Failover);
        assert!(model_mapping.enabled);
    }

    #[test]
    fn test_global_settings_default() {
        let settings = GlobalSettings::default();

        // 测试默认值是否合理
        assert!(settings.health_check_interval_seconds > 0);
        assert!(settings.request_timeout_seconds > 0);
        assert!(settings.max_retries > 0);
        assert!(settings.circuit_breaker_failure_threshold > 0);
        assert!(settings.circuit_breaker_timeout_seconds > 0);
    }
}
