# ⚖️ 负载均衡策略

Berry API 使用智能AI负载均衡策略，专为小流量、成本敏感的场景设计。



## 🧠 SmartAI策略详解

SmartAI是Berry API的核心创新，专为小流量、成本敏感的场景设计：

**核心特性：**

-   **成本感知选择**：优先选择便宜的后端，premium后端作为备选
-   **小流量优化**：80%选择最佳后端，20%探索其他选项
-   **智能健康检查**：基于用户请求进行被动验证
-   **信心度机制**：动态调整后端选择权重

**工作原理：**

```
1. 初始化：所有后端获得初始信心度(0.8)
2. 请求处理：根据信心度和权重选择后端
3. 结果反馈：成功提升信心度，失败降低信心度
4. 动态调整：信心度影响下次选择概率
5. 探索机制：20%流量用于测试其他后端
```

**配置示例：**

```toml
[models.smart_model]
name = "smart-model"
strategy = "smart_ai"  # 使用SmartAI策略
enabled = true

[[models.smart_model.backends]]
provider = "cheap-provider"
model = "gpt-3.5-turbo"
weight = 0.6    # 60%权重 - 成本优先
priority = 1
enabled = true
tags = []       # 普通后端

[[models.smart_model.backends]]
provider = "premium-provider"
model = "gpt-4"
weight = 0.4    # 40%权重 - 质量保证
priority = 2
enabled = true
tags = ["premium"]  # 标记为premium后端

[[models.smart_model.backends]]
provider = "backup-provider"
model = "gpt-3.5-turbo"
weight = 0.2    # 20%权重 - 备用服务
priority = 3
enabled = true
tags = []
```

**SmartAI的优势：**

- ✅ **成本控制**：优先使用便宜的后端，premium作为备选
- ✅ **智能故障转移**：自动检测和屏蔽不健康的后端
- ✅ **小流量优化**：适合个人使用的低流量场景
- ✅ **自适应学习**：根据历史表现动态调整选择策略
- ✅ **无需额外配置**：开箱即用的智能负载均衡
