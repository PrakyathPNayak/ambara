# Copilot Instructions for Ambara

Ambara is a node-based image-processing engine in Rust with a Tauri + React desktop UI and a Python (FastAPI) chatbot sidecar that generates graphs from natural language. The repo is a Cargo workspace with three members: the root `ambara` crate, `plugins/comfyui_bridge`, and `ui/src-tauri`.

## Build / Test / Lint

Rust (root crate, library + `ambara` CLI):
- Build: `cargo build` (release: `cargo build --release`)
- All tests: `cargo test` (use `-- --nocapture` for stdout)
- Single test: `cargo test <test_name>` or by module path `cargo test graph::tests` / `cargo test --doc`
- Lint: `cargo clippy -- -D warnings`  •  Format: `cargo fmt` (must pass `cargo fmt -- --check` before PR)

UI (`ui/`, requires Node 20+ for `tauri dev`):
- Install: `cd ui && npm install`
- Dev (web only): `npm run dev`  •  Desktop dev: `npm run tauri dev`  •  Build: `npm run tauri build`
- Tests: `npm test` (Vitest); single test file: `npx vitest run path/to/file.test.tsx`

Chatbot (`chatbot/`, Python):
- Start API: `bash chatbot/api/startup.sh` (FastAPI on `http://127.0.0.1:8765`, health: `/health`)
- All tests: `pytest chatbot/tests`  •  Single test: `pytest chatbot/tests/test_generation.py::test_name`

Full stack launcher (boots chatbot + UI + Tauri together): `./tauri-ui` from repo root.

CI (`.github/workflows/build-release.yml`) only runs `npm run tauri build` on tag/dispatch — there is no PR test workflow, so run `cargo test`, `cargo clippy`, and `pytest chatbot/tests` locally.

## Architecture (Big Picture)

Three layers that you almost always need to understand together:

1. **Rust core (`src/`)** — graph engine. Flow: `FilterRegistry` (creates filter instances by id) → `ProcessingGraph` (`src/graph/structure.rs`, nodes + connections) → `ValidationPipeline` (`src/validation/`, multi-stage: structural / type / constraint / custom / resource) → `ExecutionEngine` (`src/execution/engine.rs`, topo-sorted via `TopologyAnalyzer`, parallel batches via Rayon, LRU result cache in `execution/cache.rs`). The central abstraction is the `FilterNode` trait in `src/core/node.rs` (`metadata` / `validate` / `execute` on an `ExecutionContext`).

2. **Tauri backend (`ui/src-tauri/src/lib.rs`)** — bridges the UI to the core. Defines `#[tauri::command]` functions (`get_filters`, `validate_graph`, `execute_graph`, `save_graph`/`load_graph`, plugin commands, `export_graph_json`/`import_graph_json`, `get_external_api_capabilities`). It owns "bridge" structs that mirror frontend types and converts the UI's `GraphState` into `ProcessingGraph`. Depends on the root crate via `path = "../.."`.

3. **React UI (`ui/src/`)** — `App.tsx` is the shell; `@xyflow/react` canvas in `components/canvas/GraphCanvas.tsx`; Zustand store `store/graphStore.ts` owns nodes/edges; all backend calls go through the thin wrapper `ui/src/api/commands.ts` (Tauri `invoke`). Shared TS types in `ui/src/types/index.ts` mirror Rust port/value types.

4. **Python chatbot (`chatbot/`)** — separate process, talks HTTP to the UI. Generation pipeline is **Plan → Select → Connect → Validate+Repair**: `generation/planner.py` (LLM, decomposes query), `generation/selector.py` (LLM, picks filter per step from top-5 candidates), `generation/connector.py` (deterministic, no LLM — wires ports by type compatibility), orchestrated by `generation/graph_generator.py` with keyword-fallback. `retrieval/code_retriever.py` parses filter metadata directly from Rust source as the primary retrieval path. LLM backend is auto-selected: Anthropic → Groq → OpenAI → Ollama (see `.env.example`).

Two distinct serialization formats coexist: the Rust core's `SerializedGraph` (`src/graph/serialization.rs`) and the UI's `GraphState` JSON. `import_graph_json` accepts either an envelope payload or raw `GraphState`. Don't confuse them.

## Project-Specific Conventions

- **Adding a filter**: implement `FilterNode` in a new file under `src/filters/builtin/<topic>.rs`, then register it from that file's `register(registry)` and wire it into `src/filters/builtin/mod.rs::register_all`. Always use `NodeMetadata::builder(...)` + `PortDefinition` + `ParameterDefinition` (with `Constraint` and `UiHint`) — these drive the auto-generated UI properties panel; do not hand-roll metadata.
- **Filter ids are stable strings** (e.g. `"gaussian_blur"`, `"load_image"`). The chatbot's filter whitelist (`chatbot/generation/graph_validator.py`) and code retriever depend on these ids, so renaming a filter id is a cross-cutting change across Rust + Python.
- **`Value` enum (`src/core/types.rs`)** is the single data currency between nodes (`Image`, `Integer`, `Float`, `String`, `Boolean`, `Color`, vectors, arrays, maps, `None`). Use the typed accessors (`as_image`, `as_float`, …) instead of pattern-matching ad hoc.
- **Errors**: library code returns `Result` with the typed errors from `src/core/error.rs` (`AmbaraError` / `GraphError` / `ValidationError` / `ExecutionError` / `PluginError` / `BatchError`). No `panic!` in library paths; tests may `unwrap`.
- **Batch / array nodes**: filters that should handle both single images and arrays implement `BatchAware` (`src/core/batch.rs`) and use the helpers there rather than re-checking `Value` shapes.
- **GPU code** (`src/core/gpu.rs`, `wgpu`): the `GpuAccelerated` trait + global `GpuPool` is scaffolding — most filters are CPU/Rayon. Don't assume a real GPU path exists for a given filter without checking.
- **Plugin system**: dynamic `.so/.dll` plugins loaded via `libloading`; manifests are `ambara-plugin.toml` (parsed by `src/plugins/manifest.rs`). The in-tree example is `plugins/comfyui_bridge` (a workspace member).
- **UI graph mutations** go through `graphStore.ts` actions (which encode rules like "auto-replace an existing input connection"); don't mutate ReactFlow nodes/edges directly from components.
- **Chatbot LLM output handling**: planner/selector strip qwen3 `<think>...</think>` tags before parsing — preserve this when changing prompts or adding new LLM stages.
- **Commits**: Conventional Commits (`feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `perf:`, `chore:`) per `CONTRIBUTING.md`.

## Useful Pointers

- `REPO_FILE_GUIDE.md` — exhaustive per-file map; use it before grepping the codebase.
- `docs/external-api.md`, `docs/chatbot-system.md`, `docs/chatbot-dacp.md` — design docs for the Tauri command API and chatbot pipeline.
- `checkpoint.py` — TODO-checkpoint helper used by the project's agent workflow (writes `checkpoint_log.json`); not part of the runtime.
