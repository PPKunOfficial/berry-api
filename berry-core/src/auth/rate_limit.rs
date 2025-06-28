use crate::config::model::RateLimit;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// 速率限制器状态
#[derive(Debug, Clone)]
struct RateLimitState {
    /// 每分钟请求计数
    minute_requests: Vec<Instant>,
    /// 每小时请求计数
    hour_requests: Vec<Instant>,
    /// 每天请求计数
    day_requests: Vec<Instant>,
    /// 最后清理时间
    last_cleanup: Instant,
}

impl RateLimitState {
    fn new() -> Self {
        Self {
            minute_requests: Vec::new(),
            hour_requests: Vec::new(),
            day_requests: Vec::new(),
            last_cleanup: Instant::now(),
        }
    }

    /// 清理过期的请求记录
    fn cleanup(&mut self) {
        let now = Instant::now();

        // 清理超过1分钟的记录
        self.minute_requests
            .retain(|&time| now.duration_since(time) < Duration::from_secs(60));

        // 清理超过1小时的记录
        self.hour_requests
            .retain(|&time| now.duration_since(time) < Duration::from_secs(3600));

        // 清理超过1天的记录
        self.day_requests
            .retain(|&time| now.duration_since(time) < Duration::from_secs(86400));

        self.last_cleanup = now;
    }

    /// 检查是否超过速率限制
    fn check_rate_limit(&mut self, limit: &RateLimit) -> Result<()> {
        let now = Instant::now();

        // 如果距离上次清理超过10秒，执行清理
        if now.duration_since(self.last_cleanup) > Duration::from_secs(10) {
            self.cleanup();
        }

        // 检查每分钟限制
        if self.minute_requests.len() >= limit.requests_per_minute as usize {
            anyhow::bail!(
                "Rate limit exceeded: {} requests per minute",
                limit.requests_per_minute
            );
        }

        // 检查每小时限制
        if self.hour_requests.len() >= limit.requests_per_hour as usize {
            anyhow::bail!(
                "Rate limit exceeded: {} requests per hour",
                limit.requests_per_hour
            );
        }

        // 检查每天限制
        if self.day_requests.len() >= limit.requests_per_day as usize {
            anyhow::bail!(
                "Rate limit exceeded: {} requests per day",
                limit.requests_per_day
            );
        }

        // 记录本次请求
        self.minute_requests.push(now);
        self.hour_requests.push(now);
        self.day_requests.push(now);

        Ok(())
    }

    /// 获取当前使用情况
    fn get_usage(&mut self) -> RateLimitUsage {
        self.cleanup();
        RateLimitUsage {
            requests_this_minute: self.minute_requests.len() as u32,
            requests_this_hour: self.hour_requests.len() as u32,
            requests_this_day: self.day_requests.len() as u32,
        }
    }
}

/// 速率限制使用情况
#[derive(Debug, Clone)]
pub struct RateLimitUsage {
    pub requests_this_minute: u32,
    pub requests_this_hour: u32,
    pub requests_this_day: u32,
}

/// 速率限制服务
pub struct RateLimitService {
    /// 用户速率限制状态
    user_states: Arc<RwLock<HashMap<String, RateLimitState>>>,
}

impl RateLimitService {
    /// 创建新的速率限制服务
    pub fn new() -> Self {
        Self {
            user_states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 检查用户的速率限制
    pub async fn check_rate_limit(&self, user_id: &str, limit: &RateLimit) -> Result<()> {
        let mut states = self.user_states.write().await;
        let state = states
            .entry(user_id.to_string())
            .or_insert_with(RateLimitState::new);

        state.check_rate_limit(limit)
    }

    /// 获取用户的速率限制使用情况
    pub async fn get_usage(&self, user_id: &str) -> Option<RateLimitUsage> {
        let mut states = self.user_states.write().await;
        states.get_mut(user_id).map(|state| state.get_usage())
    }

    /// 清理所有过期的状态（定期调用）
    pub async fn cleanup_expired_states(&self) {
        let mut states = self.user_states.write().await;
        let now = Instant::now();

        // 移除超过1天没有活动的用户状态
        states
            .retain(|_, state| now.duration_since(state.last_cleanup) < Duration::from_secs(86400));

        // 清理剩余状态中的过期记录
        for state in states.values_mut() {
            state.cleanup();
        }
    }
}

impl Default for RateLimitService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limit_basic() {
        let service = RateLimitService::new();
        let limit = RateLimit {
            requests_per_minute: 2,
            requests_per_hour: 10,
            requests_per_day: 100,
        };

        // 前两个请求应该成功
        assert!(service.check_rate_limit("user1", &limit).await.is_ok());
        assert!(service.check_rate_limit("user1", &limit).await.is_ok());

        // 第三个请求应该失败（超过每分钟限制）
        assert!(service.check_rate_limit("user1", &limit).await.is_err());
    }

    #[tokio::test]
    async fn test_rate_limit_different_users() {
        let service = RateLimitService::new();
        let limit = RateLimit {
            requests_per_minute: 1,
            requests_per_hour: 10,
            requests_per_day: 100,
        };

        // 不同用户应该有独立的限制
        assert!(service.check_rate_limit("user1", &limit).await.is_ok());
        assert!(service.check_rate_limit("user2", &limit).await.is_ok());

        // 同一用户的第二个请求应该失败
        assert!(service.check_rate_limit("user1", &limit).await.is_err());
    }

    #[tokio::test]
    async fn test_usage_tracking() {
        let service = RateLimitService::new();
        let limit = RateLimit {
            requests_per_minute: 10,
            requests_per_hour: 100,
            requests_per_day: 1000,
        };

        // 发送几个请求
        service.check_rate_limit("user1", &limit).await.unwrap();
        service.check_rate_limit("user1", &limit).await.unwrap();

        // 检查使用情况
        let usage = service.get_usage("user1").await.unwrap();
        assert_eq!(usage.requests_this_minute, 2);
        assert_eq!(usage.requests_this_hour, 2);
        assert_eq!(usage.requests_this_day, 2);
    }
}
