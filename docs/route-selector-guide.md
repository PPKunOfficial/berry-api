# 🎯 路由选择器使用指南

Berry API 内置智能路由选择器，采用 **SmartAI 模式** 进行负载均衡。本指南将详细介绍 SmartAI 路由选择器的工作原理、配置方式以及如何在实际应用中使用。

## 核心概念

-   **SmartAI 模式**：Berry API 的核心负载均衡策略。它根据多个实时指标（如后端健康状态、响应时间、错误率和动态权重）智能地评估并选择最优的后端服务来处理请求。
-   **后端 (Backend)**：指代实际提供 AI 模型服务的上游服务，例如 OpenAI API、Anthropic API 或 Google Gemini API。每个后端都与特定的提供商和模型关联。
-   **提供商 (Provider)**：后端服务的提供者，例如 `openai`、`anthropic`、`google`。
-   **模型 (Model)**：提供商提供的具体 AI 模型，例如 `gpt-4`、`claude-3-opus`、`gemini-pro`。
-   **动态权重 (Dynamic Weight)**：SmartAI 模式会根据后端的实时表现动态调整其权重。表现良好的后端会获得更高的权重，从而获得更多的请求；表现不佳的后端权重会降低，甚至在故障时被暂时移除。
-   **用户标签 (User Tags)**：在用户配置中定义的标签，可用于对后端进行精细化过滤。例如，您可以为某些后端打上 `dev`、`test` 或 `prod` 标签，然后通过用户标签将请求路由到特定环境的后端。

## SmartAI 模式工作原理

SmartAI 模式旨在提供一个自适应、高可用的负载均衡解决方案。其主要工作流程如下：

1.  **健康检查**：定期对所有配置的后端进行健康检查，确保它们可达且响应正常。不健康的后端会被暂时从可用列表中移除。
2.  **指标收集**：持续收集每个后端的性能指标，包括请求延迟、成功率、错误率等。
3.  **动态权重调整**：根据收集到的指标，SmartAI 算法会实时调整每个后端的动态权重。例如：
    -   低延迟、高成功率的后端权重会增加。
    -   高延迟、高错误率的后端权重会降低。
    -   长时间不健康的后端权重会降至零，直至恢复。
4.  **请求路由**：当接收到新的 API 请求时，路由选择器会根据当前所有健康后端的动态权重进行选择。权重越高的后端被选中的概率越大。
5.  **故障转移与恢复**：如果选中的后端在处理请求时失败，SmartAI 会立即将其标记为不健康，并尝试从其他健康后端中重新选择。一旦不健康的后端恢复正常，它会逐渐恢复其权重并重新加入到负载均衡池中。
6.  **用户标签过滤**：在路由选择之前，系统会根据请求用户的标签对可用的后端进行初步过滤。只有与用户标签匹配的后端才会被纳入 SmartAI 的选择范围。

## 配置 SmartAI

SmartAI 模式的配置主要通过 Berry API 的主配置文件（通常是 `config.toml`）进行。您可以在模型映射中为每个模型定义多个后端，并为每个后端设置初始权重、优先级和标签。

**示例 `config.toml` 片段**:

```toml
[models.gpt-4]
name = "GPT-4 Model"
strategy = "SmartAI" # 明确指定使用 SmartAI 策略
enabled = true

[[models.gpt-4.backends]]
provider = "openai"
model = "gpt-4-turbo"
weight = 1.0 # 初始权重
priority = 1 # 优先级，数字越小优先级越高
enabled = true
tags = ["prod", "us-east"] # 用户标签，用于精细化路由
billing_mode = "InputOutput"

[[models.gpt-4.backends]]
provider = "anthropic"
model = "claude-3-opus"
weight = 0.8 # 初始权重
priority = 2
enabled = true
tags = ["prod", "eu-west"]
billing_mode = "InputOutput"

[users.user1]
token = "user-token-123"
enabled = true
allowed_models = ["gpt-4"]
tags = ["prod", "us-east"] # 用户标签，请求将优先路由到带有这些标签的后端
```

在上述配置中：
-   `models.gpt-4.strategy = "SmartAI"` 明确指定了 `gpt-4` 模型使用 SmartAI 负载均衡策略。
-   每个后端都有 `weight`、`priority` 和 `tags` 属性，这些都会影响 SmartAI 的决策。
-   用户 `user1` 配置了 `tags = ["prod", "us-east"]`，这意味着 `user1` 的请求将优先路由到同时带有 `prod` 和 `us-east` 标签的后端。

## 监控 SmartAI 权重

您可以通过以下管理接口监控 SmartAI 的动态权重和后端健康状态：

-   **SmartAI 全局权重**: `/smart-ai/weights`
    -   获取所有后端在 SmartAI 模式下的当前动态权重、健康因子、延迟因子和错误因子等信息。
    -   示例请求: `curl http://localhost:3000/smart-ai/weights`

-   **特定模型 SmartAI 权重**: `/smart-ai/models/{model}/weights`
    -   获取特定模型下所有后端的 SmartAI 动态权重信息。
    -   示例请求: `curl http://localhost:3000/smart-ai/models/gpt-4/weights`

这些接口提供了透明度，让您可以了解 SmartAI 如何根据实时数据调整路由决策。

## 错误处理和状态码

当路由选择器无法找到合适的后端或后端返回错误时，Berry API 会返回相应的 HTTP 状态码和错误信息。

-   **404 Not Found**: 如果请求的模型不存在或没有可用的后端。
-   **429 Too Many Requests**: 如果触发了速率限制。
-   **500 Internal Server Error**: 如果后端返回了服务器错误或发生其他内部错误。
-   **502 Bad Gateway**: 如果后端无法连接或返回无效响应。
-   **503 Service Unavailable**: 如果所有后端均不健康或不可用。

错误响应通常包含一个 JSON 对象，提供 `type`、`message` 和 `code` 字段，以便客户端进行错误处理。

**示例错误响应**:

```json
{
  "error": {
    "type": "route_selection_failed",
    "message": "No healthy backends available for model 'gpt-4'",
    "code": 503
  }
}