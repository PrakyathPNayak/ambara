//! Result caching for execution.
//!
//! Caches node outputs to avoid re-computation when inputs haven't changed.

use crate::core::error::NodeId;
use crate::core::types::Value;
use lru::LruCache;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A cache key combining node ID with input hash.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// The node ID.
    pub node_id: NodeId,
    /// Hash of the inputs.
    pub input_hash: u64,
}

impl CacheKey {
    /// Create a new cache key.
    pub fn new(node_id: NodeId, inputs: &HashMap<String, Value>) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        
        // Sort keys for deterministic hashing
        let mut sorted_keys: Vec<_> = inputs.keys().collect();
        sorted_keys.sort();

        for key in sorted_keys {
            key.hash(&mut hasher);
            if let Some(value) = inputs.get(key) {
                hash_value(value, &mut hasher);
            }
        }

        Self {
            node_id,
            input_hash: hasher.finish(),
        }
    }
}

/// Hash a Value for caching purposes.
fn hash_value<H: Hasher>(value: &Value, hasher: &mut H) {
    std::mem::discriminant(value).hash(hasher);
    
    match value {
        Value::Integer(i) => i.hash(hasher),
        Value::Float(f) => f.to_bits().hash(hasher),
        Value::String(s) => s.hash(hasher),
        Value::Boolean(b) => b.hash(hasher),
        Value::Color(c) => {
            c.r.hash(hasher);
            c.g.hash(hasher);
            c.b.hash(hasher);
            c.a.hash(hasher);
        }
        Value::Vector2(x, y) => {
            x.to_bits().hash(hasher);
            y.to_bits().hash(hasher);
        }
        Value::Vector3(x, y, z) => {
            x.to_bits().hash(hasher);
            y.to_bits().hash(hasher);
            z.to_bits().hash(hasher);
        }
        Value::Array(arr) => {
            arr.len().hash(hasher);
            for v in arr {
                hash_value(v, hasher);
            }
        }
        Value::Map(map) => {
            map.len().hash(hasher);
            let mut sorted_keys: Vec<_> = map.keys().collect();
            sorted_keys.sort();
            for k in sorted_keys {
                k.hash(hasher);
                if let Some(v) = map.get(k) {
                    hash_value(v, hasher);
                }
            }
        }
        Value::Image(img) => {
            // Hash metadata
            img.metadata.width.hash(hasher);
            img.metadata.height.hash(hasher);
        }
        Value::None => {}
    }
}

/// Cached entry with metadata.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// The cached outputs.
    pub outputs: HashMap<String, Value>,
    /// When the entry was created.
    pub created_at: Instant,
    /// How long the original computation took.
    pub computation_time: Duration,
    /// Approximate memory size in bytes.
    pub memory_size: usize,
}

impl CacheEntry {
    /// Create a new cache entry.
    pub fn new(
        outputs: HashMap<String, Value>,
        computation_time: Duration,
    ) -> Self {
        let memory_size = estimate_memory_size(&outputs);
        Self {
            outputs,
            created_at: Instant::now(),
            computation_time,
            memory_size,
        }
    }

    /// Check if the entry has expired.
    pub fn is_expired(&self, ttl: Duration) -> bool {
        self.created_at.elapsed() > ttl
    }
}

/// Estimate memory size of a value map.
fn estimate_memory_size(outputs: &HashMap<String, Value>) -> usize {
    let mut size = std::mem::size_of::<HashMap<String, Value>>();
    
    for (key, value) in outputs {
        size += key.len();
        size += estimate_value_size(value);
    }
    
    size
}

/// Estimate memory size of a single value.
fn estimate_value_size(value: &Value) -> usize {
    match value {
        Value::Integer(_) => std::mem::size_of::<i64>(),
        Value::Float(_) => std::mem::size_of::<f64>(),
        Value::String(s) => std::mem::size_of::<String>() + s.len(),
        Value::Boolean(_) => std::mem::size_of::<bool>(),
        Value::Color(_) => std::mem::size_of::<f32>() * 4,
        Value::Vector2(_, _) => std::mem::size_of::<f64>() * 2,
        Value::Vector3(_, _, _) => std::mem::size_of::<f64>() * 3,
        Value::Array(arr) => {
            std::mem::size_of::<Vec<Value>>() + arr.iter().map(estimate_value_size).sum::<usize>()
        }
        Value::Map(map) => {
            std::mem::size_of::<HashMap<String, Value>>()
                + map.iter().map(|(k, v)| k.len() + estimate_value_size(v)).sum::<usize>()
        }
        Value::Image(img) => {
            let base = std::mem::size_of_val(img);
            base + img.estimated_memory_size()
        }
        Value::None => 0,
    }
}

/// Thread-safe result cache.
pub struct ResultCache {
    /// The LRU cache.
    cache: Mutex<LruCache<CacheKey, CacheEntry>>,
    /// Maximum memory usage in bytes.
    max_memory: usize,
    /// Current memory usage.
    current_memory: Mutex<usize>,
    /// Time-to-live for entries.
    ttl: Duration,
    /// Cache statistics.
    stats: Mutex<CacheStats>,
}

/// Cache statistics.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Number of cache hits.
    pub hits: u64,
    /// Number of cache misses.
    pub misses: u64,
    /// Number of entries evicted.
    pub evictions: u64,
    /// Total time saved by cache hits.
    pub time_saved: Duration,
}

impl CacheStats {
    /// Calculate hit ratio.
    pub fn hit_ratio(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            return 0.0;
        }
        self.hits as f64 / total as f64
    }
}

impl ResultCache {
    /// Create a new cache with the given capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Mutex::new(LruCache::new(
                NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(100).unwrap()),
            )),
            max_memory: 512 * 1024 * 1024, // 512 MB default
            current_memory: Mutex::new(0),
            ttl: Duration::from_secs(3600), // 1 hour default
            stats: Mutex::new(CacheStats::default()),
        }
    }

    /// Create a cache with custom memory limit.
    pub fn with_memory_limit(capacity: usize, max_memory_mb: usize) -> Self {
        let mut cache = Self::new(capacity);
        cache.max_memory = max_memory_mb * 1024 * 1024;
        cache
    }

    /// Set the TTL for cache entries.
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Get a cached result.
    pub fn get(&self, key: &CacheKey) -> Option<HashMap<String, Value>> {
        let mut cache = self.cache.lock();
        
        if let Some(entry) = cache.get(key) {
            // Check if expired
            if entry.is_expired(self.ttl) {
                cache.pop(key);
                let mut stats = self.stats.lock();
                stats.misses += 1;
                return None;
            }

            let mut stats = self.stats.lock();
            stats.hits += 1;
            stats.time_saved += entry.computation_time;
            
            Some(entry.outputs.clone())
        } else {
            let mut stats = self.stats.lock();
            stats.misses += 1;
            None
        }
    }

    /// Store a result in the cache.
    pub fn put(
        &self,
        key: CacheKey,
        outputs: HashMap<String, Value>,
        computation_time: Duration,
    ) {
        let entry = CacheEntry::new(outputs, computation_time);
        let entry_size = entry.memory_size;

        // Evict entries if needed to stay under memory limit
        {
            let mut current = self.current_memory.lock();
            while *current + entry_size > self.max_memory {
                let mut cache = self.cache.lock();
                if let Some((_, evicted)) = cache.pop_lru() {
                    *current = current.saturating_sub(evicted.memory_size);
                    let mut stats = self.stats.lock();
                    stats.evictions += 1;
                } else {
                    break;
                }
            }
            *current += entry_size;
        }

        let mut cache = self.cache.lock();
        cache.put(key, entry);
    }

    /// Invalidate a specific entry.
    pub fn invalidate(&self, key: &CacheKey) {
        let mut cache = self.cache.lock();
        if let Some(entry) = cache.pop(key) {
            let mut current = self.current_memory.lock();
            *current = current.saturating_sub(entry.memory_size);
        }
    }

    /// Invalidate all entries for a node.
    pub fn invalidate_node(&self, node_id: NodeId) {
        let mut cache = self.cache.lock();
        let keys_to_remove: Vec<_> = cache
            .iter()
            .filter(|(k, _)| k.node_id == node_id)
            .map(|(k, _)| k.clone())
            .collect();

        let mut total_freed = 0;
        for key in keys_to_remove {
            if let Some(entry) = cache.pop(&key) {
                total_freed += entry.memory_size;
            }
        }

        let mut current = self.current_memory.lock();
        *current = current.saturating_sub(total_freed);
    }

    /// Clear the entire cache.
    pub fn clear(&self) {
        let mut cache = self.cache.lock();
        cache.clear();
        *self.current_memory.lock() = 0;
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        self.stats.lock().clone()
    }

    /// Get current memory usage in bytes.
    pub fn memory_usage(&self) -> usize {
        *self.current_memory.lock()
    }

    /// Get number of cached entries.
    pub fn len(&self) -> usize {
        self.cache.lock().len()
    }

    /// Check if cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for ResultCache {
    fn default() -> Self {
        Self::new(100)
    }
}

/// A shared cache wrapped in Arc.
pub type SharedCache = Arc<ResultCache>;

/// Create a new shared cache.
pub fn new_shared_cache(capacity: usize) -> SharedCache {
    Arc::new(ResultCache::new(capacity))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::error::NodeId;

    fn create_test_outputs() -> HashMap<String, Value> {
        let mut outputs = HashMap::new();
        outputs.insert("result".to_string(), Value::Integer(42));
        outputs
    }

    #[test]
    fn test_cache_key_creation() {
        let mut inputs = HashMap::new();
        inputs.insert("a".to_string(), Value::Integer(1));
        inputs.insert("b".to_string(), Value::Float(2.0));

        let node1 = NodeId::new();
        let node2 = NodeId::new();

        let key1 = CacheKey::new(node1, &inputs);
        let key2 = CacheKey::new(node1, &inputs);
        let key3 = CacheKey::new(node2, &inputs);

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_cache_put_get() {
        let cache = ResultCache::new(10);
        let node_id = NodeId::new();
        let key = CacheKey {
            node_id,
            input_hash: 12345,
        };

        cache.put(key.clone(), create_test_outputs(), Duration::from_millis(100));
        
        let result = cache.get(&key);
        assert!(result.is_some());
        assert_eq!(result.unwrap().get("result"), Some(&Value::Integer(42)));
    }

    #[test]
    fn test_cache_miss() {
        let cache = ResultCache::new(10);
        let key = CacheKey {
            node_id: NodeId::new(),
            input_hash: 12345,
        };

        let result = cache.get(&key);
        assert!(result.is_none());

        let stats = cache.stats();
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_cache_invalidation() {
        let cache = ResultCache::new(10);
        let key = CacheKey {
            node_id: NodeId::new(),
            input_hash: 12345,
        };

        cache.put(key.clone(), create_test_outputs(), Duration::from_millis(100));
        assert!(cache.get(&key).is_some());

        cache.invalidate(&key);
        assert!(cache.get(&key).is_none());
    }

    #[test]
    fn test_invalidate_node() {
        let cache = ResultCache::new(10);
        let node1 = NodeId::new();
        let node2 = NodeId::new();
        
        // Add multiple entries for the same node
        for i in 0..5 {
            let key = CacheKey {
                node_id: node1,
                input_hash: i,
            };
            cache.put(key, create_test_outputs(), Duration::from_millis(100));
        }
        
        // Add entry for different node
        let other_key = CacheKey {
            node_id: node2,
            input_hash: 0,
        };
        cache.put(other_key.clone(), create_test_outputs(), Duration::from_millis(100));

        assert_eq!(cache.len(), 6);

        cache.invalidate_node(node1);

        assert_eq!(cache.len(), 1);
        assert!(cache.get(&other_key).is_some());
    }
}
