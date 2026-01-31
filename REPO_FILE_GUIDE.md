# Ambara Repository File Guide

This document explains the purpose of the most important files and modules in this repository.
It focuses on the “source of truth” code and configuration that defines Ambara’s behavior.

> Note: build artifacts and generated directories (e.g. `target/`, `ui/node_modules/`, `ui/dist/`, `ui/src-tauri/target/`) are intentionally not documented in detail.

## High-level architecture (how the pieces fit)

Ambara is split into two main parts:

- **Rust library + CLI** (root crate): a graph-based image-processing engine.
- **Tauri + React UI** (`ui/`): a desktop node editor (ComfyUI-like) that calls into a Tauri backend.

The usual flow is:

1. **UI** builds a node graph (nodes + edges).
2. **Tauri backend** converts that UI graph into an `ambara::graph::structure::ProcessingGraph`.
3. **Validation** checks structure/types/constraints.
4. **Execution** runs nodes in topological order (optionally parallel) and returns outputs.

---

## Root (repository-level) files

### Cargo manifests and lockfiles

- `Cargo.toml`
  - Root Rust package metadata.
  - Defines `ambara` as both:
    - a **library crate** (`src/lib.rs`) and
    - a **binary/CLI** (`src/main.rs`).
  - Declares dependencies for image processing (`image`, `imageproc`), graph algorithms (`petgraph`, `indexmap`), parallel execution (`rayon`), serialization (`serde`, `serde_json`), caching (`lru`), etc.

- `Cargo.lock`
  - Locked Rust dependency graph (reproducible builds).

### Project documentation

- `README.md`
  - Primary project overview: features, architecture summary, quick start, custom filter example.

- `CHANGELOG.md`
  - Keep-a-Changelog formatted change history.
  - Useful for understanding recent features (e.g. node category colors, batch save node).

- `RELEASE_NOTES.md`
  - Human-friendly release summary for the latest version.

- `CONTRIBUTING.md`
  - Dev workflow, code style expectations, and how to add new filters.

- `LICENSE`
  - Project license.

### Top-level directories

- `src/`
  - The core Rust library and CLI.

- `ui/`
  - React/Vite frontend and Tauri app.

- `target/`
  - Rust build output (generated).

---

## Rust crate (library + CLI): `src/`

### Entry points

- `src/lib.rs`
  - The **library crate root**.
  - Exposes modules: `core`, `execution`, `filters`, `graph`, `validation`.
  - Defines a `prelude` with the commonly used public types (graph types, errors, registry, engine, etc.).
  - Declares `VERSION` and `NAME` constants.
  - Contains basic unit tests that verify the crate wiring and a minimal graph.

- `src/main.rs`
  - A **demo CLI** for the library.
  - Supports commands:
    - `list` (print available filters by category)
    - `info <filter_id>` (dump ports/parameters)
    - `process <in> <out> [options]` (builds a small graph and executes it)
  - Demonstrates how to:
    - create a `FilterRegistry`,
    - build a `ProcessingGraph`,
    - validate with `ValidationPipeline`,
    - execute with `ExecutionEngine`.

---

## Core library modules: `src/core/`

This folder contains the fundamental types that everything else depends on.

- `src/core/mod.rs`
  - Module index and re-exports for core types.
  - Intended to keep import paths ergonomic.

- `src/core/types.rs`
  - Defines the core data model that flows through the graph:
    - `Value` enum (`Image`, `Integer`, `Float`, `String`, `Boolean`, `Color`, vectors, arrays, maps, `None`).
    - `ImageValue`, `ImageMetadata`, `ImageDataRef`.
    - `PortType` (type system for ports; supports `Any`, arrays, maps).
  - Includes helper methods like `Value::get_type()` and typed accessors (`as_float`, `as_image`, …).

- `src/core/node.rs`
  - Defines:
    - `Category` enum: how nodes are grouped in the UI (Input/Output/Transform/…)
    - `NodeMetadata`: node name/id, ports, parameters, tags, UI hints.
    - `NodeMetadataBuilder`: fluent builder used throughout built-in filters.
    - The `FilterNode` trait (the central abstraction):
      - `metadata()` (ports + parameters + UI info)
      - `validate()` (pre-flight validation)
      - `execute()` (runs on an `ExecutionContext`)

- `src/core/port.rs`
  - Defines port and parameter schemas:
    - `PortDefinition` (name/type/optional/default/constraints)
    - `ParameterDefinition` (similar, but used for UI-edited node parameters)
    - `Constraint` enum (range, length, patterns, image constraints, one-of, custom closures)
    - `UiHint` enum (slider/dropdown/file chooser/etc.)
  - This is the “form language” for auto-generating property panels.

- `src/core/context.rs`
  - Runtime/validation contexts:
    - `ValidationContext`: used by `FilterNode::validate`.
    - `ExecutionContext`: used by `FilterNode::execute`.
  - Provides typed getters for inputs/parameters and manages node outputs.

- `src/core/error.rs`
  - Central error types:
    - `AmbaraError` (top-level), and sub-errors `GraphError`, `ValidationError`, `ExecutionError`, `PluginError`, `BatchError`.
    - `NodeId`, `ConnectionId` (UUID-backed identifiers used throughout graph/execution).
  - These errors are designed to be actionable (pointing to node/port/parameter when possible).

- `src/core/batch.rs`
  - Batch processing support:
    - `BatchSize`, `BatchMode`, `BatchContext`.
    - `BatchAware` trait for filters that can handle batches efficiently.
    - Utilities to extract images from `Value` (single image vs image arrays).

- `src/core/gpu.rs`
  - GPU acceleration scaffolding:
    - `GpuDevice`, `GpuBackend`, `GpuAccelerated` trait, and a global `GpuPool`.
  - Currently structured as an abstraction layer and placeholder for future real `wgpu` integration.

---

## Graph data structures: `src/graph/`

- `src/graph/mod.rs`
  - Module index and re-exports.

- `src/graph/structure.rs`
  - Owns the core graph model:
    - `ProcessingGraph`: nodes + connections + metadata.
    - `GraphNode`: node instance (contains the concrete `FilterNode` implementation and parameter overrides).
    - `Position` and `GraphMetadata` for UI placement and graph info.
  - Provides graph operations:
    - adding/removing nodes
    - connecting/disconnecting ports
    - cycle prevention and type compatibility checks.

- `src/graph/connection.rs`
  - Defines `Endpoint` and `Connection` structs.
  - Connections are directed: output endpoint → input endpoint.

- `src/graph/topology.rs`
  - `TopologyAnalyzer`:
    - topological sorting (Kahn’s algorithm)
    - parallel execution batch grouping (by depth)
    - subgraph discovery (connectivity ignoring direction)

- `src/graph/serialization.rs`
  - Serializable graph format:
    - `SerializedGraph`, `SerializedNode`, `SerializedConnection`.
  - Provides `to_json`/`from_json` helpers.
  - This is distinct from UI’s graph state format (the UI saves its own nodes/edges too).

---

## Validation: `src/validation/`

- `src/validation/mod.rs`
  - Module index and re-exports.

- `src/validation/pipeline.rs`
  - `ValidationPipeline`: runs multiple stages and returns a `ValidationReport`.
  - Provides default pipelines:
    - full pipeline (structural + type + constraints + custom + resource)
    - minimal pipeline (structural + type).

- `src/validation/stages.rs`
  - Individual stages implementing `ValidationStage`:
    - `StructuralValidation`: checks required inputs and cycles, warns about disconnected subgraphs.
    - `TypeValidation`: checks port type compatibility for every edge.
    - `ConstraintValidation`: validates parameter values vs constraints.
    - `CustomValidation`: calls each node’s `FilterNode::validate`.
    - `ResourceValidation`: checks paths/globs and output directories.

---

## Execution: `src/execution/`

- `src/execution/mod.rs`
  - Module index and re-exports.

- `src/execution/engine.rs`
  - `ExecutionEngine`: executes a validated graph.
  - Supports:
    - sequential or parallel execution
    - per-node caching
    - cancellation and progress reporting
    - error aggregation vs stop-on-first-error
  - Uses `TopologyAnalyzer` to compute execution order and parallel batches.

- `src/execution/cache.rs`
  - Result caching system:
    - `CacheKey` hashes node inputs (deterministic ordering).
    - `ResultCache` is an LRU cache with TTL and memory accounting.
    - `CacheStats` tracks hit ratio/time saved.

- `src/execution/progress.rs`
  - Progress events and cancellation:
    - `ProgressUpdate` event enum (started, node started/completed/skipped, percent, completed, cancelled, error).
    - `ProgressTracker` manages counters and estimates remaining time.

---

## Filters: `src/filters/`

- `src/filters/mod.rs`
  - Module index and re-exports.

- `src/filters/registry.rs`
  - `FilterRegistry`: stores factories and cached `NodeMetadata`.
  - Key responsibilities:
    - register filters (builtins and custom)
    - create filter instances by id
    - group by category for UI display
    - search by name/description/tags

### Built-in filters: `src/filters/builtin/`

The built-ins are split by topic. Each file typically:

- defines multiple `FilterNode` implementations,
- provides a `register(registry: &mut FilterRegistry)` function, and
- uses `NodeMetadataBuilder` + `PortDefinition` + `ParameterDefinition` for consistent UI behavior.

- `src/filters/builtin/mod.rs`
  - Wires together all builtin modules and exposes `register_all`.
  - Re-exports a subset of builtin node types for convenience.

- `src/filters/builtin/io.rs`
  - Image I/O and disk interaction:
    - `LoadImage`, `LoadFolder`, `SaveImage`, plus batch-saving helpers.
  - Validates paths and file types, and produces `Value::Image` / image arrays.

- `src/filters/builtin/blur.rs`
  - Blur operations:
    - `GaussianBlur`, `BoxBlur`.

- `src/filters/builtin/color.rs`
  - Color adjustments:
    - `Brightness`, `Contrast`, `Saturation`, `Grayscale`, `Invert`.

- `src/filters/builtin/transform.rs`
  - Geometry transforms:
    - `Resize`, `Rotate`, `Flip`, `Crop`.

- `src/filters/builtin/composite.rs`
  - Multi-image compositing:
    - `Blend`, `Overlay` and blend-mode logic.

- `src/filters/builtin/utility.rs`
  - Utility and debugging nodes:
    - pass-through, previews, channel split/merge, notes/info, array utilities, value display.
  - Includes thumbnail generation (base64) used by the UI preview node.

- `src/filters/builtin/constants.rs`
  - Constant-value nodes (integer/float/string/bool/color) for injecting literals.

- `src/filters/builtin/math.rs`
  - Numeric math nodes (add/sub/mul/div/mod/pow/min/max/clamp).

- `src/filters/builtin/comparison.rs`
  - Comparisons and boolean logic (equal/lt/gt/and/or/not/xor, etc.).

- `src/filters/builtin/conversion.rs`
  - Conversion nodes: `ToInteger`, `ToFloat`, `ToString`, `ToBoolean`.

- `src/filters/builtin/astro.rs`
  - Astrophotography-oriented processing:
    - stacking, calibration (dark/flat), hot pixel removal, histogram stretch.

- `src/filters/builtin/batch.rs`
  - Batch-aware filter examples for operating on single images or arrays efficiently.
  - Demonstrates parallelism via Rayon for many operations.

- `src/filters/builtin/array.rs`
  - Array utilities (`ArrayMap`, `ArrayFilter`, etc.) that normalize “single vs array” processing.

---

## UI (React + Vite): `ui/`

### Project/config files

- `ui/package.json`
  - Frontend dependencies:
    - `react`, `@xyflow/react` (graph editor), `zustand` (state).
    - Tauri JS API and plugins.
  - Defines scripts for dev/build and for invoking Tauri.

- `ui/package-lock.json`
  - Locked Node dependency graph (reproducible installs).

- `ui/vite.config.ts`
  - Vite config tuned for Tauri (fixed dev port, no clearScreen, ignores `src-tauri/`).

- `ui/tsconfig.json`, `ui/tsconfig.node.json`
  - TypeScript configuration for React app and tooling.

- `ui/index.html`
  - Single-page app root file; mounts React at `#root`.

- `ui/README.md`
  - Template README from the Tauri/Vite starter.

### Frontend source: `ui/src/`

- `ui/src/main.tsx`
  - React entry point; renders `<App />`.

- `ui/src/App.tsx`
  - App “shell” and coordination:
    - loads filter metadata from the Tauri backend (`get_filters`)
    - manages validate/execute/save/load/clear actions
    - wires together:
      - `FilterPalette` (left)
      - `GraphCanvas` (center)
      - `PropertiesPanel` (right)
      - toast notifications + confirmation modal

- `ui/src/api/commands.ts`
  - Thin wrapper around Tauri `invoke()` commands and dialog plugin helpers.
  - Defines `getFilters`, `validateGraph`, `executeGraph`, `saveGraph`, `loadGraph`.

- `ui/src/store/graphStore.ts`
  - Zustand store that owns the UI graph state:
    - node/edge arrays
    - selection
    - handlers for ReactFlow change events
    - connection rules (e.g. auto-replace existing input connection)

- `ui/src/types/index.ts`
  - Shared TypeScript types for nodes/ports/graph state.
  - Mirrors backend concepts (port types, filter metadata, execution/validation results).

#### Main UI components

- `ui/src/components/canvas/GraphCanvas.tsx`
  - ReactFlow canvas with:
    - minimap, zoom/controls, background grid
    - toolbar actions (validate/execute/save/load/clear)
    - custom node renderers: filter nodes, preview nodes, value display nodes.

- `ui/src/components/sidebar/FilterPalette.tsx`
  - Searchable filter list grouped by category.
  - Calls back into `App` to add nodes.

- `ui/src/components/sidebar/PropertiesPanel.tsx`
  - Shows selected node ports and a parameter editor.
  - Supports file/folder browsing via Tauri dialog plugins.

- `ui/src/components/nodes/FilterNode.tsx`
  - Main node renderer for typical filters.
  - Renders input/output handles and shows non-image output values after execution.

- `ui/src/components/nodes/PreviewNode.tsx`
  - Specialized node renderer for preview/thumbnails.
  - Supports expanding/collapsing the preview image.

- `ui/src/components/nodes/ValueDisplayNode.tsx`
  - A node renderer focused on displaying a typed value.

- `ui/src/components/Toast.tsx` and `ui/src/hooks/useToast.ts`
  - Lightweight toast notification system.

- `ui/src/components/ConfirmDialog.tsx`
  - Reusable modal confirmation dialog (used for “clear graph”).

#### Styling and assets

CSS files generally correspond 1:1 with components and define layout, colors, and the editor theme.
Notable:

- `ui/src/App.css`: global layout and basic app theme.
- `ui/src/components/**.css`: component-level styles.

Other directories:

- `ui/public/`
  - Static assets served directly by Vite.

- `ui/dist/`
  - Frontend build output (generated by `vite build`).

- `ui/node_modules/`
  - Node dependency install output (generated).

---

## Tauri backend (Rust): `ui/src-tauri/`

This is the desktop “host” application. It compiles to a native binary and embeds the web UI.

- `ui/src-tauri/Cargo.toml`
  - Tauri backend crate manifest.
  - Depends on `tauri` + plugins, and depends on the root `ambara` crate via `path = "../.."`.

- `ui/src-tauri/tauri.conf.json`
  - Tauri app configuration:
    - `beforeDevCommand` / `devUrl` for dev mode
    - `beforeBuildCommand` / `frontendDist` for production builds
    - window sizing/title and bundle icons.

- `ui/src-tauri/build.rs`
  - Runs `tauri_build::build()`.

- `ui/src-tauri/src/main.rs`
  - Minimal Tauri entry point that calls `ui_lib::run()`.

- `ui/src-tauri/src/lib.rs`
  - Main backend implementation.
  - Defines Tauri `#[command]` functions used by the frontend:
    - `get_filters`: exposes the real `FilterRegistry` metadata to the UI.
    - `validate_graph`: UI-side validation (basic checks like missing required connections).
    - `execute_graph`: converts UI graph into `ProcessingGraph` and runs `ExecutionEngine`.
    - `save_graph` / `load_graph`: save/load the UI graph state as JSON.
  - Contains “bridge” structs that mirror frontend types (ports, filter info, graph state).

- `ui/src-tauri/icons/`
  - App icons for bundling (PNG/ICO/ICNS).

- `ui/src-tauri/capabilities/`, `ui/src-tauri/gen/`
  - Tauri capability definitions / generated artifacts (tooling-managed).

- `ui/src-tauri/target/`
  - Tauri backend build output (generated).

---

## Not documented (generated / not source-of-truth)

The following are intentionally not described in detail:

- `target/`, `ui/src-tauri/target/`: Rust build artifacts.
- `ui/node_modules/`: npm install artifacts.
- `ui/dist/`: frontend build output.
- Any editor/tooling folders that don’t affect runtime behavior.
