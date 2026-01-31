use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Import the ambara library
use ambara::prelude::*;
use ambara::graph::structure::ProcessingGraph;
use ambara::execution::engine::{ExecutionEngine, ExecutionOptions};

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
fn get_filters() -> Vec<FilterInfo> {
    let registry = FilterRegistry::with_builtins();
    
    registry.filters()
        .map(|(_, entry)| {
            let metadata = &entry.metadata;
            FilterInfo {
                id: metadata.id.clone(),
                name: metadata.name.clone(),
                description: metadata.description.clone(),
                category: metadata.category.display_name().to_string(),
                inputs: metadata.inputs.iter().map(|port| PortDefinition {
                    name: port.name.clone(),
                    port_type: format!("{:?}", port.port_type),
                    required: !port.optional,
                    default_value: port.default_value.as_ref().map(|v| value_to_json(v)),
                }).collect(),
                outputs: metadata.outputs.iter().map(|port| PortDefinition {
                    name: port.name.clone(),
                    port_type: format!("{:?}", port.port_type),
                    required: !port.optional,
                    default_value: None,
                }).collect(),
                parameters: metadata.parameters.iter().map(|param| ParameterInfo {
                    name: param.name.clone(),
                    port_type: format!("{:?}", param.param_type),
                    default_value: Some(value_to_json(&param.default_value)),
                    description: param.description.clone(),
                }).collect(),
            }
        })
        .collect()
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
fn execute_graph(graph: GraphState, settings: Option<ExecutionSettings>) -> ExecutionResult {
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
    let registry = FilterRegistry::with_builtins();
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
            } else if let Some(f) = n.as_f64() {
                Some(Value::Float(f))
            } else {
                None
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
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_filters,
            validate_graph,
            execute_graph,
            save_graph,
            load_graph,
            get_execution_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
