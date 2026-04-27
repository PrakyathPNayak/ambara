# Next loop seed (loop 14)

Top candidate: audit the README "250 tests (144 Rust + 106 Python)" claim. Run `cargo test --workspace` (counts lib + integration + doc across all members), `cd ui && npm test`, `pytest chatbot/tests --collect-only -q | tail -5` to get an accurate count. Update the README claim to match. Loops 11+13 changed Rust-side test totals.

Backup candidate: ARCH decision on the unreachable cycle branch in `topological_sort` (loop 11 reveal). Two options:
  1. Replace `if result.len() != node_count { return Err(CycleDetected) }` with `debug_assert_eq!` + `unreachable!()`/explanatory comment, on the basis that `connect()` provably rejects all cycles.
  2. Add `#[cfg(test)] pub(crate) fn force_unchecked_connect` to structure.rs so a test can inject a cycle and verify the branch.
Pick one with full devil step. Option 1 is honest but irreversible-feeling. Option 2 is safer but adds production-adjacent surface.

Other queued items:
- can_execute() has zero production callers (loop 13 deferred). Decide: delete, or wire into a real call site (e.g., make `Executor::execute()` short-circuit on `!can_execute(graph)` and return a sensible error).
- Smoke tests for `plugins/comfyui_bridge` and `ui/src-tauri` (loop 13 reveal — both have zero tests).
- Missing git tags v0.7.1 / v0.8.0 / v0.9.0 — maintainer release-process call.
- chatbot LLM client timeout/retry policy review (60s + 1 retry).
- Architectural decision: confirm self-feedback edges are intentionally rejected (loop 8 reveal).
