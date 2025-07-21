# 🔌 API 参考文档

本文件将提供 Berry API 的详细接口参考。目前，Berry API 完全兼容 OpenAI API 格式。

## 核心 API

-   **聊天完成 (Chat Completions)**: `/v1/chat/completions`
    -   **方法**: `POST`
    -   **描述**: 用于与 AI 模型进行对话。支持流式和非流式响应。兼容 OpenAI Chat Completions API。
    -   **认证**: 需要 Bearer Token。
    -   **请求头**:
        -   `Content-Type: application/json`
        -   `Authorization: Bearer your-token-here`
    -   **请求体示例**:
        ```json
        {
          "model": "gpt-4",
          "messages": [
            {"role": "system", "content": "You are a helpful assistant."},
            {"role": "user", "content": "Hello, world!"}
          ],
          "stream": false,
          "max_tokens": 1000,
          "temperature": 0.7,
          "top_p": 1.0,
          "frequency_penalty": 0,
          "presence_penalty": 0
        }
        ```
    -   **响应示例**: 兼容 OpenAI Chat Completions API 响应格式。

-   **模型列表 (Models List)**: `/v1/models`
    -   **方法**: `GET`
    -   **描述**: 获取当前用户可用的 AI 模型列表。
    -   **认证**: 需要 Bearer Token。
    -   **响应示例**:
        ```json
        {
          "object": "list",
          "data": [
            {
              "id": "gpt-4",
              "object": "model",
              "created": 1677610602,
              "owned_by": "berry-api"
            },
            {
              "id": "gpt-3.5-turbo",
              "object": "model",
              "created": 1677610602,
              "owned_by": "berry-api"
            }
          ]
        }
        ```

## 管理 API

-   **健康检查**: `/health`
    -   **方法**: `GET`
    -   **描述**: 获取服务的基础健康状态。
    -   **认证**: 无
    -   **响应示例**: `Berry API - Load Balanced AI Gateway` (文本响应)

-   **详细指标**: `/metrics`
    -   **方法**: `GET`
    -   **描述**: 获取详细的服务性能指标。
    -   **认证**: 无
    -   **响应示例**: JSON 格式的指标数据。

-   **Prometheus 指标**: `/prometheus`
    -   **方法**: `GET`
    -   **描述**: 获取 Prometheus 格式的指标数据。
    -   **认证**: 无
    -   **响应示例**: Prometheus 文本格式。

-   **模型权重**: `/admin/model-weights`
    -   **方法**: `GET`
    -   **描述**: 获取当前模型的负载均衡权重信息。
    -   **认证**: 需要 Bearer Token (管理员权限)。
    -   **查询参数**: `model` (可选，字符串): 指定模型名称，不指定则返回所有模型。
    -   **响应示例**:
        ```json
        {
          "models": {
            "gpt-4": {
              "model_name": "gpt-4",
              "display_name": "GPT-4 Model",
              "strategy": "SmartAI",
              "enabled": true,
              "backends": [
                {
                  "provider": "openai",
                  "model": "gpt-4-turbo",
                  "original_weight": 1.0,
                  "effective_weight": 1.0,
                  "is_healthy": true,
                  "is_enabled": true,
                  "priority": 1,
                  "tags": [],
                  "billing_mode": "InputOutput",
                  "failure_count": 0
                }
              ],
              "total_effective_weight": 1.0
            }
          },
          "timestamp": "2025-07-21T04:00:00Z",
          "total_models": 1
        }
        ```

-   **后端健康状态**: `/admin/backend-health`
    -   **方法**: `GET`
    -   **描述**: 获取所有后端服务的健康状态。
    -   **认证**: 需要 Bearer Token (管理员权限)。
    -   **响应示例**:
        ```json
        {
          "backend_health": {
            "gpt-4": {
              "model_name": "GPT-4 Model",
              "enabled": true,
              "backends": [
                {
                  "provider": "openai",
                  "model": "gpt-4-turbo",
                  "enabled": true,
                  "is_healthy": true,
                  "failure_count": 0,
                  "is_in_unhealthy_list": false,
                  "tags": [],
                  "priority": 1
                }
              ]
            }
          },
          "timestamp": "2025-07-21T04:00:00Z"
        }
        ```

-   **系统统计信息**: `/admin/system-stats`
    -   **方法**: `GET`
    -   **描述**: 获取系统运行状态和可用模型摘要。
    -   **认证**: 需要 Bearer Token (管理员权限)。
    -   **响应示例**:
        ```json
        {
          "system": {
            "is_running": true,
            "total_models": 2,
            "available_models": ["gpt-4", "gpt-3.5-turbo"]
          },
          "health_summary": {
            "total_providers": 1,
            "healthy_providers": 1,
            "total_models": 2,
            "healthy_models": 2,
            "provider_health_ratio": 1.0,
            "model_health_ratio": 1.0,
            "is_system_healthy": true
          },
          "model_stats": {
            "gpt-4": {
              "healthy_backends": 1,
              "total_backends": 1,
              "health_ratio": 1.0,
              "is_healthy": true,
              "average_latency_ms": 100
            }
          },
          "timestamp": "2025-07-21T04:00:00Z"
        }
        ```

-   **SmartAI 全局权重**: `/smart-ai/weights`
    -   **方法**: `GET`
    -   **描述**: 获取 SmartAI 策略的全局动态权重信息。
    -   **认证**: 无
    -   **响应示例**:
        ```json
        {
          "status": "ok",
          "timestamp": "2025-07-21T04:00:00Z",
          "smart_ai_weights": {
            "openai:gpt-4-turbo": {
              "current_weight": 0.8,
              "base_weight": 1.0,
              "health_factor": 0.8,
              "latency_factor": 1.0,
              "error_factor": 1.0,
              "is_healthy": true,
              "last_updated": "2025-07-21T04:00:00Z"
            }
          }
        }
        ```

-   **特定模型 SmartAI 权重**: `/smart-ai/models/{model}/weights`
    -   **方法**: `GET`
    -   **描述**: 获取特定模型的 SmartAI 策略动态权重信息。
    -   **认证**: 无
    -   **路径参数**: `{model}` (字符串): 模型名称。
    -   **响应示例**:
        ```json
        {
          "status": "ok",
          "timestamp": "2025-07-21T04:00:00Z",
          "model": "gpt-4",
          "smart_ai_weights": {
            "openai:gpt-4-turbo": {
              "current_weight": 0.8,
              "base_weight": 1.0,
              "health_factor": 0.8,
              "latency_factor": 1.0,
              "error_factor": 1.0,
              "is_healthy": true,
              "last_updated": "2025-07-21T04:00:00Z"
            }
          }
        }
