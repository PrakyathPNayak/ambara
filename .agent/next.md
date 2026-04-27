# Next loop seed

Top candidate: investigate `src/graph/topology.rs` test ~lines 253-254 with unused `c`/`d` vars. Strong smell: nodes were created but never asserted against — likely a reduced/incomplete test on subgraph or parallel-batch detection. If a real bug is hiding, this is priority 3. If it's just dead vars, priority 7 cleanup.

Backup candidate: audit `src/graph/structure.rs` cycle prevention on `connect()` and `add_node()` — bootstrap flagged this as a possible priority-1 area.

Other queued items:
- Add `.github/workflows/tests.yml` — `cargo test`, UI vitest, fast subset of pytest.
- Remaining warning: unused `ImageDataRef` import in `src/core/batch.rs:162`.
- Verify `chatbot/generation/graph_generator_legacy.py` is unused; remove if so.
- Version drift across Cargo.toml / README / tags / package.json.
- Revisit `can_execute()` contract for empty graphs — should it be false?
