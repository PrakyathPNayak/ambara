use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Import the ambara library
use ambara::prelude::*;

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
fn execute_graph(graph: GraphState) -> ExecutionResult {
    let start = std::time::Instant::now();
    
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

    // TODO: Connect to the actual ambara library for execution
    // For now, return a mock success
    ExecutionResult {
        success: true,
        errors: vec![],
        outputs: HashMap::new(),
        execution_time: start.elapsed().as_millis() as u64,
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
        .invoke_handler(tauri::generate_handler![
            get_filters,
            validate_graph,
            execute_graph,
            save_graph,
            load_graph
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
