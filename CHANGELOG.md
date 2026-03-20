# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.9.0] - 2026-03-21

### Added
- 8 ComfyUI workflow nodes in `src/filters/builtin/comfyui.rs`:
  - `comfy_checkpoint_loader` — load SD checkpoints through ComfyUI
  - `comfy_clip_text_encode` — CLIP text encoding
  - `comfy_ksampler` — full KSampler with sampler/scheduler/CFG control
  - `comfy_vae_decode` — decode latent images to pixels
  - `comfy_lora_loader` — load LoRA models with strength control
  - `comfy_image_upscale` — model-based image upscaling (RealESRGAN, etc.)
  - `comfy_controlnet_apply` — apply ControlNet to conditioning
  - `comfy_workflow_runner` — run arbitrary ComfyUI workflow JSON
- Groq API backend in `chatbot/generation/llm_client.py` (auto-selected when `GROQ_API_KEY` is set).
- Docker Compose configuration (`docker-compose.yml`) with:
  - Ollama service with NVIDIA GPU passthrough
  - Auto-pull of `qwen2.5:7b` (optimised for RTX 4050 / 6 GB VRAM)
  - Chatbot sidecar service
- `Dockerfile.chatbot` for containerised chatbot deployment.
- `chatbot/requirements.txt` for Python dependency management.
- `.env.example` documenting all environment variables.

### Changed
- `src/filters/builtin/mod.rs` now registers the `comfyui` module.
- `chatbot/api/main.py` runtime config endpoint now supports Groq API key updates.
- LLM auto-selection priority: Anthropic → Groq → OpenAI → Ollama.

## [0.8.0] - 2026-03-20

### Added
- Code-as-RAG retrieval via `chatbot/retrieval/code_retriever.py`, which parses Rust filter source directly instead of relying on embedding-only metadata.
- Agentic tool layer in `chatbot/generation/tools.py` with structured tool schemas and executor functions:
  - `search_filters`, `get_filter_details`, `list_categories`, `get_compatible_filters`, `generate_graph`, `explain_filter`, `suggest_pipeline`, `explain_graph`.
- LLM-driven agent router in `chatbot/generation/agent.py` replacing keyword intent routing for chat behavior.
- New API/model filters in `src/filters/builtin/api.rs`:
  - `http_image_fetch`
  - `stable_diffusion_generate`
  - `image_classify`
  - `model_inference`
  - `style_transfer`
- New Rust filter category `Api` in `src/core/node.rs`.
- Test coverage for code-as-RAG, tools, and agent mock-mode in `chatbot/tests/test_code_retriever.py`.

### Changed
- `chatbot/api/main.py` now routes `/chat` through the new agent and uses `CodeRetriever` for filter listing/search.
- `chatbot/generation/graph_generator.py` now consumes `CodeRetriever` as the primary filter source and builds a dynamic planning catalog from live source data.
- `chatbot/generation/planner.py` now supports dynamic catalog injection for up-to-date planning prompts.
- `src/filters/builtin/mod.rs` now registers the new `api` module.
- `Cargo.toml` now includes `ureq` for blocking HTTP API filters.

### Notes
- Python runtime in this environment is currently broken (`encodings` module missing), so pytest execution could not be completed here. Rust compile validation (`cargo check`) succeeded after the new filter integration.

## [0.7.1] - 2026-03-19

### Added
- 17 new filter implementations across 7 categories (91 total filters, up from 63):
  - **Sharpen**: `unsharp_mask` (sigma/amount/threshold), `sharpen` (3×3 kernel convolution).
  - **Edge**: `edge_detect` (Sobel/Prewitt with invert), `emboss` (directional relief, 4 directions).
  - **Noise**: `add_noise` (Gaussian/salt-and-pepper), `denoise` (median filter).
  - **Draw**: `draw_rectangle`, `draw_circle`, `draw_line` (filled/outline, RGB colors).
  - **Text**: `text_overlay` (built-in 8×13 bitmap font, configurable position/scale/color).
  - **Color**: `sepia`, `hue_rotate`, `threshold`, `posterize`.
  - **Adjust**: `gamma` (gamma correction with LUT), `color_balance` (independent RGB channel multipliers).
- New Rust source files: `sharpen.rs`, `edge.rs`, `noise.rs`, `draw.rs`, `text.rs`.
- CSS rules for all 16 category variants (sharpen, edge, noise, draw, text, adjust, custom).

### Fixed
- UI `categoryColors` in FilterNode.tsx now uses correct Rust `Category` enum names (`Input` not `Source`, `Blur` not `Filter`, `Analyze` not `Analysis`).
- All 16 node categories now display correct header and border colors in the graph editor.

### Changed
- Chatbot `FILTER_CATALOG` updated with all new filters and categories.
- Deterministic fallback keywords expanded for sharpen, edge, noise, draw, text, sepia, gamma, etc.
- ChromaDB embeddings rebuilt with 91 filters (was 63).
- Filter corpus, ID set, and registry snapshot regenerated.

## [0.7.0] - 2026-03-19

### Added
- Multi-stage agentic pipeline for graph generation (Plan → Select → Connect → Validate+Repair).
- `chatbot/generation/planner.py` — Stage 1: decomposes user queries into ordered processing steps.
- `chatbot/generation/selector.py` — Stage 2: selects best filter per step using compact filter cards.
- `chatbot/generation/connector.py` — Stage 3: deterministic graph wiring with port-type compatibility.
- `papers/` directory with 7 research summaries documenting design rationale.
- qwen3 `<think>` tag stripping for plan/selection parsing.
- Regex-based parameter inference for dimensions, opacity, angles.
- Query-order-preserving keyword matching in deterministic fallback.

### Changed
- Rewrote `chatbot/generation/graph_generator.py` with multi-stage orchestration (old version backed up as `graph_generator_legacy.py`).
- API version bumped to 0.7.0.
- Updated repair loop tests for new architecture semantics.

### Fixed
- Astrophotography pipelines now correctly use `load_folder` for stacking workflows.
- "resize"/"resizing" keyword matching now works for both forms.
- Branch examples use realistic overlay pattern instead of unreachable multi-branch topology.

### Verified
- Python chatbot test suite: 28 passed.
- Rust library tests: 121 passed (111 + 2 + 8), 0 failed.

## [0.5.0] - 2026-03-16

### Fixed
- Chatbot responses now use query-aware retrieval and graph construction instead of returning the same default pipeline for unrelated requests.
- Conversational replies now surface relevant filters for user questions instead of generic fallback text.
- Graph generation now avoids incompatible batch/image port combinations, producing valid graphs in offline fallback mode.
- Chatbot backend now uses real LLM backends when configured, while preserving deterministic local fallback behavior.

### Verified
- Python chatbot test suite: 25 passed.
- Rust library tests: 111 passed, 2 ignored.

## [0.4.0] - 2026-03-16

### Added
- Chatbot integration sidecar under `chatbot/` with corpus extraction, retrieval, generation, validation, and FastAPI endpoints.
- Autonomous tooling scripts: `scripts/probe_env.sh`, `scripts/screenshotter.py`, `scripts/auto_loop.py`, and `scripts/verify_completion.sh`.
- New React chat UI components and hook:
  - `ui/src/components/chat/ChatPanel.tsx`
  - `ui/src/components/chat/GraphPreviewCard.tsx`
  - `ui/src/hooks/useChatApi.ts`
- Chat UI tests with Vitest and Testing Library.
- New documentation:
  - `docs/chatbot-system.md`
  - `docs/chatbot-quickstart.md`
  - `docs/chatbot-dacp.md`

### Changed
- Extended Rust CLI (`src/main.rs`) with:
  - `list --json`
  - `load-graph <file> --dry-run`
  - `load-graph <file> --execute`
- Added Vitest `jsdom` test environment in `ui/vite.config.ts`.

### Fixed
- Graph validation now handles malformed JSON gracefully in repair-loop scenarios.
- Retriever embedding path now caches model loading and uses deterministic fallback embedding for faster, more reliable test execution.

## [0.1.2] - 2026-01-04

### Added
- **Batch Save Images Node**: Save multiple images at once with auto-incrementing filenames
  - Configurable filename prefix and padding (e.g., image_001.png, image_002.png)
  - Support for PNG, JPG, WebP, BMP, TIFF formats
  - Quality control for lossy formats
  - Returns array of saved paths and count

### Changed
- **Enhanced Node Colors**: Nodes now have distinct solid background colors by category for better minimap visibility
  - Input/Source: Green (#2d4a2d)
  - Output: Red (#4a2d2d)
  - Transform: Blue (#2d3a4a)
  - Color: Pink (#4a2d3d)
  - Blur/Filter: Purple (#3d2d4a)
  - Utility: Gray (#35414a)
  - Math: Cyan (#2d3d4a)
  - Composite/Analysis: Orange (#4a3d2d)
- **Increased Zoom Range**: Zoom out capability increased 10x (minZoom: 0.05, maxZoom: 4)
- **Load Folder**: Parameter renamed from "path" to "directory" for better directory picker integration
- **Output Value Display**: Non-image output values now display directly in nodes after execution
  - Numbers, booleans, strings, and arrays shown with formatted values
  - Green badges appear next to output ports
  - Images excluded to avoid clutter
- **Version Format**: Changed to numeric versioning (0.1.2) for MSI build compatibility

### Fixed
- Minimap colors now correctly match node category colors
- MSI bundler compatibility with numeric-only version format

### Improved
- Minimap node visibility with better stroke styling and matching colors
- Preview node background colors
- Directory selection in UI for folder-based operations

## [0.1.0-alpha.1] - 2026-01-03

### Added

#### Core Features
- Node-based image processing library with ComfyUI-style visual editor
- 28+ built-in filters across 16 categories
- Type-safe node connections with automatic validation
- Parallel execution engine for batch processing
- Graph serialization (save/load workflows)

#### Astrophotography Filters
- **Image Stack**: Combine multiple images using mean, median, sigma-clip, max, or min algorithms
- **Dark Frame Subtract**: Remove thermal noise using dark frame calibration
- **Flat Field Correct**: Remove vignetting and dust artifacts
- **Hot Pixel Removal**: Detect and remove hot/dead pixels using median filtering
- **Histogram Stretch**: Enhance faint details with adjustable black point, white point, and midtone

#### Image Preview
- **Image Preview Node**: Display thumbnails within the node graph
- Base64-encoded preview generation
- Collapsible preview area
- Shows original image dimensions

#### UI Features
- ReactFlow-based node editor
- Filter palette with search functionality
- Properties panel for parameter editing
- Connection management (auto-replace duplicate inputs)
- Edge deletion (Backspace/Delete keys)
- Clear graph button with confirmation
- File/directory dialogs for I/O operations

#### Filter Categories
- **Input**: LoadImage, LoadFolder
- **Output**: SaveImage (with directory, filename, format options)
- **Transform**: Resize, Rotate, Flip, Crop
- **Adjust**: Brightness, Contrast, Saturation
- **Blur**: GaussianBlur, BoxBlur
- **Sharpen**: Sharpen, UnsharpMask
- **Edge**: EdgeDetect, Sobel
- **Noise**: AddNoise, Denoise
- **Draw**: DrawRectangle, DrawCircle, DrawLine, DrawText
- **Text**: TextOverlay
- **Composite**: Blend, Overlay (with multiple blend modes)
- **Color**: Grayscale, Invert, HueShift, ColorBalance, Threshold
- **Analyze**: Histogram, ImageInfo
- **Math**: Add, Subtract, Multiply, Divide, Modulo, Power, Min, Max, Clamp
- **Utility**: Preview, SplitChannels, MergeChannels, Note, ImagePreview
- **Custom**: Astrophotography filters

#### Developer Features
- Comprehensive test suite (75+ tests passing)
- FilterRegistry for extensibility
- Strong typing with Rust's type system
- Documentation and examples

### Technical Details
- Rust 2021 edition
- Tauri 2.x for desktop UI
- React 19 + TypeScript
- ReactFlow for graph visualization
- Zustand for state management

### Known Limitations
- Preview nodes require execution to display thumbnails
- Sequential execution mode (parallel mode available but disabled by default)
- No undo/redo support yet

[0.1.0-alpha.1]: https://github.com/PrakyathPNayak/ambara/releases/tag/v0.1.0-alpha.1
