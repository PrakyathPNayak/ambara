//! Ambara CLI - Node-based Image Processing.

use std::collections::HashMap;
use std::path::Path;

use ambara::graph::serialization::SerializedGraph;
use ambara::prelude::*;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct CliPortDefinition {
    name: String,
    #[serde(rename = "type")]
    port_type: String,
    required: bool,
}

#[derive(Debug, Serialize)]
struct CliParameterDefinition {
    name: String,
    #[serde(rename = "type")]
    param_type: String,
    default: String,
    description: String,
}

#[derive(Debug, Serialize)]
struct CliFilterMetadata {
    id: String,
    name: String,
    description: String,
    category: String,
    input_ports: Vec<CliPortDefinition>,
    output_ports: Vec<CliPortDefinition>,
    parameters: Vec<CliParameterDefinition>,
    tags: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LoadGraphResult {
    success: bool,
    errors: Vec<String>,
    outputs: HashMap<String, serde_json::Value>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage(&args[0]);
        return;
    }

    match args[1].as_str() {
        "list" => {
            let as_json = args.iter().any(|a| a == "--json");
            list_filters(as_json);
        }
        "info" => {
            if args.len() < 3 {
                eprintln!("Error: Please specify a filter ID");
                return;
            }
            filter_info(&args[2]);
        }
        "process" => {
            if args.len() < 4 {
                eprintln!("Error: Please specify input and output paths");
                eprintln!(
                    "Usage: {} process <input> <output> [--blur <sigma>] [--brightness <amount>]",
                    args[0]
                );
                return;
            }
            process_image(&args[2..]);
        }
        "load-graph" => {
            if args.len() < 4 {
                eprintln!("Usage: {} load-graph <graph.json> --dry-run|--execute", args[0]);
                std::process::exit(1);
            }
            let graph_path = &args[2];
            let dry_run = args.iter().any(|a| a == "--dry-run");
            let execute = args.iter().any(|a| a == "--execute");
            if !dry_run && !execute {
                eprintln!("Either --dry-run or --execute must be provided");
                std::process::exit(1);
            }

            let code = load_graph_command(graph_path, dry_run, execute);
            if code != 0 {
                std::process::exit(code);
            }
        }
        "help" | "--help" | "-h" => print_usage(&args[0]),
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_usage(&args[0]);
            std::process::exit(1);
        }
    }
}

fn print_usage(program: &str) {
    println!("Usage: {} <command> [options]", program);
    println!();
    println!("Commands:");
    println!("  list [--json]                     List all available filters");
    println!("  info <filter>                     Show detailed info about a filter");
    println!("  process <in> <out> [options]      Process an image");
    println!("  load-graph <path> --dry-run       Validate serialized graph only");
    println!("  load-graph <path> --execute       Validate and execute serialized graph");
    println!("  help                              Show this help message");
    println!();
    println!("Process options:");
    println!("  --blur <sigma>      Apply Gaussian blur (default: none)");
    println!("  --brightness <amt>  Adjust brightness -1.0 to 1.0 (default: 0)");
    println!("  --grayscale         Convert to grayscale");
    println!("  --resize <WxH>      Resize to dimensions (e.g., 800x600)");
}

fn list_filters(as_json: bool) {
    let registry = FilterRegistry::with_builtins();

    if as_json {
        let mut filters = Vec::new();
        for (_, entry) in registry.filters() {
            let metadata = &entry.metadata;
            let inputs = metadata
                .inputs
                .iter()
                .map(|p| CliPortDefinition {
                    name: p.name.clone(),
                    port_type: format!("{:?}", p.port_type),
                    required: !p.optional,
                })
                .collect();
            let outputs = metadata
                .outputs
                .iter()
                .map(|p| CliPortDefinition {
                    name: p.name.clone(),
                    port_type: format!("{:?}", p.port_type),
                    required: !p.optional,
                })
                .collect();
            let parameters = metadata
                .parameters
                .iter()
                .map(|param| CliParameterDefinition {
                    name: param.name.clone(),
                    param_type: format!("{:?}", param.param_type),
                    default: format!("{:?}", param.default_value),
                    description: param.description.clone(),
                })
                .collect();

            filters.push(CliFilterMetadata {
                id: metadata.id.clone(),
                name: metadata.name.clone(),
                description: metadata.description.clone(),
                category: metadata.category.display_name().to_string(),
                input_ports: inputs,
                output_ports: outputs,
                parameters,
                tags: metadata.tags.clone(),
            });
        }

        if let Ok(json) = serde_json::to_string_pretty(&filters) {
            println!("{json}");
        } else {
            eprintln!("Failed to serialize filter registry");
        }
        return;
    }

    let grouped = registry.grouped_by_category();
    println!("Available filters ({} total):", registry.len());
    println!();
    for (category, filters) in grouped {
        println!("  [{:?}]", category);
        for metadata in filters {
            println!("      - {} : {}", metadata.id, metadata.description);
        }
        println!();
    }
}

fn filter_info(filter_id: &str) {
    let registry = FilterRegistry::with_builtins();
    
    match registry.get_metadata(filter_id) {
        Some(metadata) => {
            println!("Filter: {}", metadata.name);
            println!("ID: {}", metadata.id);
            println!("Category: {:?}", metadata.category);
            println!("Version: {}", metadata.version);
            println!("Author: {}", metadata.author);
            println!();
            println!("Description:");
            println!("  {}", metadata.description);
            println!();

            if !metadata.inputs.is_empty() {
                println!("Inputs:");
                for port in &metadata.inputs {
                    let optional = if port.optional { " (optional)" } else { "" };
                    println!("  • {} [{:?}]{}", port.name, port.port_type, optional);
                    if !port.description.is_empty() {
                        println!("    {}", port.description);
                    }
                }
                println!();
            }

            if !metadata.outputs.is_empty() {
                println!("Outputs:");
                for port in &metadata.outputs {
                    println!("  • {} [{:?}]", port.name, port.port_type);
                    if !port.description.is_empty() {
                        println!("    {}", port.description);
                    }
                }
                println!();
            }

            if !metadata.parameters.is_empty() {
                println!("Parameters:");
                for param in &metadata.parameters {
                    println!("  • {} [{:?}] = {:?}", param.name, param.param_type, param.default_value);
                    if !param.description.is_empty() {
                        println!("    {}", param.description);
                    }
                }
            }
        }
        None => {
            eprintln!("Filter not found: {}", filter_id);
            eprintln!("Use 'list' to see available filters.");
        }
    }
}

fn process_image(args: &[String]) {
    let input_path = &args[0];
    let output_path = &args[1];
    
    // Parse options
    let mut blur_sigma: Option<f64> = None;
    let mut brightness: Option<f64> = None;
    let mut grayscale = false;
    let mut resize: Option<(u32, u32)> = None;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--blur" if i + 1 < args.len() => {
                blur_sigma = args[i + 1].parse().ok();
                i += 2;
            }
            "--brightness" if i + 1 < args.len() => {
                brightness = args[i + 1].parse().ok();
                i += 2;
            }
            "--grayscale" => {
                grayscale = true;
                i += 1;
            }
            "--resize" if i + 1 < args.len() => {
                if let Some((w, h)) = parse_dimensions(&args[i + 1]) {
                    resize = Some((w, h));
                }
                i += 2;
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                i += 1;
            }
        }
    }

    // Build and execute pipeline
    let registry = FilterRegistry::with_builtins();
    let mut graph = ProcessingGraph::new();

    // Load image
    let mut load_node = GraphNode::new(registry.create("load_image").unwrap());
    load_node.set_parameter("path", Value::String(input_path.clone()));
    let load = graph.add_node(load_node);

    let mut prev_node = load;
    let mut prev_port = "image";

    // Apply blur
    if let Some(sigma) = blur_sigma {
        let mut blur_node = GraphNode::new(registry.create("gaussian_blur").unwrap());
        blur_node.set_parameter("sigma", Value::Float(sigma));
        let blur = graph.add_node(blur_node);
        graph.connect(prev_node, prev_port, blur, "image").unwrap();
        prev_node = blur;
        prev_port = "image";
    }

    // Apply brightness
    if let Some(amt) = brightness {
        let mut bright_node = GraphNode::new(registry.create("brightness").unwrap());
        bright_node.set_parameter("amount", Value::Float(amt));
        let bright = graph.add_node(bright_node);
        graph.connect(prev_node, prev_port, bright, "image").unwrap();
        prev_node = bright;
        prev_port = "image";
    }

    // Apply grayscale
    if grayscale {
        let gray = graph.add_node(GraphNode::new(registry.create("grayscale").unwrap()));
        graph.connect(prev_node, prev_port, gray, "image").unwrap();
        prev_node = gray;
        prev_port = "image";
    }

    // Apply resize
    if let Some((w, h)) = resize {
        let mut res_node = GraphNode::new(registry.create("resize").unwrap());
        res_node.set_parameter("width", Value::Integer(w as i64));
        res_node.set_parameter("height", Value::Integer(h as i64));
        res_node.set_parameter("preserve_aspect", Value::Boolean(false));
        let res = graph.add_node(res_node);
        graph.connect(prev_node, prev_port, res, "image").unwrap();
        prev_node = res;
        prev_port = "image";
    }

    // Save image
    let mut save_node = GraphNode::new(registry.create("save_image").unwrap());
    save_node.set_parameter("path", Value::String(output_path.clone()));
    let save = graph.add_node(save_node);
    graph.connect(prev_node, prev_port, save, "image").unwrap();

    // Validate
    println!("🔍 Validating pipeline...");
    let pipeline = ValidationPipeline::default();
    let report = pipeline.validate(&graph);
    
    if !report.warnings.is_empty() {
        for warning in &report.warnings {
            println!("⚠️  {}", warning.message);
        }
    }
    
    if !report.errors.is_empty() {
        eprintln!("❌ Validation failed:");
        for error in &report.errors {
            eprintln!("   {}", error);
        }
        return;
    }

    // Execute
    println!("⚙️  Processing {} -> {}", input_path, output_path);
    let engine = ExecutionEngine::new();
    
    let options = ExecutionOptions::new()
        .with_progress(|update| {
            match update {
                ProgressUpdate::NodeStarted { node_name, .. } => {
                    println!("   • Running: {}", node_name);
                }
                ProgressUpdate::Completed { total_duration_ms, nodes_processed, .. } => {
                    println!("✅ Complete in {}ms ({} nodes)", total_duration_ms, nodes_processed);
                }
                ProgressUpdate::Error { message, .. } => {
                    eprintln!("❌ Error: {}", message);
                }
                _ => {}
            }
        });

    match engine.execute(&graph, Some(options)) {
        Ok(_result) => {
            println!("🎉 Image saved to: {}", output_path);
        }
        Err(e) => {
            eprintln!("❌ Execution failed: {}", e);
        }
    }
}

fn load_graph_command(path: &str, dry_run: bool, execute: bool) -> i32 {
    if !Path::new(path).exists() {
        eprintln!("Graph file does not exist: {path}");
        return 1;
    }

    let text = match std::fs::read_to_string(path) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("Failed to read graph file: {err}");
            return 1;
        }
    };

    let serialized = match parse_serialized_graph(&text) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("Invalid graph JSON: {err}");
            return 1;
        }
    };

    let registry = FilterRegistry::with_builtins();
    let errors = validate_serialized_graph(&serialized, &registry);
    if !errors.is_empty() {
        for error in errors {
            eprintln!("{error}");
        }
        return 1;
    }

    if dry_run {
        println!("Graph validation passed");
        return 0;
    }

    if execute {
        let execution = execute_serialized_graph(&serialized, &registry);
        let out = serde_json::to_string_pretty(&execution).unwrap_or_else(|_| {
            "{\"success\":false,\"errors\":[\"serialization error\"],\"outputs\":{}}".to_string()
        });
        println!("{out}");
        return if execution.success { 0 } else { 1 };
    }

    0
}

fn parse_serialized_graph(text: &str) -> Result<SerializedGraph, serde_json::Error> {
    if let Ok(graph) = SerializedGraph::from_json(text) {
        return Ok(graph);
    }

    let mut raw: serde_json::Value = serde_json::from_str(text)?;
    if raw.get("version").is_none() {
        raw["version"] = serde_json::Value::String(SerializedGraph::VERSION.to_string());
    }
    if raw.get("metadata").is_none() || !raw["metadata"].is_object() {
        raw["metadata"] = serde_json::json!({});
    }
    if raw["metadata"].get("tags").is_none() {
        raw["metadata"]["tags"] = serde_json::json!([]);
    }
    serde_json::from_value(raw)
}

fn validate_serialized_graph(graph: &SerializedGraph, registry: &FilterRegistry) -> Vec<String> {
    let mut errors = Vec::new();
    let mut node_ids = std::collections::HashSet::new();

    for node in &graph.nodes {
        node_ids.insert(node.id);
        if !registry.contains(&node.filter_id) {
            errors.push(format!("Unknown filter id: {}", node.filter_id));
        }
    }

    for conn in &graph.connections {
        if !node_ids.contains(&conn.from_node) {
            errors.push(format!("Connection references unknown from_node: {}", conn.from_node));
        }
        if !node_ids.contains(&conn.to_node) {
            errors.push(format!("Connection references unknown to_node: {}", conn.to_node));
        }
    }

    errors
}

fn execute_serialized_graph(graph: &SerializedGraph, registry: &FilterRegistry) -> LoadGraphResult {
    let mut processing_graph = ProcessingGraph::new();
    let mut node_map: HashMap<NodeId, NodeId> = HashMap::new();

    for node in &graph.nodes {
        let Some(filter) = registry.create(&node.filter_id) else {
            return LoadGraphResult {
                success: false,
                errors: vec![format!("Unknown filter id: {}", node.filter_id)],
                outputs: HashMap::new(),
            };
        };

        let mut graph_node = GraphNode::new(filter).with_position(node.position.x, node.position.y);
        for (key, value) in &node.parameters {
            graph_node = graph_node.with_parameter(key, value.clone());
        }

        let new_id = processing_graph.add_node(graph_node);
        node_map.insert(node.id, new_id);
    }

    for conn in &graph.connections {
        let Some(from) = node_map.get(&conn.from_node).copied() else {
            return LoadGraphResult {
                success: false,
                errors: vec![format!("Unknown from_node {}", conn.from_node)],
                outputs: HashMap::new(),
            };
        };
        let Some(to) = node_map.get(&conn.to_node).copied() else {
            return LoadGraphResult {
                success: false,
                errors: vec![format!("Unknown to_node {}", conn.to_node)],
                outputs: HashMap::new(),
            };
        };
        if let Err(err) = processing_graph.connect(from, &conn.from_port, to, &conn.to_port) {
            return LoadGraphResult {
                success: false,
                errors: vec![format!("Connection error: {err}")],
                outputs: HashMap::new(),
            };
        }
    }

    let validation = ValidationPipeline::default().validate(&processing_graph);
    if !validation.errors.is_empty() {
        return LoadGraphResult {
            success: false,
            errors: validation.errors.iter().map(ToString::to_string).collect(),
            outputs: HashMap::new(),
        };
    }

    if graph.nodes.is_empty() {
        return LoadGraphResult {
            success: true,
            errors: Vec::new(),
            outputs: HashMap::new(),
        };
    }

    let engine = ExecutionEngine::new();
    match engine.execute(&processing_graph, None) {
        Ok(result) => {
            let outputs = result
                .outputs
                .iter()
                .map(|(node_id, values)| {
                    (
                        node_id.to_string(),
                        serde_json::json!({ "outputCount": values.len() }),
                    )
                })
                .collect();

            LoadGraphResult {
                success: true,
                errors: Vec::new(),
                outputs,
            }
        }
        Err(err) => LoadGraphResult {
            success: false,
            errors: vec![format!("Execution failed: {err}")],
            outputs: HashMap::new(),
        },
    }
}

fn parse_dimensions(s: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = s.split('x').collect();
    if parts.len() == 2 {
        let w = parts[0].parse().ok()?;
        let h = parts[1].parse().ok()?;
        Some((w, h))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_list_json_contains_filters() {
        let registry = FilterRegistry::with_builtins();
        assert!(registry.len() > 5);
        assert!(registry.contains("load_image"));
    }

    #[test]
    fn cli_validate_empty_graph() {
        let registry = FilterRegistry::with_builtins();
        let graph = SerializedGraph {
            version: "1.0.0".to_string(),
            metadata: ambara::graph::structure::GraphMetadata::default(),
            nodes: vec![],
            connections: vec![],
        };
        let errors = validate_serialized_graph(&graph, &registry);
        assert!(errors.is_empty());
    }
}
