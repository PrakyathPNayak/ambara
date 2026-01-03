# Contributing to Ambara

Thank you for your interest in contributing to Ambara! This document provides guidelines and instructions for contributing.

## Code of Conduct

Be respectful, inclusive, and constructive in all interactions. We're here to build great software together.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/yourusername/ambara.git
   cd ambara
   ```
3. **Add the upstream remote**:
   ```bash
   git remote add upstream https://github.com/originalowner/ambara.git
   ```
4. **Create a branch** for your feature:
   ```bash
   git checkout -b feature/your-feature-name
   ```

## Development Setup

### Prerequisites

- Rust 1.70 or higher
- Node.js 16+ and npm (for UI development)
- Git

### Build and Test

```bash
# Build the library
cargo build

# Run tests
cargo test

# Run clippy (linter)
cargo clippy -- -D warnings

# Format code
cargo fmt

# Build UI
cd ui && npm install && npm run tauri dev
```

## Coding Standards

### Rust Code

- **Format**: Use `cargo fmt` with default settings
- **Linting**: Code must pass `cargo clippy` with no warnings
- **Naming**: Follow Rust API guidelines (RFC 199)
  - Types: `UpperCamelCase`
  - Functions/variables: `snake_case`
  - Constants: `SCREAMING_SNAKE_CASE`
- **Documentation**: All public APIs must have doc comments
- **Error Handling**: Use `Result` types, avoid panics in library code
- **Tests**: Add tests for all new functionality

### Example: Good Code Style

```rust
/// Applies Gaussian blur to an image.
///
/// # Arguments
///
/// * `image` - The input image to blur
/// * `radius` - The blur radius in pixels
///
/// # Returns
///
/// Returns the blurred image or an error if processing fails.
pub fn gaussian_blur(image: &DynamicImage, radius: f32) -> Result<DynamicImage, Error> {
    // Implementation
}
```

### TypeScript Code (UI)

- **Format**: Use Prettier (configured in UI package.json)
- **Linting**: Use ESLint (configured in UI)
- **Types**: Avoid `any`, prefer explicit types
- **Components**: Use functional components with hooks
- **State**: Use Zustand for global state

## Adding New Filters

To add a new filter node:

1. Create a new file in `src/filters/builtin/` or create a new module
2. Implement the `FilterNode` trait
3. Register the filter in `mod.rs`
4. Add tests
5. Update documentation

### Filter Template

```rust
use crate::core::prelude::*;

/// Brief description of what this filter does.
#[derive(Debug, Clone)]
pub struct MyFilter;

impl FilterNode for MyFilter {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("my_filter", "My Filter")
            .description("Detailed description")
            .category(Category::YourCategory)
            .input(
                PortDefinition::input("input", PortType::Image)
                    .with_description("Input image")
            )
            .output(
                PortDefinition::output("output", PortType::Image)
                    .with_description("Output image")
            )
            .parameter(
                ParameterDefinition::float("param")
                    .with_default(1.0)
                    .with_range(0.0, 10.0)
                    .with_description("Parameter description")
            )
            .build()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        // Validate inputs and parameters
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let input = ctx.get_input_image("input")?;
        let param = ctx.get_float("param")?;
        
        // Process image
        let output = process(input, param)?;
        
        ctx.set_output("output", Value::Image(output))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_filter() {
        // Test implementation
    }
}
```

## Testing Guidelines

### Unit Tests

- Place tests in the same file as the code using `#[cfg(test)]`
- Test edge cases and error conditions
- Use descriptive test names

### Integration Tests

- Place in `tests/` directory
- Test complete workflows
- Test error handling

### Running Tests

```bash
# All tests
cargo test

# Specific test
cargo test test_name

# With output
cargo test -- --nocapture

# Documentation tests
cargo test --doc
```

## Documentation

### API Documentation

- Use `///` for public item documentation
- Include:
  - Brief description
  - `# Arguments` section
  - `# Returns` section
  - `# Errors` section (if applicable)
  - `# Examples` section (when helpful)
  - `# Panics` section (if applicable)

### Building Documentation

```bash
cargo doc --no-deps --open
```

## Pull Request Process

1. **Update your branch** with latest upstream:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Ensure all checks pass**:
   ```bash
   cargo test
   cargo clippy
   cargo fmt -- --check
   ```

3. **Write a clear commit message**:
   ```
   feat: Add Gaussian blur filter
   
   - Implements FilterNode trait
   - Adds configurable radius parameter
   - Includes unit tests
   - Updates documentation
   ```

4. **Push to your fork**:
   ```bash
   git push origin feature/your-feature-name
   ```

5. **Create Pull Request**:
   - Provide clear description of changes
   - Reference related issues
   - Add screenshots for UI changes
   - Ensure CI passes

### Commit Message Format

Use conventional commits:

- `feat:` New feature
- `fix:` Bug fix
- `docs:` Documentation changes
- `test:` Test additions/changes
- `refactor:` Code refactoring
- `perf:` Performance improvements
- `chore:` Build/tooling changes

## Areas for Contribution

### High Priority

- New filter implementations
- Performance optimizations
- Bug fixes
- Documentation improvements
- Test coverage

### Medium Priority

- UI/UX improvements
- Example projects
- Tutorial content
- Platform-specific optimizations

### Future

- GPU acceleration
- ML-based filters
- Python bindings
- Plugin system

## Getting Help

- **Questions**: Open a discussion on GitHub
- **Bugs**: Open an issue with reproduction steps
- **Features**: Open an issue to discuss before implementing

## Performance Guidelines

- Profile before optimizing
- Use `criterion` for benchmarks
- Consider memory allocations
- Leverage parallelism when appropriate
- Document performance characteristics

## Review Process

1. Maintainers review code and provide feedback
2. Address review comments
3. Once approved, code is merged
4. CI runs final checks
5. Your contribution is live!

## License

By contributing, you agree that your contributions will be licensed under the same MIT License that covers the project.

## Recognition

Contributors are recognized in:
- GitHub contributors page
- CHANGELOG.md for significant contributions
- README.md for major features

Thank you for contributing to Ambara! 🎨🚀
