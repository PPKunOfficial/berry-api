use berry_core::Backend;
use once_cell::sync::Lazy;
use rand::Rng;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, trace}; // Added

static INSTANT_EPOCH: Lazy<Instant> = Lazy::new(Instant::now); // Added

/// 缓存条目
#[derive(Debug)]
struct CacheEntry {
    backend: Backend,
    created_at: Instant,
    hit_count: AtomicU64,
    last_access: AtomicU64, // Changed to AtomicU64
}

impl Clone for CacheEntry {
    fn clone(&self) -> Self {
        Self {
            backend: self.backend.clone(),
            created_at: self.created_at,
            hit_count: AtomicU64::new(self.hit_count.load(Ordering::Relaxed)),
            last_access: AtomicU64::new(self.last_access.load(Ordering::Relaxed)), // Clone the atomic value
        }
    }
}

impl CacheEntry {
    fn new(backend: Backend) -> Self {
        let now_nanos = Instant::now().duration_since(*INSTANT_EPOCH).as_nanos() as u64; // Fixed
        Self {
            backend,
            created_at: Instant::now(),
            hit_count: AtomicU64::new(0),
            last_access: AtomicU64::new(now_nanos),
        }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        self.created_at.elapsed() > ttl
    }

    fn touch(&self) {
        // No longer async
        let now_nanos = Instant::now().duration_since(*INSTANT_EPOCH).as_nanos() as u64; // Fixed
        self.last_access.store(now_nanos, Ordering::Relaxed);
        self.hit_count.fetch_add(1, Ordering::Relaxed);
    }

    fn get_hit_count(&self) -> u64 {
        self.hit_count.load(Ordering::Relaxed)
    }
}

/// 后端选择缓存
///
/// 提供基于TTL的后端选择缓存机制，减少重复的后端选择计算
pub struct BackendSelectionCache {
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    ttl: Duration,
    max_entries: usize,
    // 统计信息
    total_requests: AtomicU64,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    evictions: AtomicU64,
}

impl Default for BackendSelectionCache {
    fn default() -> Self {
        Self::new(
            Duration::from_secs(30), // 30秒TTL
            1000,                    // 最大1000个条目
        )
    }
}

impl BackendSelectionCache {
    /// 创建新的缓存实例
    pub fn new(ttl: Duration, max_entries: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl,
            max_entries,
            total_requests: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
        }
    }

    /// 生成缓存键
    fn generate_cache_key(&self, model: &str, user_tags: Option<&[String]>) -> String {
        match user_tags {
            Some(tags) if !tags.is_empty() => {
                let mut sorted_tags = tags.to_vec();
                sorted_tags.sort();
                format!("{}:{}", model, sorted_tags.join(","))
            }
            _ => model.to_string(),
        }
    }

    /// 从缓存获取后端
    pub async fn get(&self, model: &str, user_tags: Option<&[String]>) -> Option<Backend> {
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        let cache_key = self.generate_cache_key(model, user_tags);

        let cache = self.cache.read().await;
        if let Some(entry) = cache.get(&cache_key) {
            if !entry.is_expired(self.ttl) {
                // 缓存命中
                entry.touch();
                self.cache_hits.fetch_add(1, Ordering::Relaxed);

                trace!(
                    "Cache hit for key '{}', hit_count: {}",
                    cache_key,
                    entry.get_hit_count()
                );

                return Some(entry.backend.clone());
            } else {
                // 缓存过期，需要在写锁中清理
                debug!("Cache entry expired for key '{}'", cache_key);
            }
        }

        // 缓存未命中
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
        trace!("Cache miss for key '{}'", cache_key);

        None
    }

    /// 将后端存入缓存
    pub async fn put(&self, model: &str, user_tags: Option<&[String]>, backend: Backend) {
        let cache_key = self.generate_cache_key(model, user_tags);

        let mut cache = self.cache.write().await;

        // 清理过期条目
        self.cleanup_expired_entries(&mut cache).await;

        // 检查是否需要驱逐条目
        if cache.len() >= self.max_entries {
            self.evict_lru_entry(&mut cache).await;
        }

        // 插入新条目
        let entry = CacheEntry::new(backend);
        cache.insert(cache_key.clone(), entry);

        debug!("Cached backend selection for key '{}'", cache_key);
    }

    /// 清理过期条目
    async fn cleanup_expired_entries(&self, cache: &mut HashMap<String, CacheEntry>) {
        let expired_keys: Vec<String> = cache
            .iter()
            .filter(|(_, entry)| entry.is_expired(self.ttl))
            .map(|(key, _)| key.clone())
            .collect();

        for key in expired_keys {
            cache.remove(&key);
            debug!("Removed expired cache entry: {}", key);
        }
    }

    /// 驱逐最少使用的条目 (随机采样)
    async fn evict_lru_entry(&self, cache: &mut HashMap<String, CacheEntry>) {
        if cache.is_empty() {
            return;
        }

        const SAMPLE_SIZE: usize = 5; // 随机采样大小
        let mut candidates = Vec::with_capacity(SAMPLE_SIZE);

        // 随机选择N个条目作为候选
        let keys: Vec<&String> = cache.keys().collect();
        let mut rng = rand::rng();

        for _ in 0..SAMPLE_SIZE {
            if keys.is_empty() {
                break;
            }
            let random_index = rng.random_range(0..keys.len());
            let key = keys[random_index];
            if let Some(entry) = cache.get(key) {
                candidates.push((key.clone(), entry.last_access.load(Ordering::Relaxed)));
            }
        }

        if candidates.is_empty() {
            return;
        }

        // 找到采样中最近最少使用的条目
        if let Some((lru_key, _)) = candidates
            .into_iter()
            .min_by_key(|&(_, last_access)| last_access)
        {
            cache.remove(&lru_key);
            self.evictions.fetch_add(1, Ordering::Relaxed);
            debug!("Evicted LRU cache entry: {}", lru_key);
        } else {
            // This case should theoretically not be reached because `candidates` is checked for emptiness above.
            // If it is reached, it indicates a deeper logic error.
            error!("LRU eviction: min_by_key returned None despite candidates not being empty. This is unexpected.");
        }
    }

    /// 清空缓存
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        debug!("Cache cleared");
    }

    /// 获取缓存统计信息
    pub fn get_stats(&self) -> CacheStats {
        let total = self.total_requests.load(Ordering::Relaxed);
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let evictions = self.evictions.load(Ordering::Relaxed);

        let hit_rate = if total > 0 {
            (hits as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        CacheStats {
            total_requests: total,
            cache_hits: hits,
            cache_misses: misses,
            hit_rate,
            evictions,
        }
    }

    /// 获取缓存大小
    pub async fn size(&self) -> usize {
        self.cache.read().await.len()
    }

    /// 重置统计信息
    pub fn reset_stats(&self) {
        self.total_requests.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
        self.evictions.store(0, Ordering::Relaxed);
    }
}

/// 缓存统计信息
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub hit_rate: f64,
    pub evictions: u64,
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Cache Stats: {} requests, {} hits ({:.1}%), {} misses, {} evictions",
            self.total_requests, self.cache_hits, self.hit_rate, self.cache_misses, self.evictions
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use berry_core::config::model::BillingMode;

    fn create_test_backend(provider: &str, model: &str) -> Backend {
        Backend {
            provider: provider.to_string(),
            model: model.to_string(),
            weight: 1.0,
            priority: 1,
            enabled: true,
            tags: vec![],
            billing_mode: BillingMode::PerToken,
        }
    }

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let cache = BackendSelectionCache::new(Duration::from_secs(60), 100);
        let backend = create_test_backend("test-provider", "test-model");

        // 测试缓存未命中
        assert!(cache.get("test-model", None).await.is_none());

        // 存入缓存
        cache.put("test-model", None, backend.clone()).await;

        // 测试缓存命中
        let cached_backend = cache.get("test-model", None).await;
        assert!(cached_backend.is_some());
        assert_eq!(cached_backend.unwrap().provider, "test-provider");

        // 验证统计信息
        let stats = cache.get_stats();
        assert_eq!(stats.total_requests, 2);
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.cache_misses, 1);
        assert_eq!(stats.hit_rate, 50.0);
    }

    #[tokio::test]
    async fn test_cache_with_user_tags() {
        let cache = BackendSelectionCache::default();
        let backend = create_test_backend("test-provider", "test-model");

        let tags = vec!["premium".to_string(), "fast".to_string()];

        // 存入带标签的缓存
        cache.put("test-model", Some(&tags), backend.clone()).await;

        // 测试相同标签的缓存命中
        assert!(cache.get("test-model", Some(&tags)).await.is_some());

        // 测试不同标签的缓存未命中
        let different_tags = vec!["basic".to_string()];
        assert!(cache
            .get("test-model", Some(&different_tags))
            .await
            .is_none());

        // 测试无标签的缓存未命中
        assert!(cache.get("test-model", None).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let cache = BackendSelectionCache::new(Duration::from_millis(100), 100);
        let backend = create_test_backend("test-provider", "test-model");

        // 存入缓存
        cache.put("test-model", None, backend).await;

        // 立即访问应该命中
        assert!(cache.get("test-model", None).await.is_some());

        // 等待过期
        tokio::time::sleep(Duration::from_millis(150)).await;

        // 过期后应该未命中
        assert!(cache.get("test-model", None).await.is_none());
    }
}
