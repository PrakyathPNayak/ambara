//! Ambara CLI - Node-based Image Processing
//!
//! This is a demonstration CLI for the Ambara library.

use ambara::prelude::*;
use std::path::PathBuf;

fn main() {
    println!("🎨 Ambara - Node-based Image Processing v{}", ambara::VERSION);
    println!();

    // Parse command line args
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        print_usage(&args[0]);
        return;
    }

    match args[1].as_str() {
        "list" => list_filters(),
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
                eprintln!("Usage: {} process <input> <output> [--blur <sigma>] [--brightness <amount>]", args[0]);
                return;
            }
            process_image(&args[2..]);
        }
        "help" | "--help" | "-h" => print_usage(&args[0]),
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_usage(&args[0]);
        }
    }
}

fn print_usage(program: &str) {
    println!("Usage: {} <command> [options]", program);
    println!();
    println!("Commands:");
    println!("  list              List all available filters");
    println!("  info <filter>     Show detailed info about a filter");
    println!("  process <in> <out> [options]  Process an image");
    println!("  help              Show this help message");
    println!();
    println!("Process options:");
    println!("  --blur <sigma>      Apply Gaussian blur (default: none)");
    println!("  --brightness <amt>  Adjust brightness -1.0 to 1.0 (default: 0)");
    println!("  --grayscale         Convert to grayscale");
    println!("  --resize <WxH>      Resize to dimensions (e.g., 800x600)");
}

fn list_filters() {
    let registry = FilterRegistry::with_builtins();
    let grouped = registry.grouped_by_category();

    println!("Available filters ({} total):", registry.len());
    println!();

    for (category, filters) in grouped {
        println!("  📁 {:?}", category);
        for metadata in filters {
            println!("      • {} - {}", metadata.id, metadata.description);
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
