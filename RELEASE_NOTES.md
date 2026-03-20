# Ambara v0.9.0 Release Notes

**Release Date:** 21 March 2026

## Highlights

- **8 ComfyUI workflow nodes** — Full ComfyUI integration as native Ambara nodes. Load checkpoints, encode prompts with CLIP, sample with KSampler, decode with VAE, apply LoRAs, upscale, use ControlNets, or run arbitrary ComfyUI workflows.
- **Groq API support** — Set `GROQ_API_KEY` to use Groq's ultra-fast inference (llama-3.3-70b-versatile by default). Auto-selected when the key is present.
- **Docker Compose with Ollama + GPU** — One-command setup: `docker compose up` starts Ollama with NVIDIA GPU passthrough, auto-pulls `qwen2.5:7b` (fits RTX 4050 6 GB VRAM), and launches the chatbot sidecar.
- **Environment configuration** — New `.env.example` documents all config options.

## What Changed

### ComfyUI nodes (`src/filters/builtin/comfyui.rs`)
- `comfy_checkpoint_loader` — Load a Stable Diffusion checkpoint; outputs model/CLIP/VAE references.
- `comfy_clip_text_encode` — Encode text prompts via CLIP.
- `comfy_ksampler` — Core sampling step with sampler, scheduler, CFG, seed, and denoise controls.
- `comfy_vae_decode` — Decode latent images to pixel images; downloads result from ComfyUI.
- `comfy_lora_loader` — Apply LoRA with independent model/CLIP strength.
- `comfy_image_upscale` — Upscale via RealESRGAN or other upscale models in ComfyUI.
- `comfy_controlnet_apply` — Guide generation with ControlNet (canny, depth, pose, etc.).
- `comfy_workflow_runner` — Paste any ComfyUI API-format workflow JSON and execute it.

### Chatbot
- Added Groq backend (`_generate_groq`) using the OpenAI-compatible API at `api.groq.com`.
- Backend auto-selection priority: Anthropic → Groq → OpenAI → Ollama (local).
- Runtime `/llm/config` endpoint now supports updating Groq API key.

### Infrastructure
- `docker-compose.yml` — Ollama + chatbot services with GPU passthrough.
- `Dockerfile.chatbot` — Python 3.11 slim image for the chatbot sidecar.
- `chatbot/requirements.txt` — Pinned Python dependencies.
- `.env.example` — Full environment variable documentation.

---

# Ambara v0.8.0 Release Notes

**Release Date:** 20 March 2026

## Highlights

- **Code-as-RAG retrieval** — The chatbot now reads Ambara's Rust source code directly to retrieve filter metadata and behavior. This removes drift between implementation and retrieval corpus.
- **Agentic chat router** — The old keyword intent classifier has been replaced with an LLM-driven tool-using agent that decides when to explain filters, search, suggest pipelines, or generate full graphs.
- **API/model integration filters** — Added 5 new built-in filters for external model and API workflows:
	- `http_image_fetch`
	- `stable_diffusion_generate`
	- `image_classify`
	- `model_inference`
	- `style_transfer`
- **New `Api` category** — Built-in categories now include an explicit API integration class for model-serving and remote inference nodes.
- **Dynamic planning catalog** — Planning prompt catalog is now generated from live code-retriever data instead of a stale static string.

## What Changed

### Chatbot backend
- Added `chatbot/retrieval/code_retriever.py` for source-level filter parsing and in-memory indexing.
- Added `chatbot/generation/tools.py` with structured tool schemas and runtime tool execution.
- Added `chatbot/generation/agent.py` implementing a ReAct-style multi-round tool loop.
- Updated `chatbot/api/main.py`:
	- `/chat` now uses `Agent`
	- `/filters` and `/filters/search` now use code-as-RAG source retrieval
	- startup corpus generation now derives from parsed source

### Graph generation
- Updated `chatbot/generation/graph_generator.py` to consume `CodeRetriever` instead of ChromaDB retrieval.
- Updated `chatbot/generation/planner.py` to accept dynamic catalog injection.

### Rust filter system
- Added `src/filters/builtin/api.rs` with API/model filters.
- Updated `src/filters/builtin/mod.rs` to register the new `api` module.
- Updated `src/core/node.rs` to add `Category::Api`.
- Added `ureq` dependency in `Cargo.toml` for HTTP calls.

### Tests
- Added `chatbot/tests/test_code_retriever.py` to validate code-retriever indexing/search, tool executor behavior, and agent mock-mode interactions.

## Verification

- `cargo check`: successful
- Python test execution in this environment was blocked by a broken interpreter (`ModuleNotFoundError: encodings`), so pytest could not be run in-session.

---

# Ambara v0.7.1 Release Notes

**Release Date:** 19 March 2026

## Highlights

- **17 new filters** — 91 total filters across all 16 categories. All previously empty categories (Sharpen, Edge, Noise, Draw, Text) now have implementations.
- **UI color fix** — All 16 node categories now display correct header and border colors in the graph editor. Fixed mismatched keys (`Source`→`Input`, `Filter`→`Blur`, `Analysis`→`Analyze`).
- **New Adjust filters** — Gamma correction (with LUT optimization) and Color Balance (independent RGB multipliers).
- **New Color filters** — Sepia tone, Hue Rotate, Binary Threshold, and Posterize.
- **Drawing primitives** — Rectangle, Circle, and Line drawing nodes with fill/outline, RGB colors, and thickness.
- **Text overlay** — Built-in 8×13 bitmap font for rendering text directly on images, no external font files needed.
- **Chatbot updated** — Filter catalog, deterministic keyword fallbacks, and ChromaDB embeddings all updated for the new filters.

## New Filters by Category

| Category | Filters | Description |
|----------|---------|-------------|
| Sharpen | `unsharp_mask`, `sharpen` | Classic unsharp masking and 3×3 convolution kernel |
| Edge | `edge_detect`, `emboss` | Sobel/Prewitt edge detection, directional emboss effect |
| Noise | `add_noise`, `denoise` | Gaussian/salt-and-pepper noise, median filter denoising |
| Draw | `draw_rectangle`, `draw_circle`, `draw_line` | Shape drawing with fill/outline modes |
| Text | `text_overlay` | Bitmap text rendering with configurable position, scale, color |
| Color | `sepia`, `hue_rotate`, `threshold`, `posterize` | Tone/hue/quantization effects |
| Adjust | `gamma`, `color_balance` | Gamma correction, per-channel RGB adjustment |

## UI Fixes

- Fixed `categoryColors` mapping in `FilterNode.tsx` — now uses exact Rust `Category` enum names.
- Added CSS rules for `adjust`, `custom`, `sharpen`, `edge`, `noise`, `draw`, `text` categories.
- All 16 categories now have distinct border colors and background tints.

---

# Ambara v0.7.0 Release Notes

**Release Date:** 19 March 2026

## Highlights

- **Complete pipeline redesign** — Graph generation now uses a multi-stage agentic pipeline inspired by HuggingGPT and ReAct patterns, replacing the old single-shot LLM approach.
- **4-stage architecture** — Plan → Select → Connect → Validate+Repair, with each LLM call focused on one simple task.
- **Deterministic connection wiring** — Stage 3 uses 100% code (no LLM) to wire graph connections, eliminating hallucinated port names.
- **qwen3:8b optimized** — Compact filter catalog, filter cards, few-shot examples, and `<think>` tag stripping ensure reliable output from smaller models.
- **Intelligent parameter inference** — Regex-based extraction of dimensions, opacity, angles from query text without extra LLM calls.
- **Robust fallback** — Keyword-based deterministic generation when any pipeline stage fails.

## What Changed

### Pipeline architecture (new)
- Added `chatbot/generation/planner.py` — Stage 1: decomposes query into ordered steps using compact filter catalog with 5 few-shot examples.
- Added `chatbot/generation/selector.py` — Stage 2: selects best filter per step using compact "filter card" format and semantic retrieval.
- Added `chatbot/generation/connector.py` — Stage 3: deterministic graph wiring with priority-based port matching (exact name → exact type → Any → coercion → fallback).
- Rewrote `chatbot/generation/graph_generator.py` — Orchestrates all 4 stages with graceful degradation to deterministic fallback.

### Reliability improvements
- qwen3 `<think>` tag stripping in plan/selection parsers.
- Query-order-preserving keyword matching in deterministic fallback.
- Astrophotography pipelines correctly use `load_folder` for stacking workflows.
- Parameter values extracted from query text (e.g., "512x512", "70% opacity", "90 degrees") without LLM calls.

### Test suite
- Updated repair loop tests for new multi-stage architecture semantics.
- All 28 Python tests passing.
- All 121 Rust tests passing (111 + 2 + 8).

### Research
- Added `papers/` directory with 7 research summaries (ReAct, Chain-of-Thought, HuggingGPT, Toolformer, Gorilla, TaskWeaver/ToolBench, synthesis) documenting the design rationale.

## Verification

- `python3 -m pytest chatbot/tests/ --ignore=chatbot/tests/test_e2e.py`: 28 passed
- `cargo test`: 121 passed, 0 failed

## Links

- [Full Changelog](https://github.com/PrakyathPNayak/ambara/compare/v0.6.0...v0.7.0)
- [Documentation](https://github.com/PrakyathPNayak/ambara#readme)
- [Report Issues](https://github.com/PrakyathPNayak/ambara/issues)
