# Next loop seed (loop 15)

Top candidate: Either a smoke test for `plugins/comfyui_bridge` or the cycle-branch architecture decision in `topological_sort`. The bridge test is more concrete and harder to drop later — start there. Steps:
  1. Read `plugins/comfyui_bridge/src/lib.rs` to find a public function with deterministic, dependency-free behavior (e.g., manifest parser, graph translator with a fixed input).
  2. Add one test to `plugins/comfyui_bridge/src/lib.rs#tests` covering happy path + at least one error case if surface allows.
  3. Verify `cargo test -p comfyui_bridge` reports >0 passed.

Backup: ARCH cycle-branch decision (loop 11). `topological_sort`'s cycle branch is unreachable through the public API. Two options:
  1. Replace branch with `unreachable!()` + safety comment, on the basis that `connect()` provably rejects all cycles.
  2. Add a `#[cfg(test)] pub(crate) fn force_unchecked_connect` to structure.rs to inject cycles for the test.
Pick one with full devil step.

Other queued items:
- Smoke test for `ui/src-tauri` (similar to plugin smoke test).
- can_execute() has zero production callers — delete or wire into Executor::execute.
- Missing git tags v0.7.1 / v0.8.0 / v0.9.0 — maintainer release-process call.
- chatbot LLM client timeout/retry policy review (60s + 1 retry).
- Self-feedback edges architecture (loop 8 reveal).
