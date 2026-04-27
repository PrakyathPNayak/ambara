# Next loop seed (loop 8)

Top candidate: cycle-prevention audit in `src/graph/structure.rs::would_create_cycle` and `connect()`. Concrete tests to write (and look at the code while writing them — if the code looks wrong, fixing the bug is a higher-priority commit than just adding tests):

  1. self-loop: `graph.connect(a, "output", a, "input")` must error with `CycleDetected`. The existing port-already-connected check fires *after* the cycle check at line 292, so this should already be rejected.
  2. near-cycle: chain A→B→C, then `connect(c, "output", a, "input")` must error with `CycleDetected`.
  3. parallel back-edge: A→B exists, then attempting `connect(b, "output", a, "input")` (a different input port if such existed) — would create a 2-cycle.

Read `would_create_cycle` first. Look for:
- Does it traverse the existing connections graph correctly?
- Does it consider the proposed edge `from_node → to_node` and check if `to_node` can reach `from_node` already?
- Is it O(V+E) or quadratic?
- Self-loop edge case: `from_node == to_node` should short-circuit.

Backup candidate: README workflow badge once tests.yml has run successfully at least once.

Other queued items:
- Verify `chatbot/generation/graph_generator_legacy.py` is dead code; remove if so.
- Version drift across Cargo.toml / README / git tags / ui/package.json.
- `ValidationReport::can_execute()` semantics for empty graphs.
- chatbot LLM client timeout/retry policy.
- `ui/src-tauri` clippy/test gating in CI.
