use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;

use super::types::{AuthError, AuthenticatedUser};
use crate::config::model::Config;

/// 认证中间件
pub struct AuthMiddleware;

impl AuthMiddleware {
    /// 从请求中提取并验证API密钥
    pub async fn authenticate(
        State(config): State<Arc<Config>>,
        mut request: Request,
        next: Next,
    ) -> Result<Response, Response> {
        // 提取Authorization头
        let auth_header = request
            .headers()
            .get("authorization")
            .and_then(|h| h.to_str().ok());

        let token = match auth_header {
            Some(header) => {
                if let Some(stripped) = header.strip_prefix("Bearer ") {
                    stripped // 移除 "Bearer " 前缀
                } else {
                    return Err(create_auth_error_response(AuthError::missing_token()));
                }
            }
            None => {
                return Err(create_auth_error_response(AuthError::missing_token()));
            }
        };

        // 验证令牌
        let user_token = match config.validate_user_token(token) {
            Some(user) => user,
            None => {
                return Err(create_auth_error_response(AuthError::invalid_token()));
            }
        };

        // 检查用户是否启用
        if !user_token.enabled {
            return Err(create_auth_error_response(AuthError::disabled_user()));
        }

        // 创建认证用户信息
        let authenticated_user = AuthenticatedUser::new(
            token.to_string(), // 使用token作为user_id
            user_token.clone(),
        );

        // 将认证用户信息添加到请求扩展中
        request.extensions_mut().insert(authenticated_user);

        // 继续处理请求
        Ok(next.run(request).await)
    }

    /// 验证用户对特定模型的访问权限
    pub fn check_model_access(
        user: &AuthenticatedUser,
        model_name: &str,
        config: &Config,
    ) -> Result<(), AuthError> {
        // 检查模型是否存在且启用
        let model = config.get_model(model_name);
        if model.is_none_or(|m| !m.enabled) {
            return Err(AuthError::model_access_denied(model_name));
        }

        // 检查用户是否有权限访问该模型
        if !config.user_can_access_model(&user.user_token, model_name) {
            return Err(AuthError::model_access_denied(model_name));
        }

        Ok(())
    }
}

/// 创建认证错误响应
fn create_auth_error_response(error: AuthError) -> Response {
    let status_code = match error.status {
        401 => StatusCode::UNAUTHORIZED,
        403 => StatusCode::FORBIDDEN,
        429 => StatusCode::TOO_MANY_REQUESTS,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };

    (
        status_code,
        Json(json!({
            "error": {
                "type": error.error,
                "message": error.message,
                "code": error.status
            }
        })),
    )
        .into_response()
}

/// 从请求扩展中获取认证用户信息
pub fn get_authenticated_user(request: &Request) -> Option<&AuthenticatedUser> {
    request.extensions().get::<AuthenticatedUser>()
}

/// 增强的认证检查函数，包含安全特性
pub fn validate_request_token<'a>(
    config: &'a Config,
    token: &str,
) -> Result<&'a crate::config::model::UserToken, AuthError> {
    use tracing::{debug, warn};

    // 检查token格式
    if token.is_empty() {
        warn!("Empty token provided");
        return Err(AuthError::invalid_token());
    }

    if token.len() < 8 {
        warn!("Token too short: {} characters", token.len());
        return Err(AuthError::invalid_token());
    }

    // 检查token是否包含危险字符
    if token.contains('\n') || token.contains('\r') || token.contains('\0') {
        warn!("Token contains dangerous characters");
        return Err(AuthError::invalid_token());
    }

    match config.validate_user_token(token) {
        Some(user) if user.enabled => {
            debug!("Token validation successful for user: {}", user.name);
            Ok(user)
        }
        Some(user) => {
            warn!("Token validation failed: user '{}' is disabled", user.name);
            Err(AuthError::disabled_user())
        }
        None => {
            warn!(
                "Token validation failed: invalid token (length: {})",
                token.len()
            );
            Err(AuthError::invalid_token())
        }
    }
}

/// 增强的模型访问权限检查
pub fn validate_model_access_enhanced(
    user: &AuthenticatedUser,
    model_name: &str,
    config: &Config,
) -> Result<(), AuthError> {
    use tracing::{debug, warn};

    // 检查模型名称格式
    if model_name.is_empty() {
        warn!(
            "Empty model name provided by user: {}",
            user.user_token.name
        );
        return Err(AuthError::model_access_denied(model_name));
    }

    // 检查模型是否存在且启用（通过模型名称查找）
    let model_mapping = config
        .models
        .iter()
        .find(|(_, model)| model.name == model_name && model.enabled);

    if model_mapping.is_none() {
        warn!(
            "User '{}' attempted to access non-existent or disabled model: {}",
            user.user_token.name, model_name
        );
        return Err(AuthError::model_access_denied(model_name));
    }

    // 检查用户是否有权限访问该模型
    if !config.user_can_access_model(&user.user_token, model_name) {
        warn!(
            "User '{}' denied access to model: {} (allowed models: {:?})",
            user.user_token.name, model_name, user.user_token.allowed_models
        );
        return Err(AuthError::model_access_denied(model_name));
    }

    debug!(
        "Model access granted for user '{}' to model '{}'",
        user.user_token.name, model_name
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::{
        Backend, BillingMode, LoadBalanceStrategy, ModelMapping, RateLimit, UserToken,
    };
    use std::collections::HashMap;

    fn create_test_config() -> Config {
        let mut users = HashMap::new();
        users.insert(
            "test-user".to_string(),
            UserToken {
                name: "Test User".to_string(),
                token: "test-token-123".to_string(),
                allowed_models: vec!["gpt-4-model".to_string()], // 使用模型ID而不是模型名称
                enabled: true,
                rate_limit: Some(RateLimit {
                    requests_per_minute: 60,
                    requests_per_hour: 1000,
                    requests_per_day: 10000,
                }),
                tags: vec!["test".to_string()],
            },
        );

        users.insert(
            "admin-user".to_string(),
            UserToken {
                name: "Admin User".to_string(),
                token: "admin-token-456".to_string(),
                allowed_models: vec![], // 允许所有模型
                enabled: true,
                rate_limit: None,
                tags: vec!["admin".to_string()],
            },
        );

        // 创建测试模型配置
        let mut models = HashMap::new();
        models.insert(
            "gpt-4-model".to_string(),
            ModelMapping {
                name: "gpt-4".to_string(), // 面向客户的模型名称
                backends: vec![Backend {
                    provider: "test-provider".to_string(),
                    model: "gpt-4".to_string(),
                    weight: 1.0,
                    priority: 1,
                    enabled: true,
                    tags: vec![],
                    billing_mode: BillingMode::PerToken,
                }],
                strategy: LoadBalanceStrategy::WeightedRandom,
                enabled: true,
            },
        );

        models.insert(
            "gpt-3.5-model".to_string(),
            ModelMapping {
                name: "gpt-3.5-turbo".to_string(), // 面向客户的模型名称
                backends: vec![Backend {
                    provider: "test-provider".to_string(),
                    model: "gpt-3.5-turbo".to_string(),
                    weight: 1.0,
                    priority: 1,
                    enabled: true,
                    tags: vec![],
                    billing_mode: BillingMode::PerToken,
                }],
                strategy: LoadBalanceStrategy::WeightedRandom,
                enabled: true,
            },
        );

        Config {
            providers: HashMap::new(),
            models,
            users,
            settings: Default::default(),
        }
    }

    #[test]
    fn test_validate_user_token() {
        let config = create_test_config();

        // 测试有效令牌
        let user = config.validate_user_token("test-token-123");
        assert!(user.is_some());
        assert_eq!(user.unwrap().name, "Test User");

        // 测试无效令牌
        let user = config.validate_user_token("invalid-token");
        assert!(user.is_none());
    }

    #[test]
    fn test_user_can_access_model() {
        let config = create_test_config();

        let test_user = config.validate_user_token("test-token-123").unwrap();
        let admin_user = config.validate_user_token("admin-token-456").unwrap();

        // 测试用户只能访问允许的模型
        assert!(config.user_can_access_model(test_user, "gpt-4"));
        assert!(!config.user_can_access_model(test_user, "gpt-3.5-turbo"));

        // 管理员用户可以访问所有模型
        assert!(config.user_can_access_model(admin_user, "gpt-4"));
        assert!(config.user_can_access_model(admin_user, "gpt-3.5-turbo"));
    }

    #[test]
    fn test_validate_request_token_enhanced() {
        let config = create_test_config();

        // 测试有效令牌
        let result = validate_request_token(&config, "test-token-123");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "Test User");

        // 测试空令牌
        let result = validate_request_token(&config, "");
        assert!(result.is_err());

        // 测试过短令牌
        let result = validate_request_token(&config, "short");
        assert!(result.is_err());

        // 测试包含危险字符的令牌
        let result = validate_request_token(&config, "token\nwith\nnewlines");
        assert!(result.is_err());

        // 测试无效令牌
        let result = validate_request_token(&config, "invalid-token-xyz");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_model_access_enhanced() {
        let config = create_test_config();
        let user_token = config.validate_user_token("test-token-123").unwrap();
        let user = AuthenticatedUser::new("test-user".to_string(), user_token.clone());

        // 测试有效模型访问
        let result = validate_model_access_enhanced(&user, "gpt-4", &config);
        assert!(result.is_ok());

        // 测试空模型名
        let result = validate_model_access_enhanced(&user, "", &config);
        assert!(result.is_err());

        // 测试不存在的模型
        let result = validate_model_access_enhanced(&user, "non-existent-model", &config);
        assert!(result.is_err());

        // 测试无权限访问的模型
        let result = validate_model_access_enhanced(&user, "gpt-3.5-turbo", &config);
        assert!(result.is_err());
    }
}
