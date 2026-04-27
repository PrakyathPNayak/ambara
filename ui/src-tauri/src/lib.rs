use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::State;

// Import the ambara library
use ambara::prelude::*;
use ambara::graph::structure::ProcessingGraph;
use ambara::execution::engine::{ExecutionEngine, ExecutionOptions};

struct AppState {
    filter_registry: Mutex<FilterRegistry>,
    plugin_registry: Mutex<PluginRegistry>,
}

impl AppState {
    fn new() -> Self {
        let plugin_dir = default_plugin_dir();
        let mut plugin_registry = PluginRegistry::new(plugin_dir, PluginSystemConfig::default());
        let mut filter_registry = FilterRegistry::with_builtins();

        // Best-effort auto-discovery; failures are surfaced in explicit commands.
        let _ = plugin_registry.load_all();
        let _ = plugin_registry.register_all_in_filter_registry(&mut filter_registry);

        Self {
            filter_registry: Mutex::new(filter_registry),
            plugin_registry: Mutex::new(plugin_registry),
        }
    }
}

fn default_plugin_dir() -> PathBuf {
    // ui/src-tauri -> repo root is ../../
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins")
}

// Types that mirror the frontend types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortDefinition {
    pub name: String,
    pub port_type: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub inputs: Vec<PortDefinition>,
    pub outputs: Vec<PortDefinition>,
    pub parameters: Vec<ParameterInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub library_path: String,
    pub healthy: bool,
    pub filter_count: usize,
    pub loaded_for_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterInfo {
    pub name: String,
    pub port_type: String,
    pub default_value: Option<serde_json::Value>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterValue {
    pub name: String,
    pub value: serde_json::Value,
    #[serde(rename = "type")]
    pub param_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterNodeData {
    pub filter_type: String,
    pub label: String,
    pub category: String,
    pub inputs: Vec<PortDefinition>,
    pub outputs: Vec<PortDefinition>,
    pub parameters: Vec<ParameterValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_valid: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub position: Position,
    pub data: FilterNodeData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    #[serde(rename = "sourceHandle")]
    pub source_handle: Option<String>,
    #[serde(rename = "targetHandle")]
    pub target_handle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphState {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationError {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    pub message: String,
    pub error_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationWarning {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionError {
    pub node_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionResult {
    pub success: bool,
    pub errors: Vec<ExecutionError>,
    pub outputs: HashMap<String, serde_json::Value>,
    pub execution_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalApiCapabilities {
    pub api_version: String,
    pub supports_graph_import_export: bool,
    pub supports_plugin_import_export: bool,
    pub supports_plugin_manifest_inspection: bool,
    pub supports_chatbot_assistant_hooks: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphExchangeEnvelope {
    pub format: String,
    pub schema_version: String,
    pub exported_at_unix_ms: u64,
    pub graph: GraphState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifestPreview {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub min_ambara_version: String,
    pub max_ambara_version: String,
    pub requested_capabilities: Vec<String>,
    pub declared_filters: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginImportIssue {
    pub path: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginImportSummary {
    pub loaded: Vec<PluginInfo>,
    pub failed: Vec<PluginImportIssue>,
}

/// Execution settings that can be configured from the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionSettings {
    /// Memory limit in megabytes (100-8192).
    pub memory_limit_mb: usize,
    /// Whether to enable automatic chunked processing.
    pub auto_chunk: bool,
    /// Preferred tile size for chunked processing.
    pub tile_size: u32,
    /// Whether to enable parallel execution.
    pub parallel: bool,
    /// Whether to enable caching.
    pub use_cache: bool,
}

impl Default for ExecutionSettings {
    fn default() -> Self {
        Self {
            memory_limit_mb: 500,
            auto_chunk: true,
            tile_size: 512,
            parallel: false,
            use_cache: false,
        }
    }
}

// Get default execution settings
#[tauri::command]
fn get_execution_settings() -> ExecutionSettings {
    ExecutionSettings::default()
}

// Get all available filters - uses the actual ambara FilterRegistry
#[tauri::command]
fn get_filters(state: State<AppState>) -> Result<Vec<FilterInfo>, String> {
    let registry = state
        .filter_registry
        .lock()
        .map_err(|_| "Filter registry lock poisoned".to_string())?;

    Ok(registry.filters()
        .map(|(_, entry)| {
            let metadata = &entry.metadata;
            filter_info_from_metadata(metadata)
        })
        .collect())
}

fn filter_info_from_metadata(metadata: &NodeMetadata) -> FilterInfo {
    FilterInfo {
        id: metadata.id.clone(),
        name: metadata.name.clone(),
        description: metadata.description.clone(),
        category: metadata.category.display_name().to_string(),
        inputs: metadata
            .inputs
            .iter()
            .map(|port| PortDefinition {
                name: port.name.clone(),
                port_type: format!("{:?}", port.port_type),
                required: !port.optional,
                default_value: port.default_value.as_ref().map(value_to_json),
            })
            .collect(),
        outputs: metadata
            .outputs
            .iter()
            .map(|port| PortDefinition {
                name: port.name.clone(),
                port_type: format!("{:?}", port.port_type),
                required: !port.optional,
                default_value: None,
            })
            .collect(),
        parameters: metadata
            .parameters
            .iter()
            .map(|param| ParameterInfo {
                name: param.name.clone(),
                port_type: format!("{:?}", param.param_type),
                default_value: Some(value_to_json(&param.default_value)),
                description: param.description.clone(),
            })
            .collect(),
    }
}

fn plugin_info_from_loaded(plugin: &ambara::plugins::loader::LoadedPlugin) -> PluginInfo {
    PluginInfo {
        id: plugin.manifest.plugin.id.clone(),
        name: plugin.manifest.plugin.name.clone(),
        version: plugin.manifest.plugin.version.clone(),
        description: plugin.manifest.plugin.description.clone(),
        author: plugin.manifest.plugin.author.clone(),
        library_path: plugin.library_path.display().to_string(),
        healthy: plugin.last_healthy,
        filter_count: plugin.filter_ids().len(),
        loaded_for_ms: plugin.loaded_at.elapsed().as_millis() as u64,
    }
}

#[tauri::command]
fn get_plugins(state: State<AppState>) -> Result<Vec<PluginInfo>, String> {
    let registry = state
        .plugin_registry
        .lock()
        .map_err(|_| "Plugin registry lock poisoned".to_string())?;

    let mut plugins = Vec::new();
    for plugin_id in registry.plugin_ids() {
        if let Some(info) = registry.with_plugin(plugin_id, plugin_info_from_loaded) {
            plugins.push(info);
        }
    }

    Ok(plugins)
}

#[tauri::command]
fn load_plugin(path: String, state: State<AppState>) -> Result<PluginInfo, String> {
    let mut plugin_registry = state
        .plugin_registry
        .lock()
        .map_err(|_| "Plugin registry lock poisoned".to_string())?;
    let mut filter_registry = state
        .filter_registry
        .lock()
        .map_err(|_| "Filter registry lock poisoned".to_string())?;

    let plugin_id = plugin_registry
        .load_plugin(Path::new(&path))
        .map_err(|e| e.to_string())?;

    if let Err(err) = plugin_registry.register_plugin_in_filter_registry(&plugin_id, &mut filter_registry) {
        let _ = plugin_registry.unload_plugin(&plugin_id);
        return Err(err.to_string());
    }

    plugin_registry
        .with_plugin(&plugin_id, plugin_info_from_loaded)
        .ok_or_else(|| format!("Loaded plugin '{plugin_id}' not found in registry"))
}

#[tauri::command]
fn unload_plugin(plugin_id: String, state: State<AppState>) -> Result<(), String> {
    let mut plugin_registry = state
        .plugin_registry
        .lock()
        .map_err(|_| "Plugin registry lock poisoned".to_string())?;
    let mut filter_registry = state
        .filter_registry
        .lock()
        .map_err(|_| "Filter registry lock poisoned".to_string())?;

    filter_registry.unregister_plugin_filters(&plugin_id);
    plugin_registry
        .unload_plugin(&plugin_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_plugin_filters(plugin_id: String, state: State<AppState>) -> Result<Vec<FilterInfo>, String> {
    let filter_registry = state
        .filter_registry
        .lock()
        .map_err(|_| "Filter registry lock poisoned".to_string())?;

    let ids = filter_registry.plugin_filters_for(&plugin_id);
    let filters = ids
        .iter()
        .filter_map(|id| filter_registry.get_metadata(id).map(filter_info_from_metadata))
        .collect();

    Ok(filters)
}

#[tauri::command]
fn get_external_api_capabilities() -> ExternalApiCapabilities {
    ExternalApiCapabilities {
        api_version: "v1".to_string(),
        supports_graph_import_export: true,
        supports_plugin_import_export: true,
        supports_plugin_manifest_inspection: true,
        supports_chatbot_assistant_hooks: false,
        notes: vec![
            "Chatbot assistant hooks are planned and tracked in roadmap TODOs".to_string(),
            "Graph schema is UI GraphState JSON envelope v1".to_string(),
        ],
    }
}

#[tauri::command]
fn export_graph_json(graph: GraphState) -> Result<String, String> {
    let envelope = GraphExchangeEnvelope {
        format: "ambara-graph".to_string(),
        schema_version: "1".to_string(),
        exported_at_unix_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis() as u64,
        graph,
    };
    serde_json::to_string_pretty(&envelope).map_err(|e| e.to_string())
}

#[tauri::command]
fn import_graph_json(content: String) -> Result<GraphState, String> {
    // First try the exchange envelope format.
    if let Ok(envelope) = serde_json::from_str::<GraphExchangeEnvelope>(&content) {
        return Ok(envelope.graph);
    }

    // Fallback to raw GraphState format for backward compatibility.
    serde_json::from_str::<GraphState>(&content).map_err(|e| e.to_string())
}

#[tauri::command]
fn inspect_plugin_manifest(path: String) -> Result<PluginManifestPreview, String> {
    let manifest = ambara::plugins::manifest::PluginManifest::from_path(Path::new(&path))
        .map_err(|e| e.to_string())?;

    let mut requested_capabilities = Vec::new();
    if manifest.plugin.capabilities.network {
        requested_capabilities.push("network".to_string());
    }
    if manifest.plugin.capabilities.filesystem_read {
        requested_capabilities.push("filesystem_read".to_string());
    }
    if manifest.plugin.capabilities.filesystem_write {
        requested_capabilities.push("filesystem_write".to_string());
    }
    if manifest.plugin.capabilities.gpu {
        requested_capabilities.push("gpu".to_string());
    }

    Ok(PluginManifestPreview {
        id: manifest.plugin.id,
        name: manifest.plugin.name,
        version: manifest.plugin.version,
        description: manifest.plugin.description,
        author: manifest.plugin.author,
        min_ambara_version: manifest.plugin.min_ambara_version,
        max_ambara_version: manifest.plugin.max_ambara_version,
        requested_capabilities,
        declared_filters: manifest.plugin.filters.ids,
    })
}

#[tauri::command]
fn import_plugins_from_directory(dir: String, state: State<AppState>) -> Result<PluginImportSummary, String> {
    let path = Path::new(&dir);
    if !path.exists() {
        return Err(format!("Directory does not exist: {dir}"));
    }
    if !path.is_dir() {
        return Err(format!("Path is not a directory: {dir}"));
    }

    let mut plugin_registry = state
        .plugin_registry
        .lock()
        .map_err(|_| "Plugin registry lock poisoned".to_string())?;
    let mut filter_registry = state
        .filter_registry
        .lock()
        .map_err(|_| "Filter registry lock poisoned".to_string())?;

    let mut loaded = Vec::new();
    let mut failed = Vec::new();

    let entries = std::fs::read_dir(path).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = match entry {
            Ok(v) => v,
            Err(err) => {
                failed.push(PluginImportIssue {
                    path: dir.clone(),
                    error: err.to_string(),
                });
                continue;
            }
        };
        let child = entry.path();
        if !child.is_dir() {
            continue;
        }

        let manifest_path = child.join("ambara-plugin.toml");
        if !manifest_path.exists() {
            continue;
        }

        let lib_path = std::fs::read_dir(&child)
            .ok()
            .and_then(|iter| {
                iter.flatten().find_map(|f| {
                    let p = f.path();
                    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or_default();
                    if ["so", "dll", "dylib"].contains(&ext) {
                        Some(p)
                    } else {
                        None
                    }
                })
            });

        let Some(lib_path) = lib_path else {
            failed.push(PluginImportIssue {
                path: child.display().to_string(),
                error: "No plugin library (.so/.dll/.dylib) found".to_string(),
            });
            continue;
        };

        let loaded_id = match plugin_registry.load_plugin(&lib_path) {
            Ok(id) => id,
            Err(err) => {
                failed.push(PluginImportIssue {
                    path: lib_path.display().to_string(),
                    error: err.to_string(),
                });
                continue;
            }
        };

        if let Err(err) = plugin_registry.register_plugin_in_filter_registry(&loaded_id, &mut filter_registry) {
            let _ = plugin_registry.unload_plugin(&loaded_id);
            failed.push(PluginImportIssue {
                path: lib_path.display().to_string(),
                error: err.to_string(),
            });
            continue;
        }

        if let Some(info) = plugin_registry.with_plugin(&loaded_id, plugin_info_from_loaded) {
            loaded.push(info);
        }
    }

    Ok(PluginImportSummary { loaded, failed })
}

#[tauri::command]
fn export_plugin_inventory_json(state: State<AppState>) -> Result<String, String> {
    let plugins = get_plugins(state)?;
    serde_json::to_string_pretty(&plugins).map_err(|e| e.to_string())
}

// Helper function to convert Value to JSON
fn value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Integer(i) => serde_json::json!(i),
        Value::Float(f) => serde_json::json!(f),
        Value::Boolean(b) => serde_json::json!(b),
        Value::String(s) => serde_json::json!(s),
        Value::Color(c) => serde_json::json!({
            "r": c.r,
            "g": c.g,
            "b": c.b,
            "a": c.a
        }),
        Value::Vector2(x, y) => serde_json::json!([x, y]),
        Value::Vector3(x, y, z) => serde_json::json!([x, y, z]),
        Value::Array(arr) => serde_json::json!(arr.iter().map(value_to_json).collect::<Vec<_>>()),
        Value::Map(map) => serde_json::json!(map.iter().map(|(k, v)| (k.clone(), value_to_json(v))).collect::<HashMap<_, _>>()),
        Value::None => serde_json::Value::Null,
        Value::Image(_) => serde_json::Value::Null, // Images can't be serialized to JSON directly
    }
}

// Validate graph
#[tauri::command]
fn validate_graph(graph: GraphState) -> ValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Check for empty graph
    if graph.nodes.is_empty() {
        warnings.push(ValidationWarning {
            node_id: None,
            message: "Graph is empty".to_string(),
        });
    }

    // Detect duplicate node ids: HashMap-keyed lookup downstream silently
    // overwrites entries for repeated ids, so any connection referencing the
    // duplicated id would be rerouted to whichever copy was inserted last.
    let mut seen_ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for node in &graph.nodes {
        if !seen_ids.insert(node.id.as_str()) {
            errors.push(ValidationError {
                node_id: Some(node.id.clone()),
                connection_id: None,
                message: format!("Duplicate node id: {}", node.id),
                error_type: "DuplicateNodeId".to_string(),
            });
        }
    }

    // Check for disconnected required inputs
    for node in &graph.nodes {
        for input in &node.data.inputs {
            if input.required {
                let has_connection = graph.edges.iter().any(|e| {
                    e.target == node.id && e.target_handle.as_deref() == Some(&input.name)
                });
                if !has_connection {
                    errors.push(ValidationError {
                        node_id: Some(node.id.clone()),
                        connection_id: None,
                        message: format!(
                            "Required input '{}' is not connected on node '{}'",
                            input.name, node.data.label
                        ),
                        error_type: "MissingConnection".to_string(),
                    });
                }
            }
        }
    }

    // Check for cycles (simple check - could be improved)
    // For now, just warn if graph might have cycles
    
    ValidationResult {
        valid: errors.is_empty(),
        errors,
        warnings,
    }
}

// Execute graph (placeholder - would connect to ambara library)
#[tauri::command]
fn execute_graph(graph: GraphState, settings: Option<ExecutionSettings>, state: State<AppState>) -> ExecutionResult {
    let start = std::time::Instant::now();
    let settings = settings.unwrap_or_default();
    
    // Validate first
    let validation = validate_graph(graph.clone());
    if !validation.valid {
        return ExecutionResult {
            success: false,
            errors: validation.errors.iter().map(|e| ExecutionError {
                node_id: e.node_id.clone().unwrap_or_default(),
                message: e.message.clone(),
            }).collect(),
            outputs: HashMap::new(),
            execution_time: start.elapsed().as_millis() as u64,
        };
    }

    // Convert UI graph to ambara ProcessingGraph
    let registry = match state.filter_registry.lock() {
        Ok(guard) => guard,
        Err(_) => {
            return ExecutionResult {
                success: false,
                errors: vec![ExecutionError {
                    node_id: String::new(),
                    message: "Failed to access filter registry".to_string(),
                }],
                outputs: HashMap::new(),
                execution_time: start.elapsed().as_millis() as u64,
            }
        }
    };
    let mut processing_graph = ProcessingGraph::new();
    let mut node_id_map: HashMap<String, NodeId> = HashMap::new();

    // Add nodes
    for ui_node in &graph.nodes {
        // Find the filter in registry
        let filter_type = &ui_node.data.filter_type;
        
        if let Some(filter) = registry.create(filter_type) {
            // Build the graph node with parameters
            let mut graph_node = ambara::graph::structure::GraphNode::new(filter)
                .with_position(ui_node.position.x, ui_node.position.y);

            // Add parameters
            for param in &ui_node.data.parameters {
                if let Some(value) = json_to_value(&param.value) {
                    graph_node = graph_node.with_parameter(&param.name, value);
                }
            }

            let added_node_id = processing_graph.add_node(graph_node);
            node_id_map.insert(ui_node.id.clone(), added_node_id);
        } else {
            return ExecutionResult {
                success: false,
                errors: vec![ExecutionError {
                    node_id: ui_node.id.clone(),
                    message: format!("Unknown filter type: {}", filter_type),
                }],
                outputs: HashMap::new(),
                execution_time: start.elapsed().as_millis() as u64,
            };
        }
    }

    // Add connections
    for edge in &graph.edges {
        if let (Some(&source_id), Some(&target_id)) = 
            (node_id_map.get(&edge.source), node_id_map.get(&edge.target)) 
        {
            let source_port = edge.source_handle.as_deref().unwrap_or("output");
            let target_port = edge.target_handle.as_deref().unwrap_or("input");
            
            if let Err(e) = processing_graph.connect(source_id, source_port, target_id, target_port) {
                return ExecutionResult {
                    success: false,
                    errors: vec![ExecutionError {
                        node_id: edge.id.clone(),
                        message: format!("Failed to connect: {:?}", e),
                    }],
                    outputs: HashMap::new(),
                    execution_time: start.elapsed().as_millis() as u64,
                };
            }
        }
    }

    // Execute the graph
    let engine = ExecutionEngine::new();
    let options = ExecutionOptions::default()
        .with_parallel(settings.parallel)
        .with_cache(settings.use_cache)
        .with_memory_limit_mb(settings.memory_limit_mb)
        .with_auto_chunk(settings.auto_chunk)
        .with_tile_size(settings.tile_size, settings.tile_size);

    match engine.execute(&processing_graph, Some(options)) {
        Ok(result) => {
            let mut outputs_map = HashMap::new();
            
            for (id, values) in &result.outputs {
                // Find original UI node ID and filter type
                let (ui_id, filter_type) = node_id_map.iter()
                    .find(|(_, &v)| v == *id)
                    .map(|(k, _)| {
                        let filter_type = graph.nodes.iter()
                            .find(|n| &n.id == k)
                            .map(|n| n.data.filter_type.clone())
                            .unwrap_or_default();
                        (k.clone(), filter_type)
                    })
                    .unwrap_or_else(|| (id.to_string(), String::new()));
                
                // Build output data
                let mut output_data = serde_json::json!({
                    "completed": true,
                    "output_count": values.len()
                });
                
                // For image_preview nodes, extract the thumbnail string
                if filter_type == "image_preview" {
                    if let Some(Value::String(thumbnail)) = values.get("thumbnail") {
                        output_data["thumbnail"] = serde_json::json!(thumbnail);
                    }
                    if let Some(Value::Integer(w)) = values.get("width") {
                        output_data["width"] = serde_json::json!(w);
                    }
                    if let Some(Value::Integer(h)) = values.get("height") {
                        output_data["height"] = serde_json::json!(h);
                    }
                }
                
                outputs_map.insert(ui_id, output_data);
            }
            
            ExecutionResult {
                success: true,
                errors: vec![],
                outputs: outputs_map,
                execution_time: start.elapsed().as_millis() as u64,
            }
        }
        Err(e) => {
            ExecutionResult {
                success: false,
                errors: vec![ExecutionError {
                    node_id: String::new(),
                    message: format!("Execution failed: {:?}", e),
                }],
                outputs: HashMap::new(),
                execution_time: start.elapsed().as_millis() as u64,
            }
        }
    }
}

// Helper to convert JSON to Value
fn json_to_value(json: &serde_json::Value) -> Option<Value> {
    match json {
        serde_json::Value::Null => Some(Value::None),
        serde_json::Value::Bool(b) => Some(Value::Boolean(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(Value::Integer(i))
            } else {
                n.as_f64().map(Value::Float)
            }
        }
        serde_json::Value::String(s) => Some(Value::String(s.clone())),
        serde_json::Value::Object(obj) => {
            // Check if it's a color
            if obj.contains_key("r") && obj.contains_key("g") && obj.contains_key("b") {
                let r = obj.get("r").and_then(|v| v.as_u64()).unwrap_or(255) as u8;
                let g = obj.get("g").and_then(|v| v.as_u64()).unwrap_or(255) as u8;
                let b = obj.get("b").and_then(|v| v.as_u64()).unwrap_or(255) as u8;
                let a = obj.get("a").and_then(|v| v.as_u64()).unwrap_or(255) as u8;
                Some(Value::Color(Color::new(r, g, b, a)))
            } else {
                None
            }
        }
        serde_json::Value::Array(arr) => {
            if arr.len() == 2 {
                let x = arr.first().and_then(|v| v.as_f64())?;
                let y = arr.get(1).and_then(|v| v.as_f64())?;
                Some(Value::Vector2(x, y))
            } else if arr.len() == 3 {
                let x = arr.first().and_then(|v| v.as_f64())?;
                let y = arr.get(1).and_then(|v| v.as_f64())?;
                let z = arr.get(2).and_then(|v| v.as_f64())?;
                Some(Value::Vector3(x, y, z))
            } else {
                None
            }
        }
    }
}

// Save graph to file
#[tauri::command]
fn save_graph(graph: GraphState, path: String) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&graph)
        .map_err(|e| e.to_string())?;
    std::fs::write(&path, json)
        .map_err(|e| e.to_string())?;
    Ok(())
}

// Load graph from file
#[tauri::command]
fn load_graph(path: String) -> Result<GraphState, String> {
    let content = std::fs::read_to_string(&path)
        .map_err(|e| e.to_string())?;
    let graph: GraphState = serde_json::from_str(&content)
        .map_err(|e| e.to_string())?;
    Ok(graph)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_filters,
            get_plugins,
            load_plugin,
            unload_plugin,
            get_plugin_filters,
            get_external_api_capabilities,
            export_graph_json,
            import_graph_json,
            inspect_plugin_manifest,
            import_plugins_from_directory,
            export_plugin_inventory_json,
            validate_graph,
            execute_graph,
            save_graph,
            load_graph,
            get_execution_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_api_capabilities_advertises_v1() {
        let caps = get_external_api_capabilities();
        assert_eq!(caps.api_version, "v1");
        assert!(caps.supports_graph_import_export);
        assert!(caps.supports_plugin_import_export);
        assert!(caps.supports_plugin_manifest_inspection);
        assert!(!caps.supports_chatbot_assistant_hooks);
        assert!(!caps.notes.is_empty());
    }

    fn sample_graph() -> GraphState {
        GraphState {
            nodes: vec![GraphNode {
                id: "n1".to_string(),
                node_type: "filter".to_string(),
                position: Position { x: 10.0, y: 20.0 },
                data: FilterNodeData {
                    filter_type: "passthrough".to_string(),
                    label: "test".to_string(),
                    category: "core".to_string(),
                    inputs: Vec::new(),
                    outputs: Vec::new(),
                    parameters: Vec::new(),
                    is_valid: None,
                    error_message: None,
                },
            }],
            edges: Vec::new(),
        }
    }

    #[test]
    fn export_then_import_envelope_preserves_graph() {
        let original = sample_graph();
        let json = export_graph_json(original.clone()).expect("export must succeed");
        let restored = import_graph_json(json).expect("import must succeed");

        // No PartialEq on GraphState; compare via serde_json::Value (strict).
        let a = serde_json::to_value(&original).unwrap();
        let b = serde_json::to_value(&restored).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn import_graph_json_accepts_raw_graphstate_for_back_compat() {
        // Older clients may send a bare GraphState (no envelope).
        let raw_json = serde_json::to_string(&sample_graph()).unwrap();
        let restored = import_graph_json(raw_json).expect("raw GraphState must still import");
        assert_eq!(restored.nodes.len(), 1);
        assert_eq!(restored.nodes[0].id, "n1");
    }

    #[test]
    fn import_graph_json_rejects_invalid_payloads() {
        assert!(import_graph_json("not json".to_string()).is_err());
        assert!(import_graph_json("{\"nodes\": \"oops\"}".to_string()).is_err());
    }

    #[test]
    fn validate_graph_flags_duplicate_node_ids() {
        let mut graph = sample_graph();
        let dup = graph.nodes[0].clone();
        graph.nodes.push(dup);
        let result = validate_graph(graph);
        assert!(!result.valid, "duplicate ids must invalidate the graph");
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.error_type == "DuplicateNodeId"),
            "expected DuplicateNodeId error, got {:?}",
            result.errors
        );
    }
}
