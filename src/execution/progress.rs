//! Progress tracking for execution.

use crate::core::error::NodeId;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// A progress update event.
#[derive(Debug, Clone)]
pub enum ProgressUpdate {
    /// Execution has started.
    Started {
        total_nodes: usize,
    },
    /// A node has started processing.
    NodeStarted {
        node_id: NodeId,
        node_name: String,
        index: usize,
        total: usize,
    },
    /// A node has completed processing.
    NodeCompleted {
        node_id: NodeId,
        duration_ms: u64,
        index: usize,
        total: usize,
    },
    /// A node was skipped (disabled or cached).
    NodeSkipped {
        node_id: NodeId,
        reason: SkipReason,
    },
    /// Overall progress percentage.
    Progress {
        percent: f32,
        elapsed_ms: u64,
        estimated_remaining_ms: Option<u64>,
    },
    /// Execution has completed.
    Completed {
        total_duration_ms: u64,
        nodes_processed: usize,
        nodes_skipped: usize,
    },
    /// Execution was cancelled.
    Cancelled,
    /// An error occurred.
    Error {
        node_id: Option<NodeId>,
        message: String,
    },
}

/// Reason why a node was skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// Node is disabled.
    Disabled,
    /// Result was cached.
    Cached,
    /// Upstream node failed.
    UpstreamFailed,
}

/// Callback type for progress updates.
pub type ProgressCallback = Box<dyn Fn(ProgressUpdate) + Send + Sync>;

/// Tracks execution progress and allows cancellation.
pub struct ProgressTracker {
    /// Total number of nodes to process.
    total_nodes: usize,
    /// Number of nodes completed.
    completed_nodes: AtomicU64,
    /// Number of nodes skipped.
    skipped_nodes: AtomicU64,
    /// Whether execution is cancelled.
    cancelled: AtomicBool,
    /// Start time.
    start_time: Option<Instant>,
    /// Progress callback.
    callback: Option<ProgressCallback>,
    /// Node completion times for estimation.
    node_times: parking_lot::Mutex<Vec<u64>>,
}

impl ProgressTracker {
    /// Create a new progress tracker.
    pub fn new(total_nodes: usize) -> Self {
        Self {
            total_nodes,
            completed_nodes: AtomicU64::new(0),
            skipped_nodes: AtomicU64::new(0),
            cancelled: AtomicBool::new(false),
            start_time: None,
            callback: None,
            node_times: parking_lot::Mutex::new(Vec::new()),
        }
    }

    /// Create a progress tracker wrapped in Arc for sharing.
    pub fn new_shared(total_nodes: usize) -> Arc<Self> {
        Arc::new(Self::new(total_nodes))
    }

    /// Set a callback for progress updates.
    pub fn with_callback(mut self, callback: ProgressCallback) -> Self {
        self.callback = Some(callback);
        self
    }

    /// Start tracking.
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
        self.send_update(ProgressUpdate::Started {
            total_nodes: self.total_nodes,
        });
    }

    /// Report that a node has started.
    pub fn node_started(&self, node_id: NodeId, node_name: String) {
        let completed = self.completed_nodes.load(Ordering::Relaxed) as usize;
        self.send_update(ProgressUpdate::NodeStarted {
            node_id,
            node_name,
            index: completed,
            total: self.total_nodes,
        });
    }

    /// Report that a node has completed.
    pub fn node_completed(&self, node_id: NodeId, duration_ms: u64) {
        let completed = self.completed_nodes.fetch_add(1, Ordering::Relaxed) as usize + 1;
        
        // Record time for estimation
        self.node_times.lock().push(duration_ms);

        self.send_update(ProgressUpdate::NodeCompleted {
            node_id,
            duration_ms,
            index: completed,
            total: self.total_nodes,
        });

        // Send overall progress
        self.send_progress_update();
    }

    /// Report that a node was skipped.
    pub fn node_skipped(&self, node_id: NodeId, reason: SkipReason) {
        self.skipped_nodes.fetch_add(1, Ordering::Relaxed);
        self.send_update(ProgressUpdate::NodeSkipped { node_id, reason });
    }

    /// Check if execution should be cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Request cancellation.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
        self.send_update(ProgressUpdate::Cancelled);
    }

    /// Report an error.
    pub fn report_error(&self, node_id: Option<NodeId>, message: String) {
        self.send_update(ProgressUpdate::Error { node_id, message });
    }

    /// Complete tracking.
    pub fn complete(&self) {
        let duration = self
            .start_time
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);

        self.send_update(ProgressUpdate::Completed {
            total_duration_ms: duration,
            nodes_processed: self.completed_nodes.load(Ordering::Relaxed) as usize,
            nodes_skipped: self.skipped_nodes.load(Ordering::Relaxed) as usize,
        });
    }

    /// Get current progress percentage.
    pub fn progress_percent(&self) -> f32 {
        if self.total_nodes == 0 {
            return 100.0;
        }
        let completed = self.completed_nodes.load(Ordering::Relaxed);
        let skipped = self.skipped_nodes.load(Ordering::Relaxed);
        ((completed + skipped) as f32 / self.total_nodes as f32) * 100.0
    }

    /// Estimate remaining time in milliseconds.
    pub fn estimated_remaining_ms(&self) -> Option<u64> {
        let times = self.node_times.lock();
        if times.is_empty() {
            return None;
        }

        // Calculate average time per node
        let avg_time: u64 = times.iter().sum::<u64>() / times.len() as u64;
        let completed = self.completed_nodes.load(Ordering::Relaxed) as usize;
        let remaining = self.total_nodes.saturating_sub(completed);

        Some(avg_time * remaining as u64)
    }

    fn send_update(&self, update: ProgressUpdate) {
        if let Some(ref callback) = self.callback {
            callback(update);
        }
    }

    fn send_progress_update(&self) {
        let elapsed = self
            .start_time
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);

        self.send_update(ProgressUpdate::Progress {
            percent: self.progress_percent(),
            elapsed_ms: elapsed,
            estimated_remaining_ms: self.estimated_remaining_ms(),
        });
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::error::NodeId;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn test_progress_calculation() {
        let tracker = ProgressTracker::new(10);
        assert_eq!(tracker.progress_percent(), 0.0);

        tracker.completed_nodes.store(5, Ordering::Relaxed);
        assert_eq!(tracker.progress_percent(), 50.0);

        tracker.skipped_nodes.store(5, Ordering::Relaxed);
        assert_eq!(tracker.progress_percent(), 100.0);
    }

    #[test]
    fn test_cancellation() {
        let tracker = ProgressTracker::new(10);
        assert!(!tracker.is_cancelled());

        tracker.cancel();
        assert!(tracker.is_cancelled());
    }

    #[test]
    fn test_callback_invoked() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();
        let node_id = NodeId::new();

        let mut tracker = ProgressTracker::new(5).with_callback(Box::new(move |_| {
            call_count_clone.fetch_add(1, Ordering::Relaxed);
        }));

        tracker.start();
        tracker.node_started(node_id, "Test".to_string());
        tracker.node_completed(node_id, 100);

        // Should have received: Started, NodeStarted, NodeCompleted, Progress
        assert!(call_count.load(Ordering::Relaxed) >= 3);
    }
}
