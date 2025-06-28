# 📈 性能优化与部署

### 性能调优建议

1.  **连接池优化**

    ```toml
    [settings]
    request_timeout_seconds = 30      # 根据网络情况调整
    max_retries = 3                   # 避免过多重试
    health_check_interval_seconds = 30 # 平衡检查频率和性能
    ```

2.  **权重分配策略**

    -   根据Provider的实际性能和成本调整权重
    -   高性能Provider分配更高权重
    -   备用Provider保持较低权重

3.  **超时设置**

    -   设置合理的请求超时时间
    -   避免过长的等待导致用户体验差
    -   考虑不同Provider的响应特性

4.  **熔断参数**

    ```toml
    circuit_breaker_failure_threshold = 5  # 根据容错需求调整
    circuit_breaker_timeout_seconds = 60   # 平衡恢复速度和稳定性
    ```
