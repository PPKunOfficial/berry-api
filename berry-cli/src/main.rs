//! Berry CLI Tool
//! 
//! Command line interface for managing Berry API

use clap::{Parser, Subcommand};
use anyhow::Result;

#[derive(Parser)]
#[command(name = "berry-cli")]
#[command(about = "A CLI tool for managing Berry API")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate configuration file
    ValidateConfig {
        /// Path to configuration file
        #[arg(short, long, default_value = "config.toml")]
        config: String,
    },
    /// Check backend health
    HealthCheck {
        /// Path to configuration file
        #[arg(short, long, default_value = "config.toml")]
        config: String,
        /// Specific provider to check
        #[arg(short, long)]
        provider: Option<String>,
    },
    /// Generate example configuration file
    GenerateConfig {
        /// Output path for configuration file
        #[arg(short, long, default_value = "config_example.toml")]
        output: String,
        /// Include advanced features
        #[arg(long)]
        advanced: bool,
    },
    /// Show service metrics and statistics
    Metrics {
        /// Path to configuration file
        #[arg(short, long, default_value = "config.toml")]
        config: String,
        /// Show detailed backend statistics
        #[arg(long)]
        detailed: bool,
    },
    /// Test backend connectivity
    TestBackend {
        /// Path to configuration file
        #[arg(short, long, default_value = "config.toml")]
        config: String,
        /// Provider name to test
        #[arg(short, long)]
        provider: String,
        /// Model name to test
        #[arg(short, long)]
        model: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::ValidateConfig { config } => {
            println!("Validating configuration file: {}", config);
            match berry_core::config::loader::load_config_from_path(&config) {
                Ok(cfg) => {
                    println!("✅ Configuration is valid");
                    println!("  - {} providers configured", cfg.providers.len());
                    println!("  - {} models configured", cfg.models.len());
                    println!("  - {} users configured", cfg.users.len());
                }
                Err(e) => {
                    eprintln!("❌ Configuration validation failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::HealthCheck { config, provider } => {
            println!("Performing health check...");
            let cfg = berry_core::config::loader::load_config_from_path(&config)?;
            let health_checker = berry_loadbalance::HealthChecker::new(std::sync::Arc::new(cfg), std::sync::Arc::new(Default::default()));
            
            if let Some(provider_id) = provider {
                println!("Checking provider: {}", provider_id);
                match health_checker.check_provider(&provider_id).await {
                    Ok(_) => println!("✅ Provider {} is healthy", provider_id),
                    Err(e) => {
                        eprintln!("❌ Provider {} health check failed: {}", provider_id, e);
                        std::process::exit(1);
                    }
                }
            } else {
                println!("Checking all providers...");
                match health_checker.check_now().await {
                    Ok(_) => println!("✅ Health check completed"),
                    Err(e) => {
                        eprintln!("❌ Health check failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
        Commands::GenerateConfig { output, advanced } => {
            println!("Generating configuration file: {}", output);
            generate_config_file(&output, advanced)?;
            println!("✅ Configuration file generated successfully");
        }
        Commands::Metrics { config, detailed } => {
            println!("Loading service metrics...");
            let cfg = berry_core::config::loader::load_config_from_path(&config)?;
            show_service_metrics(cfg, detailed).await?;
        }
        Commands::TestBackend { config, provider, model } => {
            println!("Testing backend connectivity: {}:{}", provider, model);
            let cfg = berry_core::config::loader::load_config_from_path(&config)?;
            test_backend_connectivity(cfg, &provider, &model).await?;
        }
    }

    Ok(())
}

/// 生成配置文件
fn generate_config_file(output_path: &str, advanced: bool) -> Result<()> {
    let config_content = if advanced {
        r#"# Berry API Advanced Configuration File
# This configuration includes all available features

[settings]
health_check_interval_seconds = 30
request_timeout_seconds = 30
max_retries = 3
circuit_breaker_failure_threshold = 5
circuit_breaker_timeout_seconds = 60
recovery_check_interval_seconds = 120
max_internal_retries = 2
health_check_timeout_seconds = 10

[settings.smart_ai]
enabled = true
small_traffic_threshold = 100
cost_control_factor = 0.8
health_check_weight = 0.3

# Provider Configuration with advanced features
[providers.openai]
name = "OpenAI"
base_url = "https://api.openai.com"
api_key = "your-openai-api-key-here"
models = ["gpt-3.5-turbo", "gpt-4", "gpt-4-turbo"]
enabled = true
timeout_seconds = 30
max_retries = 3
backend_type = "OpenAI"
tags = ["premium"]

[providers.openai.headers]
"X-Custom-Header" = "custom-value"

[providers.anthropic]
name = "Anthropic"
base_url = "https://api.anthropic.com"
api_key = "your-anthropic-api-key-here"
models = ["claude-3-sonnet-20240229", "claude-3-opus-20240229"]
enabled = true
timeout_seconds = 30
max_retries = 3
backend_type = "OpenAI"
tags = ["premium"]

[providers.local]
name = "Local LLM"
base_url = "http://localhost:8080"
api_key = ""
models = ["llama2-7b"]
enabled = true
timeout_seconds = 60
max_retries = 1
backend_type = "OpenAI"
tags = ["local", "cost-effective"]

# Advanced Model Mappings with multiple backends
[models.gpt-4-balanced]
name = "gpt-4-balanced"
enabled = true
strategy = "SmartWeightedFailover"

[[models.gpt-4-balanced.backends]]
provider = "openai"
model = "gpt-4"
weight = 0.7
priority = 1
enabled = true
billing_mode = "PerToken"
tags = ["premium"]

[[models.gpt-4-balanced.backends]]
provider = "anthropic"
model = "claude-3-sonnet-20240229"
weight = 0.3
priority = 2
enabled = true
billing_mode = "PerToken"
tags = ["premium"]

[models.cost-effective]
name = "cost-effective"
enabled = true
strategy = "WeightedRandom"

[[models.cost-effective.backends]]
provider = "local"
model = "llama2-7b"
weight = 0.8
priority = 1
enabled = true
billing_mode = "PerRequest"
tags = ["local"]

[[models.cost-effective.backends]]
provider = "openai"
model = "gpt-3.5-turbo"
weight = 0.2
priority = 2
enabled = true
billing_mode = "PerToken"
tags = ["fallback"]

# User Configuration with different access levels
[users.admin]
token = "admin-token-here"
enabled = true
models = ["gpt-4-balanced", "cost-effective"]
tags = ["premium", "local"]

[users.basic_user]
token = "basic-user-token-here"
enabled = true
models = ["cost-effective"]
tags = ["local"]

[users.premium_user]
token = "premium-user-token-here"
enabled = true
models = ["gpt-4-balanced"]
tags = ["premium"]
"#
    } else {
        r#"# Berry API Configuration File
# This is a basic configuration example

[settings]
health_check_interval_seconds = 30
request_timeout_seconds = 30
max_retries = 3
circuit_breaker_failure_threshold = 5
circuit_breaker_timeout_seconds = 60
recovery_check_interval_seconds = 120
max_internal_retries = 2
health_check_timeout_seconds = 10

[settings.smart_ai]
enabled = false
small_traffic_threshold = 100
cost_control_factor = 0.8
health_check_weight = 0.3

# Provider Configuration
[providers.openai]
name = "OpenAI"
base_url = "https://api.openai.com"
api_key = "your-openai-api-key-here"
models = ["gpt-3.5-turbo", "gpt-4"]
enabled = true
timeout_seconds = 30
max_retries = 3
backend_type = "OpenAI"

[providers.anthropic]
name = "Anthropic"
base_url = "https://api.anthropic.com"
api_key = "your-anthropic-api-key-here"
models = ["claude-3-sonnet-20240229"]
enabled = true
timeout_seconds = 30
max_retries = 3
backend_type = "OpenAI"

# Model Mappings
[models.gpt-3_5-turbo]
name = "gpt-3.5-turbo"
enabled = true
strategy = "WeightedRandom"

[[models.gpt-3_5-turbo.backends]]
provider = "openai"
model = "gpt-3.5-turbo"
weight = 1.0
priority = 1
enabled = true
billing_mode = "PerToken"

[models.claude-3-sonnet]
name = "claude-3-sonnet"
enabled = true
strategy = "WeightedRandom"

[[models.claude-3-sonnet.backends]]
provider = "anthropic"
model = "claude-3-sonnet-20240229"
weight = 1.0
priority = 1
enabled = true
billing_mode = "PerToken"

# User Configuration
[users.user1]
token = "your-user-token-here"
enabled = true
models = ["gpt-3.5-turbo", "claude-3-sonnet"]
"#
    };

    std::fs::write(output_path, config_content)?;
    Ok(())
}

/// 显示服务指标
async fn show_service_metrics(config: berry_core::Config, detailed: bool) -> Result<()> {
    let load_balancer = berry_loadbalance::LoadBalanceService::new(config)?;
    load_balancer.start().await?;

    // 等待一下让服务初始化
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let health = load_balancer.get_service_health().await;
    let metrics = load_balancer.get_metrics();

    println!("📊 Service Metrics");
    println!("==================");
    println!("Service Status: {}", if health.is_running { "🟢 Running" } else { "🔴 Stopped" });
    println!("Total Requests: {}", health.total_requests);
    println!("Successful Requests: {}", health.successful_requests);
    println!("Success Rate: {:.2}%", health.success_rate() * 100.0);
    println!();

    println!("🏥 Health Summary");
    println!("=================");
    println!("Total Providers: {}", health.health_summary.total_providers);
    println!("Healthy Providers: {}", health.health_summary.healthy_providers);
    println!("Total Models: {}", health.health_summary.total_models);
    println!("Healthy Models: {}", health.health_summary.healthy_models);
    println!("Provider Health Ratio: {:.2}%", health.health_summary.provider_health_ratio * 100.0);
    println!("Model Health Ratio: {:.2}%", health.health_summary.model_health_ratio * 100.0);
    println!();

    if detailed {
        println!("📈 Detailed Backend Statistics");
        println!("==============================");
        let request_counts = metrics.get_all_request_counts();
        for (backend_key, count) in request_counts {
            let parts: Vec<&str> = backend_key.split(':').collect();
            if parts.len() == 2 {
                let provider = parts[0];
                let model = parts[1];
                let is_healthy = metrics.is_healthy(provider, model);
                let failure_count = metrics.get_failure_count(provider, model);
                let latency = metrics.get_latency(provider, model);

                println!("Backend: {}", backend_key);
                println!("  Status: {}", if is_healthy { "🟢 Healthy" } else { "🔴 Unhealthy" });
                println!("  Requests: {}", count);
                println!("  Failures: {}", failure_count);
                if let Some(lat) = latency {
                    println!("  Latency: {}ms", lat.as_millis());
                }
                println!();
            }
        }
    }

    load_balancer.stop().await;
    Ok(())
}

/// 测试后端连接性
async fn test_backend_connectivity(config: berry_core::Config, provider_name: &str, model_name: &str) -> Result<()> {
    // 检查provider是否存在
    let provider = config.providers.get(provider_name)
        .ok_or_else(|| anyhow::anyhow!("Provider '{}' not found", provider_name))?;

    if !provider.enabled {
        println!("❌ Provider '{}' is disabled", provider_name);
        return Ok(());
    }

    if !provider.models.contains(&model_name.to_string()) {
        println!("❌ Model '{}' not found in provider '{}'", model_name, provider_name);
        println!("Available models: {:?}", provider.models);
        return Ok(());
    }

    println!("🔍 Testing connectivity to {}:{}", provider_name, model_name);
    println!("Base URL: {}", provider.base_url);
    println!();

    // 创建HTTP客户端
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    // 测试models API
    let models_url = format!("{}/v1/models", provider.base_url.trim_end_matches('/'));
    println!("Testing models API: {}", models_url);

    let mut request = client.get(&models_url);
    if !provider.api_key.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", provider.api_key));
    }

    match request.send().await {
        Ok(response) => {
            let status = response.status();
            println!("Models API Status: {} {}", status.as_u16(), status.canonical_reason().unwrap_or(""));

            if status.is_success() {
                println!("✅ Models API test passed");
            } else {
                println!("❌ Models API test failed");
                if let Ok(body) = response.text().await {
                    println!("Response: {}", body);
                }
            }
        }
        Err(e) => {
            println!("❌ Models API test failed: {}", e);
        }
    }

    println!();

    // 测试chat completions API
    let chat_url = format!("{}/v1/chat/completions", provider.base_url.trim_end_matches('/'));
    println!("Testing chat completions API: {}", chat_url);

    let test_body = serde_json::json!({
        "model": model_name,
        "messages": [
            {
                "role": "user",
                "content": "Hello, this is a connectivity test."
            }
        ],
        "max_tokens": 1,
        "stream": false
    });

    let mut request = client.post(&chat_url)
        .header("Content-Type", "application/json")
        .json(&test_body);

    if !provider.api_key.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", provider.api_key));
    }

    match request.send().await {
        Ok(response) => {
            let status = response.status();
            println!("Chat API Status: {} {}", status.as_u16(), status.canonical_reason().unwrap_or(""));

            if status.is_success() {
                println!("✅ Chat API test passed");
                println!("🎉 Backend {}:{} is fully functional!", provider_name, model_name);
            } else {
                println!("❌ Chat API test failed");
                if let Ok(body) = response.text().await {
                    println!("Response: {}", body);
                }
            }
        }
        Err(e) => {
            println!("❌ Chat API test failed: {}", e);
        }
    }

    Ok(())
}
