# Next loop seed (loop 17)

Top candidate: ARCH cycle-branch decision in `topological_sort` (loop 11 reveal). The cycle-detection branch at src/graph/topology.rs:67-75 is unreachable through the public API (connect() rejects all cycles). Two options:
  1. Replace with `unreachable!("ProcessingGraph::connect rejects all cycles; if this fires, an unsafe-bypass was introduced — see src/graph/structure.rs::would_create_cycle")` and convert the function to never return Err. Honest, removes dead code, but irreversible-feeling.
  2. Add `#[cfg(test)] pub(crate) fn force_unchecked_connect` to structure.rs to inject cycles for tests; keep the branch live. Safer but adds test-only surface.
Run full devil step. Lean: option 2 (preserves defensive code, lets the branch be tested) — but verify `pub(crate)` truly hides the helper from consumer crates. Add one cycle-injection test exercising `topological_sort` returning Err and `has_cycle()` returning true.

Backup: schema-version snapshot test for `FilterNodeData` JSON shape (loop 16 reveal). Pin the camelCase output of `serde_json::to_value(&FilterNodeData { ... })` against a literal JSON object — protects the JS contract from silent field renames.

Other queued items:
- can_execute() has zero production callers — delete or wire into Executor::execute.
- Missing git tags v0.7.1 / v0.8.0 / v0.9.0 — maintainer release-process call.
- chatbot LLM client timeout/retry policy review (60s + 1 retry).
- Self-feedback edges architecture (loop 8 reveal).
- When real comfyui filters land, replace `filter_count == 0` smoke test (loop 15 reveal).
