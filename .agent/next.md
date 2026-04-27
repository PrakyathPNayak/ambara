# Next loop seed (loop 13)

Top candidate: `ValidationReport::can_execute()` empty-graph semantics. Loop 3 codified `success=true / can_execute=true` for empty graphs but flagged it semantically wrong. Steps:
  1. Read the report struct (likely in `src/validation/pipeline.rs` or `src/lib.rs`).
  2. Change `can_execute()` to require `node_count > 0` (or equivalent: at least one source node).
  3. Update loop-3's `test_empty_graph_validation` and `test_validation_pipeline` so they assert `can_execute=false` for empty graphs while keeping the warning + success contract.
  4. Search for callers; ensure no caller relied on can_execute being true for empty graphs.

Backup candidate: audit the README "250 tests" claim. Run cargo test --workspace, cargo test --doc, npm test, pytest, sum the numbers, update README to the real count. Loop 12 reveal.

Other queued items:
- ARCH: cycle branch in `topological_sort` is unreachable through public API (loop 11 reveal). Decide between `unreachable!()` + comment OR test backdoor.
- Missing git tags v0.7.1 / v0.8.0 / v0.9.0 — maintainer release-process call (do NOT auto-tag).
- chatbot LLM client timeout/retry policy review (60s + 1 retry).
- `ui/src-tauri` clippy/test gating in CI (separate workspace).
- Architectural decision: confirm self-feedback edges are intentionally rejected (loop 8 reveal).
