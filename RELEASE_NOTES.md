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
