# NOTE: PERSONAL PROJECT WITH HIGH AMOUNTS OF "VIBECODING"
This project was done with the intent of testing the ease of developement with the new, arising AI tools and get some useful software out of it. 


# Ambara

**Ambara** is a high-performance, node-based image processing library written in Rust, with a ComfyUI-inspired visual editor built on Tauri.

## Features

- 🎨 **Node-Based Workflow**: Build complex image processing pipelines with an intuitive graph-based approach
- 🚀 **High Performance**: Built with Rust for maximum speed and memory efficiency
- 🖼️ **Rich Filter Library**: Comprehensive set of built-in filters and operations
- 🔌 **Extensible Architecture**: Easy to add custom filters and operations
- 💻 **Cross-Platform**: Works on Linux, macOS, and Windows
- 🎯 **Type-Safe**: Strong type checking for node connections and parameters
- 🔄 **Parallel Execution**: Automatic parallelization of independent operations
- 🧪 **Well-Tested**: Comprehensive test suite with 72+ tests

## Quick Start

### Prerequisites

- Rust 1.70 or higher
- For UI: GTK3 development libraries (Linux), WebKit (macOS), WebView2 (Windows)

### Building the Library

```bash
# Clone the repository
git clone https://github.com/yourusername/ambara.git
cd ambara

# Build the library
cargo build --release

# Run tests
cargo test
```

### Running the UI

```bash
# Navigate to the UI directory
cd ui

# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

## Architecture

Ambara uses a graph-based architecture where:

1. **Nodes** represent operations (filters, I/O, utilities)
2. **Edges** represent data flow between nodes
3. **Ports** are typed inputs and outputs on nodes
4. **Values** are the data passed between nodes

### Core Components

- **`FilterNode`**: Trait that all filter nodes implement
- **`Graph`**: Represents the processing pipeline
- **`FilterRegistry`**: Central registry of all available filters
- **`ExecutionEngine`**: Executes the graph with parallel processing
- **`ValidationPipeline`**: Validates graph structure and types

## Built-in Filters

### Image I/O
- **LoadImage**: Load a single image from disk
- **LoadFolder**: Batch load images from a directory
- **SaveImage**: Save processed images to disk

### Blur Operations
- **GaussianBlur**: Apply Gaussian blur with specified radius
- **BoxBlur**: Apply box blur (faster alternative)

### Color Operations
- **Brightness**: Adjust image brightness
- **Contrast**: Adjust image contrast
- **Saturation**: Adjust color saturation
- **Grayscale**: Convert to grayscale
- **Invert**: Invert colors
- **HueShift**: Rotate hue values

### Transform Operations
- **Resize**: Resize images with various algorithms
- **Rotate**: Rotate images by specified angle
- **Flip**: Flip horizontally or vertically
- **Crop**: Extract a region from the image

### Composite Operations
- **Blend**: Blend two images with various modes
- **Overlay**: Overlay one image on another
- **Mask**: Apply alpha mask

### Math Operations
- **Add, Subtract, Multiply, Divide**: Basic arithmetic
- **Modulo, Power**: Advanced math operations
- **Min, Max, Clamp**: Value constraints

### Comparison & Logic
- **Equal, NotEqual, LessThan, GreaterThan**: Comparison operations
- **And, Or, Not, Xor**: Boolean logic

### Constant Values
- **Integer, Float, String, Boolean, Color**: Provide constant values

### Type Conversion
- **ToInteger, ToFloat, ToString, ToBoolean**: Convert between types

### Utilities
- **Passthrough**: Pass data without modification
- **Copy**: Duplicate data to multiple outputs
- **Debug**: Print debug information

## Usage Examples

### Programmatic Usage

```rust
use ambara::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a graph
    let mut graph = Graph::new();
    
    // Add nodes
    let load_id = graph.add_node("load_image")?;
    let blur_id = graph.add_node("gaussian_blur")?;
    let save_id = graph.add_node("save_image")?;
    
    // Set parameters
    graph.set_parameter(load_id, "path", Value::String("input.png".to_string()))?;
    graph.set_parameter(blur_id, "radius", Value::Float(5.0))?;
    graph.set_parameter(save_id, "path", Value::String("output.png".to_string()))?;
    
    // Connect nodes
    graph.connect(load_id, "image", blur_id, "input")?;
    graph.connect(blur_id, "output", save_id, "image")?;
    
    // Execute
    let engine = ExecutionEngine::new();
    engine.execute(&graph)?;
    
    Ok(())
}
```

### Using the Visual Editor

1. Launch the UI application
2. Drag filters from the palette onto the canvas
3. Configure filter parameters in the properties panel
4. Connect filters by dragging from output ports to input ports
5. Execute the graph to process images
6. Save/load graphs as JSON for reuse

## Creating Custom Filters

```rust
use ambara::prelude::*;

#[derive(Debug, Clone)]
pub struct MyCustomFilter;

impl FilterNode for MyCustomFilter {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("my_filter", "My Custom Filter")
            .description("Does something amazing")
            .category(Category::Custom)
            .input(
                PortDefinition::input("input", PortType::Image)
                    .with_description("Input image")
            )
            .output(
                PortDefinition::output("output", PortType::Image)
                    .with_description("Processed image")
            )
            .parameter(
                ParameterDefinition::float("intensity")
                    .with_default(1.0)
                    .with_range(0.0, 2.0)
            )
            .build()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        // Validate inputs and parameters
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        // Get inputs
        let input = ctx.get_input_image("input")?;
        let intensity = ctx.get_float("intensity")?;
        
        // Process image
        let output = process_image(input, intensity)?;
        
        // Set output
        ctx.set_output("output", Value::Image(output))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

// Register your filter
fn register(registry: &mut FilterRegistry) {
    registry.register(|| Box::new(MyCustomFilter));
}
```

## Performance

Ambara is designed for high performance:

- **Parallel Execution**: Automatically executes independent nodes in parallel using Rayon
- **Zero-Copy**: Minimizes data copying where possible
- **Efficient Memory**: Uses reference counting for large images
- **SIMD**: Leverages SIMD instructions for image processing (via imageproc)

## API Documentation

Generate and view the full API documentation:

```bash
cargo doc --no-deps --open
```

## Testing

```bash
# Run all tests
cargo test

# Run specific test module
cargo test graph::tests

# Run with output
cargo test -- --nocapture
```

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Guidelines

- Follow Rust naming conventions and idioms
- Add tests for new functionality
- Document public APIs with doc comments
- Run `cargo fmt` and `cargo clippy` before committing
- Ensure all tests pass with `cargo test`

## Project Structure

```
ambara/
├── src/
│   ├── core/          # Core abstractions (Graph, Node, Port, Value)
│   ├── filters/       # Filter implementations
│   │   ├── builtin/   # Built-in filters
│   │   └── registry.rs
│   ├── execution/     # Execution engine and parallelization
│   ├── validation/    # Graph validation
│   ├── graph/         # Graph data structure and algorithms
│   └── lib.rs
├── ui/                # Tauri + React UI
│   ├── src/           # React components and TypeScript
│   ├── src-tauri/     # Tauri backend (Rust)
│   └── package.json
├── tests/             # Integration tests
├── examples/          # Usage examples
└── Cargo.toml
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Inspired by [ComfyUI](https://github.com/comfyanonymous/ComfyUI)
- Built with [image-rs](https://github.com/image-rs/image)
- UI powered by [Tauri](https://tauri.app/) and [ReactFlow](https://reactflow.dev/)

## Roadmap

- [ ] GPU acceleration support
- [ ] More advanced filters (ML-based, style transfer)
- [ ] Animation/video processing
- [ ] Python bindings
- [ ] Plugin system for community filters
- [ ] Cloud execution support
- [ ] Real-time preview
- [ ] Undo/redo in UI

## Support

For questions, issues, or suggestions:
- Open an issue on [GitHub](https://github.com/yourusername/ambara/issues)
- Join our [Discord server](#)
- Check the [documentation](#)

---

Made with ❤️ in Rust
