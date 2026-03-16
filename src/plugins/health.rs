//! # Plugin Health Checking
//!
//! Health checks are run periodically against loaded plugins to detect
//! plugins that have entered a broken state (e.g., after a caught panic or
//! resource exhaustion).
//!
//! ## Design
//!
//! Each [`LoadedPlugin`] exposes a `health_check()` method that calls the
//! vtable's `plugin_health_check` function pointer. The result is captured
//! in a [`HealthReport`] which records the timestamp and the verdict.
//!
//! The registry's `health_check_all()` iterates all loaded plugins, collects
//! their reports, and returns the IDs of any unhealthy plugins for the caller
//! to decide whether to unload them.
//!
//! ## Examples
//!
//! ```rust,ignore
//! use ambara::plugins::health::HealthReport;
//! use std::time::Instant;
//!
//! let report = HealthReport::healthy("com.example.plugin", Instant::now());
//! assert!(report.is_healthy());
//! ```

use std::time::Instant;

/// The result of a single health-check invocation.
#[derive(Debug, Clone)]
pub struct HealthReport {
    /// Plugin ID that was checked.
    pub plugin_id: String,
    /// Whether the plugin reports itself as healthy.
    pub healthy: bool,
    /// Optional human-readable reason for an unhealthy result.
    pub reason: Option<String>,
    /// Timestamp of the check.
    pub checked_at: Instant,
}

impl HealthReport {
    /// Create a healthy report.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - ID of the plugin.
    /// * `checked_at` - When the check was performed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ambara::plugins::health::HealthReport;
    /// use std::time::Instant;
    /// let report = HealthReport::healthy("com.example.plugin", Instant::now());
    /// assert!(report.is_healthy());
    /// ```
    #[must_use]
    pub fn healthy(plugin_id: impl Into<String>, checked_at: Instant) -> Self {
        Self {
            plugin_id: plugin_id.into(),
            healthy: true,
            reason: None,
            checked_at,
        }
    }

    /// Create an unhealthy report with a reason.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - ID of the plugin.
    /// * `reason` - Human-readable description of the failure.
    /// * `checked_at` - When the check was performed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ambara::plugins::health::HealthReport;
    /// use std::time::Instant;
    /// let report = HealthReport::unhealthy(
    ///     "com.example.plugin",
    ///     "health_check returned ErrUnknown",
    ///     Instant::now(),
    /// );
    /// assert!(!report.is_healthy());
    /// ```
    #[must_use]
    pub fn unhealthy(
        plugin_id: impl Into<String>,
        reason: impl Into<String>,
        checked_at: Instant,
    ) -> Self {
        Self {
            plugin_id: plugin_id.into(),
            healthy: false,
            reason: Some(reason.into()),
            checked_at,
        }
    }

    /// Returns `true` if the plugin is healthy.
    #[must_use]
    pub fn is_healthy(&self) -> bool {
        self.healthy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healthy_report_is_healthy() {
        let r = HealthReport::healthy("com.test", Instant::now());
        assert!(r.is_healthy());
        assert!(r.reason.is_none());
    }

    #[test]
    fn unhealthy_report_has_reason() {
        let r = HealthReport::unhealthy("com.test", "panic detected", Instant::now());
        assert!(!r.is_healthy());
        assert_eq!(r.reason.as_deref(), Some("panic detected"));
    }
}
