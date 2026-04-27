# Next loop seed (loop 7)

Top candidate: promote clippy to a CI gate now that the codebase is at 0 warnings + 0 clippy lints. Add a step `cargo clippy --lib --tests -- -D warnings` to the `rust` job in `.github/workflows/tests.yml`. Verify locally first that `cargo clippy --all-targets` (not just `--lib --tests`) is also clean — examples or bin targets may still have issues.

Backup candidate: cycle-prevention audit in `src/graph/structure.rs::would_create_cycle`. Concrete tests to write:
  1. self-loop: `graph.connect(a, "output", a, "input")` should error with `CycleDetected`.
  2. near-cycle: A→B→C, then `connect(c, "output", a, "input")` should error with `CycleDetected`.
  3. parallel-edge between same node pair (already legal since input port can hold one connection — reconfirms semantics).
If any test fails, that's a priority-1 bug.

Other queued items:
- README workflow badge.
- Verify `chatbot/generation/graph_generator_legacy.py` is dead code; remove if so.
- Version drift across Cargo.toml / README / git tags / ui/package.json.
- `ValidationReport::can_execute()` semantics for empty graphs.
- chatbot LLM client timeout/retry policy review.
