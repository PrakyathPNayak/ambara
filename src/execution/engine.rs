//! Execution engine implementation.
//!
//! The engine executes validated filter graphs.

use crate::core::context::ExecutionContext;
use crate::core::error::{AmbaraError, ExecutionError, NodeId};
use crate::core::types::Value;
use crate::execution::cache::{CacheKey, ResultCache, SharedCache};
use crate::execution::progress::{ProgressCallback, ProgressTracker, ProgressUpdate, SkipReason};
use crate::graph::structure::ProcessingGraph;
use crate::graph::topology::TopologyAnalyzer;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Execution options.
#[derive(Clone)]
pub struct ExecutionOptions {
    /// Whether to use parallel execution for independent nodes.
    pub parallel: bool,
    /// Maximum number of parallel threads (0 = use all available).
    pub max_threads: usize,
    /// Whether to use caching.
    pub use_cache: bool,
    /// Whether to stop on first error.
    pub stop_on_error: bool,
    /// Timeout for individual node execution.
    pub node_timeout: Option<Duration>,
    /// Whether to skip disabled nodes.
    pub skip_disabled: bool,
    /// Progress callback.
    pub progress_callback: Option<Arc<ProgressCallback>>,
    /// Memory limit in bytes for chunked processing.
    /// When processing large images, this limits peak memory usage.
    /// Default is 500 MB.
    pub memory_limit: usize,
    /// Whether to automatically use chunked processing for large images.
    /// When enabled, images that would exceed memory_limit / 2 are processed
    /// in tiles rather than all at once.
    pub auto_chunk: bool,
    /// Preferred tile size for chunked processing.
    /// Default is 512x512 pixels.
    pub tile_size: (u32, u32),
}

impl std::fmt::Debug for ExecutionOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionOptions")
            .field("parallel", &self.parallel)
            .field("max_threads", &self.max_threads)
            .field("use_cache", &self.use_cache)
            .field("stop_on_error", &self.stop_on_error)
            .field("node_timeout", &self.node_timeout)
            .field("skip_disabled", &self.skip_disabled)
            .field("progress_callback", &self.progress_callback.as_ref().map(|_| "<callback>"))
            .field("memory_limit", &self.memory_limit)
            .field("auto_chunk", &self.auto_chunk)
            .field("tile_size", &self.tile_size)
            .finish()
    }
}

impl Default for ExecutionOptions {
    fn default() -> Self {
        Self {
            parallel: true,
            max_threads: 0, // Use all available
            use_cache: true,
            stop_on_error: true,
            node_timeout: None,
            skip_disabled: true,
            progress_callback: None,
            memory_limit: crate::core::chunked::DEFAULT_MEMORY_LIMIT,
            auto_chunk: true,
            tile_size: (512, 512),
        }
    }
}

impl ExecutionOptions {
    /// Create a new options builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable/disable parallel execution.
    pub fn with_parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }

    /// Set maximum threads.
    pub fn with_max_threads(mut self, max: usize) -> Self {
        self.max_threads = max;
        self
    }

    /// Enable/disable caching.
    pub fn with_cache(mut self, use_cache: bool) -> Self {
        self.use_cache = use_cache;
        self
    }

    /// Enable/disable stop on error.
    pub fn with_stop_on_error(mut self, stop: bool) -> Self {
        self.stop_on_error = stop;
        self
    }

    /// Set node timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.node_timeout = Some(timeout);
        self
    }

    /// Set progress callback.
    pub fn with_progress<F>(mut self, callback: F) -> Self
    where
        F: Fn(ProgressUpdate) + Send + Sync + 'static,
    {
        self.progress_callback = Some(Arc::new(Box::new(callback)));
        self
    }

    /// Set memory limit in bytes for chunked processing.
    pub fn with_memory_limit(mut self, limit: usize) -> Self {
        self.memory_limit = limit;
        self
    }

    /// Set memory limit in megabytes for chunked processing.
    pub fn with_memory_limit_mb(mut self, mb: usize) -> Self {
        self.memory_limit = mb * 1024 * 1024;
        self
    }

    /// Enable or disable automatic chunked processing for large images.
    pub fn with_auto_chunk(mut self, auto_chunk: bool) -> Self {
        self.auto_chunk = auto_chunk;
        self
    }

    /// Set the preferred tile size for chunked processing.
    pub fn with_tile_size(mut self, width: u32, height: u32) -> Self {
        self.tile_size = (width, height);
        self
    }
}

/// Result of executing a graph.
#[derive(Debug)]
pub struct ExecutionResult {
    /// Outputs from all terminal nodes (nodes with no downstream connections).
    pub outputs: HashMap<NodeId, HashMap<String, Value>>,
    /// All node outputs (for inspection).
    pub all_outputs: HashMap<NodeId, HashMap<String, Value>>,
    /// Execution statistics.
    pub stats: ExecutionStats,
    /// Any errors that occurred (when stop_on_error is false).
    pub errors: Vec<(NodeId, ExecutionError)>,
}

/// Execution statistics.
#[derive(Debug, Clone, Default)]
pub struct ExecutionStats {
    /// Total execution time.
    pub total_duration: Duration,
    /// Number of nodes executed.
    pub nodes_executed: usize,
    /// Number of nodes skipped (disabled or cached).
    pub nodes_skipped: usize,
    /// Number of cache hits.
    pub cache_hits: usize,
    /// Time saved by caching.
    pub time_saved: Duration,
}

/// The execution engine.
pub struct ExecutionEngine {
    /// Result cache.
    cache: SharedCache,
    /// Default execution options.
    default_options: ExecutionOptions,
}

impl ExecutionEngine {
    /// Create a new execution engine.
    pub fn new() -> Self {
        Self {
            cache: Arc::new(ResultCache::new(100)),
            default_options: ExecutionOptions::default(),
        }
    }

    /// Create with a shared cache.
    pub fn with_cache(cache: SharedCache) -> Self {
        Self {
            cache,
            default_options: ExecutionOptions::default(),
        }
    }

    /// Set default options.
    pub fn with_default_options(mut self, options: ExecutionOptions) -> Self {
        self.default_options = options;
        self
    }

    /// Execute a graph.
    pub fn execute(
        &self,
        graph: &ProcessingGraph,
        options: Option<ExecutionOptions>,
    ) -> Result<ExecutionResult, AmbaraError> {
        let options = options.unwrap_or_else(|| self.default_options.clone());
        let start_time = Instant::now();

        // Get topological order
        let analyzer = TopologyAnalyzer::new(graph);
        let execution_order = analyzer.topological_sort()?;

        // Setup progress tracking
        let mut tracker = ProgressTracker::new(execution_order.len());
        if let Some(callback) = &options.progress_callback {
            let callback = callback.clone();
            tracker = tracker.with_callback(Box::new(move |update| callback(update)));
        }
        tracker.start();

        // Storage for outputs
        let mut all_outputs: HashMap<NodeId, HashMap<String, Value>> = HashMap::new();
        let mut errors: Vec<(NodeId, ExecutionError)> = Vec::new();
        let mut stats = ExecutionStats::default();

        if options.parallel {
            // Get parallel batches
            let batches = analyzer.parallel_batches()?;
            
            for batch in batches {
                if tracker.is_cancelled() {
                    tracker.complete();
                    return Err(AmbaraError::Execution(ExecutionError::Cancelled));
                }

                // Execute batch in parallel
                let batch_results: Vec<_> = batch
                    .par_iter()
                    .map(|&node_id| {
                        self.execute_node(
                            graph,
                            node_id,
                            &all_outputs,
                            &options,
                            &tracker,
                        )
                    })
                    .collect();

                // Process results
                for result in batch_results {
                    match result {
                        Ok((node_id, outputs, was_cached)) => {
                            if was_cached {
                                stats.cache_hits += 1;
                            } else {
                                stats.nodes_executed += 1;
                            }
                            all_outputs.insert(node_id, outputs);
                        }
                        Err((node_id, error)) => {
                            if options.stop_on_error {
                                tracker.report_error(Some(node_id), error.to_string());
                                tracker.complete();
                                return Err(AmbaraError::Execution(error));
                            }
                            errors.push((node_id, error));
                        }
                    }
                }
            }
        } else {
            // Sequential execution
            for node_id in execution_order {
                if tracker.is_cancelled() {
                    tracker.complete();
                    return Err(AmbaraError::Execution(ExecutionError::Cancelled));
                }

                match self.execute_node(graph, node_id, &all_outputs, &options, &tracker) {
                    Ok((_, outputs, was_cached)) => {
                        if was_cached {
                            stats.cache_hits += 1;
                        } else {
                            stats.nodes_executed += 1;
                        }
                        all_outputs.insert(node_id, outputs);
                    }
                    Err((_, error)) => {
                        if options.stop_on_error {
                            tracker.report_error(Some(node_id), error.to_string());
                            tracker.complete();
                            return Err(AmbaraError::Execution(error));
                        }
                        errors.push((node_id, error));
                    }
                }
            }
        }

        // Extract terminal node outputs
        let terminal_nodes: Vec<_> = graph
            .nodes()
            .filter(|n| graph.connections_from(n.id).next().is_none())
            .map(|n| n.id)
            .collect();

        let outputs: HashMap<_, _> = terminal_nodes
            .into_iter()
            .filter_map(|id| all_outputs.get(&id).map(|o| (id, o.clone())))
            .collect();

        stats.total_duration = start_time.elapsed();
        stats.nodes_skipped = graph.node_count() - stats.nodes_executed - stats.cache_hits;

        // Get cache stats
        let cache_stats = self.cache.stats();
        stats.time_saved = cache_stats.time_saved;

        tracker.complete();

        Ok(ExecutionResult {
            outputs,
            all_outputs,
            stats,
            errors,
        })
    }

    /// Execute a single node.
    fn execute_node(
        &self,
        graph: &ProcessingGraph,
        node_id: NodeId,
        upstream_outputs: &HashMap<NodeId, HashMap<String, Value>>,
        options: &ExecutionOptions,
        tracker: &ProgressTracker,
    ) -> Result<(NodeId, HashMap<String, Value>, bool), (NodeId, ExecutionError)> {
        let node = graph.get_node(node_id).map_err(|_| {
            (
                node_id,
                ExecutionError::NodeExecution {
                    node_id,
                    error: "Node not found".to_string(),
                },
            )
        })?;

        // Check if disabled
        if options.skip_disabled && node.disabled {
            tracker.node_skipped(node_id, SkipReason::Disabled);
            return Ok((node_id, HashMap::new(), false));
        }

        tracker.node_started(node_id, node.display_name().to_string());

        // Gather inputs
        let inputs = self.gather_inputs(graph, node_id, upstream_outputs);

        // Check cache
        if options.use_cache {
            let cache_key = CacheKey::new(node_id, &inputs);
            if let Some(cached) = self.cache.get(&cache_key) {
                tracker.node_skipped(node_id, SkipReason::Cached);
                return Ok((node_id, cached, true));
            }
        }

        // Build execution context with memory settings
        let mut ctx = ExecutionContext::with_memory_settings(
            node_id,
            options.memory_limit,
            options.auto_chunk,
            options.tile_size,
        );
        
        // Add inputs
        for (name, value) in &inputs {
            ctx.add_input(name.clone(), value.clone());
        }

        // Add parameters (with defaults)
        let metadata = node.filter.metadata();
        for param_def in &metadata.parameters {
            let value = node
                .parameters
                .get(&param_def.name)
                .cloned()
                .unwrap_or_else(|| param_def.default_value.clone());
            ctx.add_parameter(param_def.name.clone(), value);
        }

        // Execute with optional timeout
        let exec_start = Instant::now();
        let result = if let Some(_timeout) = options.node_timeout {
            // Note: Rust doesn't have built-in async timeout for sync code
            // In a real implementation, you'd want to use async execution
            // For now, we just execute synchronously
            node.filter.execute(&mut ctx)
        } else {
            node.filter.execute(&mut ctx)
        };

        let duration = exec_start.elapsed();

        match result {
            Ok(()) => {
                // Get outputs from context
                let outputs = ctx.take_outputs();
                
                // Cache result
                if options.use_cache {
                    let cache_key = CacheKey::new(node_id, &inputs);
                    self.cache.put(cache_key, outputs.clone(), duration);
                }

                tracker.node_completed(node_id, duration.as_millis() as u64);
                Ok((node_id, outputs, false))
            }
            Err(error) => {
                tracker.report_error(Some(node_id), error.to_string());
                Err((node_id, error))
            }
        }
    }

    /// Gather inputs for a node from upstream outputs.
    fn gather_inputs(
        &self,
        graph: &ProcessingGraph,
        node_id: NodeId,
        upstream_outputs: &HashMap<NodeId, HashMap<String, Value>>,
    ) -> HashMap<String, Value> {
        let mut inputs = HashMap::new();

        for conn in graph.connections_to(node_id) {
            if let Some(upstream_output) = upstream_outputs.get(&conn.from.node_id) {
                if let Some(value) = upstream_output.get(&conn.from.port_name) {
                    inputs.insert(conn.to.port_name.clone(), value.clone());
                }
            }
        }

        // Add default values for unconnected optional inputs
        if let Ok(node) = graph.get_node(node_id) {
            let metadata = node.filter.metadata();
            for input in &metadata.inputs {
                if !inputs.contains_key(&input.name) {
                    if let Some(ref default) = input.default_value {
                        inputs.insert(input.name.clone(), default.clone());
                    }
                }
            }
        }

        inputs
    }

    /// Clear the execution cache.
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Invalidate cache for a specific node.
    pub fn invalidate_node(&self, node_id: NodeId) {
        self.cache.invalidate_node(node_id);
    }

    /// Get cache statistics.
    pub fn cache_stats(&self) -> crate::execution::cache::CacheStats {
        self.cache.stats()
    }
}

impl Default for ExecutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::node::PassthroughNode;
    use crate::graph::structure::GraphNode;

    fn create_test_node() -> GraphNode {
        GraphNode::new(Box::new(PassthroughNode))
    }

    #[test]
    fn test_engine_creation() {
        let engine = ExecutionEngine::new();
        assert!(engine.cache_stats().hits == 0);
    }

    #[test]
    fn test_execute_empty_graph() {
        let engine = ExecutionEngine::new();
        let graph = ProcessingGraph::new();
        
        let result = engine.execute(&graph, None).unwrap();
        assert!(result.outputs.is_empty());
        assert!(result.all_outputs.is_empty());
    }

    #[test]
    fn test_execute_single_node_error_handling() {
        // Tests that the engine properly handles nodes that fail execution
        let engine = ExecutionEngine::new();
        let mut graph = ProcessingGraph::new();
        let _node_id = graph.add_node(create_test_node());

        // PassthroughNode needs input, so this should fail
        // Use stop_on_error = false to collect errors instead of returning immediately
        let options = ExecutionOptions::new().with_stop_on_error(false);
        let result = engine.execute(&graph, Some(options)).unwrap();
        
        // The execution completes but with errors
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_execution_options_builder() {
        let options = ExecutionOptions::new()
            .with_parallel(false)
            .with_max_threads(4)
            .with_cache(false)
            .with_stop_on_error(false)
            .with_timeout(Duration::from_secs(30));

        assert!(!options.parallel);
        assert_eq!(options.max_threads, 4);
        assert!(!options.use_cache);
        assert!(!options.stop_on_error);
        assert_eq!(options.node_timeout, Some(Duration::from_secs(30)));
    }
}
