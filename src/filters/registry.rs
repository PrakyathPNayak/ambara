//! Filter registry for managing available filter types.

use crate::core::node::{FilterNode, NodeMetadata, Category};
use indexmap::IndexMap;
use std::sync::Arc;

/// Factory function for creating filter instances.
pub type FilterFactory = Arc<dyn Fn() -> Box<dyn FilterNode> + Send + Sync>;

/// Registry entry containing metadata and factory.
#[derive(Clone)]
pub struct RegistryEntry {
    /// Factory function to create instances.
    pub factory: FilterFactory,
    /// Cached metadata (avoids creating instance just to get metadata).
    pub metadata: NodeMetadata,
    /// Whether this filter is enabled.
    pub enabled: bool,
    /// Tags for organization and search.
    pub tags: Vec<String>,
}

/// Registry for all available filter types.
///
/// The registry maintains a collection of filter factories that can be used
/// to create node instances. It provides methods for registration, lookup,
/// and organization of filters.
pub struct FilterRegistry {
    /// Filters indexed by their unique ID.
    filters: IndexMap<String, RegistryEntry>,
    /// Filters grouped by category.
    categories: IndexMap<Category, Vec<String>>,
}

impl FilterRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            filters: IndexMap::new(),
            categories: IndexMap::new(),
        }
    }

    /// Create a registry pre-populated with built-in filters.
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        
        // Register built-in filters
        crate::filters::builtin::register_all(&mut registry);
        
        registry
    }

    /// Register a filter type.
    pub fn register<F>(&mut self, factory: F)
    where
        F: Fn() -> Box<dyn FilterNode> + Send + Sync + 'static,
    {
        // Create a temporary instance to get metadata
        let instance = factory();
        let metadata = instance.metadata();
        let id = metadata.id.clone();
        let category = metadata.category.clone();

        let entry = RegistryEntry {
            factory: Arc::new(factory),
            metadata,
            enabled: true,
            tags: Vec::new(),
        };

        self.filters.insert(id.clone(), entry);

        // Add to category index
        self.categories
            .entry(category)
            .or_insert_with(Vec::new)
            .push(id);
    }

    /// Register a filter with additional tags.
    pub fn register_with_tags<F>(&mut self, factory: F, tags: Vec<String>)
    where
        F: Fn() -> Box<dyn FilterNode> + Send + Sync + 'static,
    {
        let instance = factory();
        let metadata = instance.metadata();
        let id = metadata.id.clone();
        let category = metadata.category.clone();

        let entry = RegistryEntry {
            factory: Arc::new(factory),
            metadata,
            enabled: true,
            tags,
        };

        self.filters.insert(id.clone(), entry);

        self.categories
            .entry(category)
            .or_insert_with(Vec::new)
            .push(id);
    }

    /// Create a new instance of a filter by ID.
    pub fn create(&self, id: &str) -> Option<Box<dyn FilterNode>> {
        self.filters.get(id).filter(|e| e.enabled).map(|e| (e.factory)())
    }

    /// Get metadata for a filter without creating an instance.
    pub fn get_metadata(&self, id: &str) -> Option<&NodeMetadata> {
        self.filters.get(id).map(|e| &e.metadata)
    }

    /// Get a registry entry.
    pub fn get_entry(&self, id: &str) -> Option<&RegistryEntry> {
        self.filters.get(id)
    }

    /// Check if a filter is registered.
    pub fn contains(&self, id: &str) -> bool {
        self.filters.contains_key(id)
    }

    /// Get all registered filter IDs.
    pub fn filter_ids(&self) -> impl Iterator<Item = &str> {
        self.filters.keys().map(|s| s.as_str())
    }

    /// Get all registered filters.
    pub fn filters(&self) -> impl Iterator<Item = (&str, &RegistryEntry)> {
        self.filters.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Get filters by category.
    pub fn filters_by_category(&self, category: &Category) -> Vec<&str> {
        self.categories
            .get(category)
            .map(|ids| ids.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Get all categories.
    pub fn categories(&self) -> impl Iterator<Item = &Category> {
        self.categories.keys()
    }

    /// Search filters by name or description.
    pub fn search(&self, query: &str) -> Vec<&str> {
        let query = query.to_lowercase();
        
        self.filters
            .iter()
            .filter(|(_, entry)| {
                let name_match = entry.metadata.name.to_lowercase().contains(&query);
                let desc_match = entry.metadata.description.to_lowercase().contains(&query);
                let tag_match = entry.tags.iter().any(|t| t.to_lowercase().contains(&query));
                let id_match = entry.metadata.id.to_lowercase().contains(&query);
                
                name_match || desc_match || tag_match || id_match
            })
            .map(|(id, _)| id.as_str())
            .collect()
    }

    /// Enable or disable a filter.
    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> bool {
        if let Some(entry) = self.filters.get_mut(id) {
            entry.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Add tags to a filter.
    pub fn add_tags(&mut self, id: &str, tags: Vec<String>) -> bool {
        if let Some(entry) = self.filters.get_mut(id) {
            entry.tags.extend(tags);
            true
        } else {
            false
        }
    }

    /// Unregister a filter.
    pub fn unregister(&mut self, id: &str) -> bool {
        if let Some(entry) = self.filters.shift_remove(id) {
            // Remove from category index
            if let Some(ids) = self.categories.get_mut(&entry.metadata.category) {
                ids.retain(|i| i != id);
            }
            true
        } else {
            false
        }
    }

    /// Get the total number of registered filters.
    pub fn len(&self) -> usize {
        self.filters.len()
    }

    /// Check if registry is empty.
    pub fn is_empty(&self) -> bool {
        self.filters.is_empty()
    }

    /// Get count of enabled filters.
    pub fn enabled_count(&self) -> usize {
        self.filters.values().filter(|e| e.enabled).count()
    }

    /// Get filters grouped by category for UI display.
    pub fn grouped_by_category(&self) -> IndexMap<Category, Vec<&NodeMetadata>> {
        let mut grouped: IndexMap<Category, Vec<&NodeMetadata>> = IndexMap::new();

        for entry in self.filters.values() {
            if entry.enabled {
                grouped
                    .entry(entry.metadata.category.clone())
                    .or_insert_with(Vec::new)
                    .push(&entry.metadata);
            }
        }

        // Sort each category by name
        for filters in grouped.values_mut() {
            filters.sort_by(|a, b| a.name.cmp(&b.name));
        }

        grouped
    }
}

impl Default for FilterRegistry {
    fn default() -> Self {
        Self::with_builtins()
    }
}

/// Builder for creating a customized registry.
pub struct RegistryBuilder {
    registry: FilterRegistry,
    include_builtins: bool,
}

impl RegistryBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            registry: FilterRegistry::new(),
            include_builtins: true,
        }
    }

    /// Include or exclude built-in filters.
    pub fn with_builtins(mut self, include: bool) -> Self {
        self.include_builtins = include;
        self
    }

    /// Register a custom filter.
    pub fn register<F>(mut self, factory: F) -> Self
    where
        F: Fn() -> Box<dyn FilterNode> + Send + Sync + 'static,
    {
        self.registry.register(factory);
        self
    }

    /// Build the registry.
    pub fn build(mut self) -> FilterRegistry {
        if self.include_builtins {
            crate::filters::builtin::register_all(&mut self.registry);
        }
        self.registry
    }
}

impl Default for RegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::node::PassthroughNode;

    #[test]
    fn test_register_and_create() {
        let mut registry = FilterRegistry::new();
        registry.register(|| Box::new(PassthroughNode));

        assert!(registry.contains("passthrough"));
        
        let filter = registry.create("passthrough");
        assert!(filter.is_some());
    }

    #[test]
    fn test_metadata_lookup() {
        let mut registry = FilterRegistry::new();
        registry.register(|| Box::new(PassthroughNode));

        let metadata = registry.get_metadata("passthrough");
        assert!(metadata.is_some());
        assert_eq!(metadata.unwrap().name, "Passthrough");
    }

    #[test]
    fn test_category_grouping() {
        let mut registry = FilterRegistry::new();
        registry.register(|| Box::new(PassthroughNode));

        let utility_filters = registry.filters_by_category(&Category::Utility);
        assert!(utility_filters.contains(&"passthrough"));
    }

    #[test]
    fn test_search() {
        let mut registry = FilterRegistry::new();
        registry.register(|| Box::new(PassthroughNode));

        let results = registry.search("pass");
        assert!(results.contains(&"passthrough"));

        let results = registry.search("nonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn test_enable_disable() {
        let mut registry = FilterRegistry::new();
        registry.register(|| Box::new(PassthroughNode));

        assert!(registry.create("passthrough").is_some());

        registry.set_enabled("passthrough", false);
        assert!(registry.create("passthrough").is_none());

        registry.set_enabled("passthrough", true);
        assert!(registry.create("passthrough").is_some());
    }

    #[test]
    fn test_unregister() {
        let mut registry = FilterRegistry::new();
        registry.register(|| Box::new(PassthroughNode));

        assert!(registry.contains("passthrough"));
        assert!(registry.unregister("passthrough"));
        assert!(!registry.contains("passthrough"));
    }
}
