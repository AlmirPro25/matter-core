// Matter Distributed Cache
// Redis-based distributed compilation cache for teams
#![allow(clippy::result_large_err)]

use matter_error::{ErrorType, MatterError};
use redis::{aio::ConnectionManager, AsyncCommands, Client};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

type Result<T> = std::result::Result<T, MatterError>;

fn runtime_error(message: impl Into<String>) -> MatterError {
    MatterError::new(ErrorType::Runtime, message)
}

/// Distributed cache client
pub struct DistributedCache {
    /// Redis connection
    client: Client,
    /// Cache prefix
    prefix: String,
    /// Default TTL
    default_ttl: Duration,
}

/// Cache entry metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub key: String,
    pub data: Vec<u8>,
    pub compressed: bool,
    pub created_at: SystemTime,
    pub hits: u64,
    pub size: usize,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_size: usize,
    pub hit_rate: f64,
    pub avg_entry_size: usize,
}

impl DistributedCache {
    /// Create a new distributed cache
    pub fn new(redis_url: &str, prefix: &str) -> Result<Self> {
        let client = Client::open(redis_url)
            .map_err(|e| runtime_error(format!("Failed to connect to Redis: {}", e)))?;

        Ok(Self {
            client,
            prefix: prefix.to_string(),
            default_ttl: Duration::from_secs(86400), // 24 hours
        })
    }

    /// Get connection manager
    async fn get_connection(&self) -> Result<ConnectionManager> {
        ConnectionManager::new(self.client.clone())
            .await
            .map_err(|e| runtime_error(format!("Failed to get Redis connection: {}", e)))
    }

    /// Build cache key with prefix
    fn build_key(&self, key: &str) -> String {
        format!("{}:{}", self.prefix, key)
    }

    /// Get value from cache
    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let mut conn = self.get_connection().await?;
        let cache_key = self.build_key(key);

        let data: Option<Vec<u8>> = conn
            .get(&cache_key)
            .await
            .map_err(|e| runtime_error(format!("Failed to get from cache: {}", e)))?;

        if data.is_some() {
            // Increment hit counter
            let hits_key = format!("{}:hits", cache_key);
            let _: () = conn
                .incr(&hits_key, 1)
                .await
                .map_err(|e| runtime_error(format!("Failed to increment hits: {}", e)))?;
        }

        Ok(data)
    }

    /// Set value in cache
    pub async fn set(&self, key: &str, data: Vec<u8>) -> Result<()> {
        self.set_with_ttl(key, data, self.default_ttl).await
    }

    /// Set value with custom TTL
    pub async fn set_with_ttl(&self, key: &str, data: Vec<u8>, ttl: Duration) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let cache_key = self.build_key(key);

        // Compress data
        let compressed = self.compress(&data)?;

        // Store entry
        let entry = CacheEntry {
            key: key.to_string(),
            data: compressed.clone(),
            compressed: true,
            created_at: SystemTime::now(),
            hits: 0,
            size: compressed.len(),
        };

        let entry_json = serde_json::to_vec(&entry)
            .map_err(|e| runtime_error(format!("Failed to serialize entry: {}", e)))?;

        let _: () = conn
            .set_ex(&cache_key, entry_json, ttl.as_secs())
            .await
            .map_err(|e| runtime_error(format!("Failed to set cache: {}", e)))?;

        Ok(())
    }

    /// Delete value from cache
    pub async fn delete(&self, key: &str) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let cache_key = self.build_key(key);

        let _: () = conn
            .del(&cache_key)
            .await
            .map_err(|e| runtime_error(format!("Failed to delete from cache: {}", e)))?;

        Ok(())
    }

    /// Check if key exists
    pub async fn exists(&self, key: &str) -> Result<bool> {
        let mut conn = self.get_connection().await?;
        let cache_key = self.build_key(key);

        let exists: bool = conn
            .exists(&cache_key)
            .await
            .map_err(|e| runtime_error(format!("Failed to check existence: {}", e)))?;

        Ok(exists)
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> Result<CacheStats> {
        let mut conn = self.get_connection().await?;
        let pattern = format!("{}:*", self.prefix);

        let keys: Vec<String> = conn
            .keys(&pattern)
            .await
            .map_err(|e| runtime_error(format!("Failed to get keys: {}", e)))?;

        let total_entries = keys.len();
        let mut total_size = 0;
        let mut total_hits = 0;

        for key in &keys {
            if let Ok(Some(data)) = self.get_raw(key).await {
                total_size += data.len();
            }

            let hits_key = format!("{}:hits", key);
            if let Ok(hits) = conn.get::<_, u64>(&hits_key).await {
                total_hits += hits;
            }
        }

        let avg_entry_size = total_size.checked_div(total_entries).unwrap_or(0);

        let hit_rate = if total_entries > 0 {
            total_hits as f64 / total_entries as f64
        } else {
            0.0
        };

        Ok(CacheStats {
            total_entries,
            total_size,
            hit_rate,
            avg_entry_size,
        })
    }

    /// Clear all cache entries
    pub async fn clear(&self) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let pattern = format!("{}:*", self.prefix);

        let keys: Vec<String> = conn
            .keys(&pattern)
            .await
            .map_err(|e| runtime_error(format!("Failed to get keys: {}", e)))?;

        if !keys.is_empty() {
            let _: () = conn
                .del(&keys)
                .await
                .map_err(|e| runtime_error(format!("Failed to clear cache: {}", e)))?;
        }

        Ok(())
    }

    /// Get raw data without decompression
    async fn get_raw(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let mut conn = self.get_connection().await?;

        let data: Option<Vec<u8>> = conn
            .get(key)
            .await
            .map_err(|e| runtime_error(format!("Failed to get raw data: {}", e)))?;

        Ok(data)
    }

    /// Compress data using LZ4
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        lz4::block::compress(data, None, false)
            .map_err(|e| runtime_error(format!("Failed to compress: {}", e)))
    }

    /// Decompress data using LZ4
    #[allow(dead_code)]
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        lz4::block::decompress(data, None)
            .map_err(|e| runtime_error(format!("Failed to decompress: {}", e)))
    }

    /// Set default TTL
    pub fn set_default_ttl(&mut self, ttl: Duration) {
        self.default_ttl = ttl;
    }

    /// Get cache key hash
    pub fn hash_key(data: &[u8]) -> String {
        let hash = blake3::hash(data);
        hash.to_hex().to_string()
    }
}

/// Distributed cache builder
pub struct DistributedCacheBuilder {
    redis_url: String,
    prefix: String,
    default_ttl: Duration,
}

impl DistributedCacheBuilder {
    /// Create a new builder
    pub fn new(redis_url: &str) -> Self {
        Self {
            redis_url: redis_url.to_string(),
            prefix: "matter".to_string(),
            default_ttl: Duration::from_secs(86400),
        }
    }

    /// Set cache prefix
    pub fn prefix(mut self, prefix: &str) -> Self {
        self.prefix = prefix.to_string();
        self
    }

    /// Set default TTL
    pub fn default_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Build the cache
    pub fn build(self) -> Result<DistributedCache> {
        let mut cache = DistributedCache::new(&self.redis_url, &self.prefix)?;
        cache.set_default_ttl(self.default_ttl);
        Ok(cache)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_hash() {
        let data = b"test data";
        let hash = DistributedCache::hash_key(data);
        assert_eq!(hash.len(), 64); // BLAKE3 produces 32-byte hash = 64 hex chars
    }

    #[test]
    fn test_builder() {
        let builder = DistributedCacheBuilder::new("redis://localhost")
            .prefix("test")
            .default_ttl(Duration::from_secs(3600));

        assert_eq!(builder.prefix, "test");
        assert_eq!(builder.default_ttl, Duration::from_secs(3600));
    }
}
