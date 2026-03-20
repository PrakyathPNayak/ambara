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
