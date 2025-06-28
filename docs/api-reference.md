# 🔌 API 参考文档

本文件将提供 Berry API 的详细接口参考。目前，Berry API 完全兼容 OpenAI API 格式。

## 核心 API

-   **聊天完成 (Chat Completions)**: `/v1/chat/completions`
    -   用于与 AI 模型进行对话。
    -   支持流式和非流式响应。
    -   兼容 OpenAI Chat Completions API。

-   **模型列表 (Models List)**: `/v1/models`
    -   获取当前可用的 AI 模型列表。
    -   兼容 OpenAI Models API。

## 管理 API

-   **健康检查**: `/health`
    -   获取服务的基础健康状态。

-   **指标**: `/metrics`
    -   获取详细的服务性能指标。

-   **Prometheus 指标**: `/prometheus`
    -   获取 Prometheus 格式的指标数据。

-   **模型权重**: `/admin/model-weights`
    -   获取当前模型的负载均衡权重信息。

-   **后端健康状态**: `/admin/backend-health`
    -   获取所有后端服务的健康状态。

-   **SmartAI 权重**: `/smart-ai/weights` 和 `/smart-ai/models/{model}/weights`
    -   查看 SmartAI 策略的动态权重。

## 认证

所有需要认证的 API 请求都应在 `Authorization` 头中包含 Bearer Token：

`Authorization: Bearer your-token-here`

有关详细的 API 使用示例，请参阅 [API 使用指南](api-guide.md)。
