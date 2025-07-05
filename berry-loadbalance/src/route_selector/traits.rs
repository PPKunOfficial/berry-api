use anyhow::Result;
use async_trait::async_trait;

use super::types::{RouteResult, RouteSelectionError, RouteStats, SelectedRoute};

/// 线路选择器 - 负载均衡的核心抽象
///
/// 这个trait将复杂的负载均衡逻辑抽象为简单的线路选择接口：
/// 1. 选择线路 - 根据模型名称选择最佳后端线路
/// 2. 报告状态 - 告知选择器请求的成功/失败状态
#[async_trait]
pub trait RouteSelector: Send + Sync {
    /// 选择线路
    ///
    /// # 参数
    /// - `model_name`: 请求的模型名称
    /// - `user_tags`: 可选的用户标签，用于过滤后端
    ///
    /// # 返回
    /// - `Ok(SelectedRoute)`: 成功选择的线路信息
    /// - `Err(RouteSelectionError)`: 选择失败的详细错误信息
    async fn select_route(
        &self,
        model_name: &str,
        user_tags: Option<&[String]>,
    ) -> Result<SelectedRoute, RouteSelectionError>;

    /// 选择指定提供商的线路（用于调试和测试）
    async fn select_specific_route(
        &self,
        model_name: &str,
        provider_name: &str,
    ) -> Result<SelectedRoute, RouteSelectionError>;

    /// 报告请求结果
    ///
    /// 这是选择器了解线路状态的唯一方式，用于：
    /// - 更新健康状态
    /// - 调整权重
    /// - 记录指标
    async fn report_result(&self, route_id: &str, result: RouteResult);

    /// 获取线路统计信息（用于监控）
    async fn get_route_stats(&self) -> RouteStats;
}
