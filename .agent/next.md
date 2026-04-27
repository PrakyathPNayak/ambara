# Next loop seed (loop 11)

Top candidate: `topological_sort` regression tests. Source `src/graph/topology.rs` exposes a sort that should:
  1. return an empty/single-element result for an empty graph (verify which);
  2. return `Err(GraphError::CycleDetected)` (or equivalent) when a cycle exists.
Currently neither path has explicit coverage. Add two tests asserting both contracts. Cross-check the error variant name in `src/core/errors.rs` before wiring.

Backup candidate: README CI badge for the `tests.yml` workflow. Quick one-liner near the top of `README.md`. Confirms loop 5's CI is visible to contributors.

Other queued items:
- Missing git tags v0.7.1 / v0.8.0 / v0.9.0 — maintainer release-process call (do NOT auto-tag).
- `ValidationReport::can_execute()` semantics for empty graphs (returns true today; semantically wrong).
- chatbot LLM client timeout/retry policy review (60s + 1 retry).
- `ui/src-tauri` clippy/test gating in CI (separate workspace, not covered by main rust job).
- Architectural decision: confirm self-feedback edges are intentionally rejected (loop 8 reveal).
- Loop-9 reveal: no transitive cleanup possible from legacy purge — all imports still in use by active code.
