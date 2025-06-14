#[cfg(test)]
mod tests {
    use crate::relay::client::types::*;
    use crate::config::model::Provider;
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
        }
    }

    #[test]
    fn test_client_error_creation() {
        let error = ClientError::HeaderParseError("header parse failed".to_string());
        assert!(matches!(error, ClientError::HeaderParseError(_)));

        let error = ClientError::UpstreamError {
            status: 500,
            body: "Internal Server Error".to_string()
        };
        assert!(matches!(error, ClientError::UpstreamError { .. }));
    }

    #[test]
    fn test_client_error_display() {
        let error = ClientError::HeaderParseError("header parse failed".to_string());
        let display_str = format!("{}", error);
        assert!(display_str.contains("请求头解析失败"));
        assert!(display_str.contains("header parse failed"));

        let error = ClientError::UpstreamError {
            status: 500,
            body: "Internal Server Error".to_string()
        };
        let display_str = format!("{}", error);
        assert!(display_str.contains("上游API返回错误"));
        assert!(display_str.contains("500"));
    }

    #[test]
    fn test_client_error_debug() {
        let error = ClientError::HeaderParseError("test-header".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("HeaderParseError"));
        assert!(debug_str.contains("test-header"));
    }

    #[test]
    fn test_client_error_from_reqwest_error() {
        // 测试从reqwest错误转换
        // 注意：这里我们无法直接创建reqwest::Error，所以只能测试转换逻辑的存在
        // 实际的转换测试需要在集成测试中进行
    }

    #[test]
    fn test_client_error_from_serde_error() {
        // 创建一个serde JSON错误
        let json_str = r#"{"invalid": json}"#;
        let parse_result: Result<serde_json::Value, serde_json::Error> = serde_json::from_str(json_str);

        if let Err(serde_error) = parse_result {
            let client_error = ClientError::from(serde_error);
            assert!(matches!(client_error, ClientError::JsonParseError(_)));
        }
    }

    #[test]
    fn test_client_response_creation() {
        let response = ClientResponse::new(200, "success".to_string());
        assert_eq!(response.status, 200);
        assert_eq!(response.body, "success");
        assert!(response.is_success);

        let response = ClientResponse::new(404, "not found".to_string());
        assert_eq!(response.status, 404);
        assert_eq!(response.body, "not found");
        assert!(!response.is_success);

        let response = ClientResponse::new(500, "server error".to_string());
        assert_eq!(response.status, 500);
        assert_eq!(response.body, "server error");
        assert!(!response.is_success);
    }

    #[test]
    fn test_client_error_chain() {
        // 测试错误链
        let error = ClientError::HeaderParseError("original error".to_string());
        let error_string = error.to_string();
        assert!(error_string.contains("original error"));
    }

    #[test]
    fn test_provider_configuration() {
        let provider = create_test_provider();
        
        assert_eq!(provider.name, "Test Provider");
        assert_eq!(provider.base_url, "https://api.test.com");
        assert_eq!(provider.api_key, "test-api-key");
        assert_eq!(provider.models.len(), 1);
        assert!(provider.models.contains(&"test-model".to_string()));
        assert!(provider.enabled);
        assert_eq!(provider.timeout_seconds, 30);
        assert_eq!(provider.max_retries, 3);
        assert!(provider.headers.is_empty());
    }

    #[test]
    fn test_provider_with_custom_headers() {
        let mut headers = HashMap::new();
        headers.insert("X-Custom-Header".to_string(), "custom-value".to_string());
        headers.insert("User-Agent".to_string(), "test-agent".to_string());
        
        let provider = Provider {
            name: "Test Provider".to_string(),
            base_url: "https://api.test.com".to_string(),
            api_key: "test-api-key".to_string(),
            models: vec!["test-model".to_string()],
            headers: headers.clone(),
            enabled: true,
            timeout_seconds: 30,
            max_retries: 3,
        };
        
        assert_eq!(provider.headers.len(), 2);
        assert_eq!(provider.headers.get("X-Custom-Header").unwrap(), "custom-value");
        assert_eq!(provider.headers.get("User-Agent").unwrap(), "test-agent");
    }

    #[test]
    fn test_provider_disabled() {
        let mut provider = create_test_provider();
        provider.enabled = false;
        
        assert!(!provider.enabled);
    }

    #[test]
    fn test_provider_multiple_models() {
        let mut provider = create_test_provider();
        provider.models = vec![
            "model1".to_string(),
            "model2".to_string(),
            "model3".to_string(),
        ];
        
        assert_eq!(provider.models.len(), 3);
        assert!(provider.models.contains(&"model1".to_string()));
        assert!(provider.models.contains(&"model2".to_string()));
        assert!(provider.models.contains(&"model3".to_string()));
    }

    #[test]
    fn test_provider_timeout_configuration() {
        let mut provider = create_test_provider();
        provider.timeout_seconds = 60;
        provider.max_retries = 5;
        
        assert_eq!(provider.timeout_seconds, 60);
        assert_eq!(provider.max_retries, 5);
    }

    #[test]
    fn test_provider_base_url_variations() {
        let mut provider = create_test_provider();
        
        // 测试不同的base_url格式
        provider.base_url = "https://api.openai.com/v1".to_string();
        assert!(provider.base_url.starts_with("https://"));
        
        provider.base_url = "http://localhost:8080".to_string();
        assert!(provider.base_url.starts_with("http://"));
        
        provider.base_url = "https://api.anthropic.com".to_string();
        assert!(!provider.base_url.ends_with("/"));
    }

    #[test]
    fn test_provider_api_key_security() {
        let provider = create_test_provider();
        
        // 确保API密钥不为空
        assert!(!provider.api_key.is_empty());
        
        // 在实际应用中，API密钥应该从环境变量或安全存储中读取
        // 这里只是测试结构体的基本功能
    }

    #[test]
    fn test_provider_clone() {
        let provider = create_test_provider();
        let cloned_provider = provider.clone();
        
        assert_eq!(provider.name, cloned_provider.name);
        assert_eq!(provider.base_url, cloned_provider.base_url);
        assert_eq!(provider.api_key, cloned_provider.api_key);
        assert_eq!(provider.models, cloned_provider.models);
        assert_eq!(provider.enabled, cloned_provider.enabled);
        assert_eq!(provider.timeout_seconds, cloned_provider.timeout_seconds);
        assert_eq!(provider.max_retries, cloned_provider.max_retries);
    }

    #[test]
    fn test_provider_debug_format() {
        let provider = create_test_provider();
        let debug_str = format!("{:?}", provider);
        
        // 确保debug输出包含关键信息
        assert!(debug_str.contains("Test Provider"));
        assert!(debug_str.contains("https://api.test.com"));
        // 注意：在实际应用中，可能需要避免在debug输出中显示API密钥
    }

    #[test]
    fn test_error_categorization() {
        // 测试不同类型的错误是否被正确分类
        let header_error = ClientError::HeaderParseError("invalid header".to_string());
        assert!(matches!(header_error, ClientError::HeaderParseError(_)));

        let upstream_error = ClientError::UpstreamError {
            status: 429,
            body: "Rate limit exceeded".to_string()
        };
        assert!(matches!(upstream_error, ClientError::UpstreamError { .. }));
    }

    #[test]
    fn test_error_message_content() {
        let error = ClientError::UpstreamError {
            status: 500,
            body: "Internal Server Error".to_string()
        };
        let message = error.to_string();
        assert!(message.contains("500"));

        let error = ClientError::HeaderParseError("Authorization header missing".to_string());
        let message = error.to_string();
        assert!(message.contains("Authorization"));
        assert!(message.contains("missing"));
    }

    #[test]
    fn test_upstream_error_structure() {
        let error = ClientError::UpstreamError {
            status: 404,
            body: "Not Found".to_string()
        };

        if let ClientError::UpstreamError { status, body } = error {
            assert_eq!(status, 404);
            assert_eq!(body, "Not Found");
        } else {
            panic!("Expected UpstreamError");
        }
    }
}
