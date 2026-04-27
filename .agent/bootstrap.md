# Bootstrap — Repo Snapshot

## What this is
- Cargo workspace: root `ambara` crate (lib + bin), `plugins/comfyui_bridge`, `ui/src-tauri`.
- Tauri + React UI in `ui/`.
- FastAPI Python chatbot sidecar in `chatbot/` (Plan→Select→Connect→Repair pipeline).
- Workspace v0.5.0 in `Cargo.toml`, but README/git tags reference up to v0.9.0 (CHANGELOG drift).
- 217 tracked source files (excluding build/cache).

## Baseline test results (this session)

Rust (`cargo test`):
- lib: 134 passed, 0 failed, 2 ignored
- bin: 2 passed
- doc: 8 passed, 11 ignored
- 5 compile warnings:
  1. `src/core/batch.rs:162` unused import `ImageDataRef`
  2. `src/lib.rs:204` unused import `std::collections::HashMap`
  3. `src/graph/topology.rs:253` unused var `c`
  4. `src/graph/topology.rs:254` unused var `d`
  5. `src/validation/pipeline.rs:118` `report.duration_ms >= 0` is tautological (u64)

UI (`cd ui && npm test` / vitest):
- **2/2 FAIL** in `src/components/chat/__tests__/ChatPanel.test.tsx`
- Cause: `messagesEndRef.current?.scrollIntoView is not a function` — jsdom does not implement `scrollIntoView`, and the optional chain `?.` only guards null, not missing methods.

Python (`pytest chatbot/tests`):
- 105 passed, **1 FAIL** (`test_e2e.py::test_e2e_queries` — `httpx.ReadTimeout` after 280s)
- Test starts a real uvicorn server and calls `/graph/generate`. With Ollama as default backend (per `.env.example`), without a running model the LLM call stalls past the 20s per-request timeout. Test has no skip/marker for missing LLM env.

## Highest-leverage problems (initial ranking)

1. **UI test suite is red** — `ChatPanel` calls `scrollIntoView` without checking method exists. Fails in jsdom now; would also fail on any older WebView lacking the method. Real robustness bug, not test-only.
2. **e2e chatbot test is flaky/blocking** — runs a real server + LLM with a 20s timeout in CI/dev. No skip when LLM unavailable. Hides regressions and burns CI minutes.
3. **Rust compile warnings** (5) — cosmetic but trivial.
4. **Dead/duplicate code**: `chatbot/generation/graph_generator_legacy.py` shipped alongside the new orchestrator. Verify if still imported.
5. **Version drift**: `Cargo.toml` says 0.5.0 but README says v0.9.0; `ui/package.json` is 0.5.0; tags up to v0.7.0.
6. **Cycle/topology correctness**: read `graph/structure.rs` cycle prevention path closely — historically cycle checks plus `add_parent`/`remove_parent` are bug magnets.
7. **Validation `duration_ms >= 0`** is a no-op assert hiding broken intent (was probably meant to verify `> 0` after running stages, or to assert a duration field is set).
8. **Unguarded LLM client timeouts / retries** in `chatbot/generation/llm_client.py` — likely root of the e2e hang.
9. **Plugin `libloading` safety**: any `unsafe` in `src/plugins/loader.rs` should be audited.
10. **No CI test workflow**: `.github/workflows/build-release.yml` only builds. PRs land without test gates.

## Plan
Start loop 1 by fixing the red UI tests with a real robustness fix (feature-check + jsdom-friendly), since priority order says fix red tests before anything cosmetic. Subsequent loops will tackle e2e flakiness, then warnings, then deeper logic audit.
