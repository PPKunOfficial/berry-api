use std::time::Duration;
use berry_core::client::{
    ClientFactory, UnifiedClient, ClientRegistry, register_global_client,
    ProviderBackendType, ClientError, OpenAIClient
};

/// 示例：如何创建和注册自定义客户端类型
/// 
/// 这个示例展示了如何使用新的插件化客户端注册表系统
/// 来动态添加新的AI后端类型支持

fn main() {
    // 初始化日志
    tracing_subscriber::init();

    println!("=== Berry API 客户端插件化系统示例 ===\n");

    // 1. 查看默认支持的后端类型
    println!("1. 默认支持的后端类型:");
    let supported_types = ClientFactory::supported_backend_types();
    for backend_type in &supported_types {
        println!("   - {:?}", backend_type);
    }
    println!("   总计: {} 种类型\n", ClientFactory::registered_client_count());

    // 2. 使用默认的客户端工厂创建客户端
    println!("2. 使用默认工厂创建 OpenAI 客户端:");
    match ClientFactory::create_client_from_provider_type(
        ProviderBackendType::OpenAI,
        "https://api.openai.com".to_string(),
        Duration::from_secs(30),
    ) {
        Ok(client) => {
            println!("   ✓ 成功创建 OpenAI 客户端");
            println!("   - 后端类型: {:?}", client.backend_type());
            println!("   - 基础URL: {}", client.base_url());
        }
        Err(e) => println!("   ✗ 创建失败: {}", e),
    }

    // 3. 创建自定义注册表
    println!("\n3. 创建自定义客户端注册表:");
    let custom_registry = ClientRegistry::new();
    println!("   ✓ 自定义注册表创建成功");
    println!("   - 默认支持: {} 种类型", custom_registry.count());

    // 4. 注册自定义客户端类型（这里演示重新注册 OpenAI 以展示覆盖功能）
    println!("\n4. 注册自定义客户端构建器:");
    custom_registry.register_client(
        ProviderBackendType::OpenAI,
        Box::new(|base_url, timeout| {
            println!("   🔧 使用自定义构建器创建 OpenAI 客户端");
            let client = OpenAIClient::with_base_url_and_timeout(base_url, timeout);
            Ok(UnifiedClient::OpenAI(client))
        }),
    );
    println!("   ✓ 自定义构建器注册成功");

    // 5. 使用自定义注册表创建客户端
    println!("\n5. 使用自定义注册表创建客户端:");
    match ClientFactory::create_client_from_provider_type_with_registry(
        &custom_registry,
        ProviderBackendType::OpenAI,
        "https://api.custom.com".to_string(),
        Duration::from_secs(60),
    ) {
        Ok(client) => {
            println!("   ✓ 使用自定义注册表创建成功");
            println!("   - 基础URL: {}", client.base_url());
        }
        Err(e) => println!("   ✗ 创建失败: {}", e),
    }

    // 6. 演示全局注册表的使用
    println!("\n6. 使用全局注册表:");
    
    // 注册到全局注册表
    register_global_client(
        ProviderBackendType::Claude,
        Box::new(|base_url, timeout| {
            println!("   🌍 使用全局注册的自定义 Claude 构建器");
            let client = berry_core::client::ClaudeClient::with_base_url_and_timeout(base_url, timeout);
            Ok(UnifiedClient::Claude(client))
        }),
    );
    
    // 使用全局注册表创建客户端
    match ClientFactory::create_client_from_provider_type(
        ProviderBackendType::Claude,
        "https://api.anthropic.com".to_string(),
        Duration::from_secs(45),
    ) {
        Ok(client) => {
            println!("   ✓ 使用全局注册表创建 Claude 客户端成功");
            println!("   - 后端类型: {:?}", client.backend_type());
        }
        Err(e) => println!("   ✗ 创建失败: {}", e),
    }

    // 7. 检查后端类型支持
    println!("\n7. 检查后端类型支持:");
    let test_types = vec![
        ProviderBackendType::OpenAI,
        ProviderBackendType::Claude,
        ProviderBackendType::Gemini,
    ];
    
    for backend_type in test_types {
        let supported = ClientFactory::supports_backend_type(&backend_type);
        println!("   - {:?}: {}", backend_type, if supported { "✓ 支持" } else { "✗ 不支持" });
    }

    // 8. 移除客户端类型注册
    println!("\n8. 移除客户端类型注册:");
    let removed = custom_registry.unregister_client(&ProviderBackendType::Gemini);
    println!("   - 移除 Gemini: {}", if removed { "✓ 成功" } else { "✗ 失败" });
    println!("   - 剩余类型数量: {}", custom_registry.count());

    // 9. 尝试创建已移除的客户端类型
    println!("\n9. 尝试创建已移除的客户端类型:");
    match ClientFactory::create_client_from_provider_type_with_registry(
        &custom_registry,
        ProviderBackendType::Gemini,
        "https://api.google.com".to_string(),
        Duration::from_secs(30),
    ) {
        Ok(_) => println!("   ✗ 意外成功（不应该发生）"),
        Err(e) => println!("   ✓ 预期失败: {}", e),
    }

    println!("\n=== 示例完成 ===");
    println!("\n💡 关键特性:");
    println!("   • 插件化客户端注册系统");
    println!("   • 支持动态添加新的AI后端类型");
    println!("   • 全局和局部注册表支持");
    println!("   • 运行时客户端类型检查");
    println!("   • 向后兼容的API设计");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_client_registration() {
        let registry = ClientRegistry::new();
        
        // 注册自定义客户端
        registry.register_client(
            ProviderBackendType::OpenAI,
            Box::new(|base_url, timeout| {
                let client = OpenAIClient::with_base_url_and_timeout(base_url, timeout);
                Ok(UnifiedClient::OpenAI(client))
            }),
        );
        
        // 验证注册成功
        assert!(registry.supports_backend(&ProviderBackendType::OpenAI));
        
        // 创建客户端
        let result = registry.create_client(
            ProviderBackendType::OpenAI,
            "https://test.com".to_string(),
            Duration::from_secs(30),
        );
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_client_factory_integration() {
        // 测试新的工厂方法
        assert!(ClientFactory::supports_backend_type(&ProviderBackendType::OpenAI));
        assert!(ClientFactory::supports_backend_type(&ProviderBackendType::Claude));
        assert!(ClientFactory::supports_backend_type(&ProviderBackendType::Gemini));
        
        // 测试客户端创建
        let result = ClientFactory::create_client_from_provider_type(
            ProviderBackendType::OpenAI,
            "https://api.openai.com".to_string(),
            Duration::from_secs(30),
        );
        
        assert!(result.is_ok());
    }
}
