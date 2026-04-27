# Next loop seed (loop 6)

Top candidate: clear the last Rust warning by removing `use ImageDataRef` in `src/core/batch.rs:162`. Trivial in isolation, but the real value is unblocking a future loop that adds `cargo clippy -- -D warnings` to the CI workflow. Verify the import is genuinely unused (not behind a cfg).

Backup candidate: cycle-prevention audit in `src/graph/structure.rs::would_create_cycle` and `connect()`. Targeted: write a test that constructs a near-cycle (A→B→C, then attempting C→A) and asserts the error path; another test for self-loops (A→A). If a real bug exists, this is priority 1.

Other queued items:
- Add `topological_sort` empty-graph + cycle-rejection tests (loop 4 noted gap).
- Verify `chatbot/generation/graph_generator_legacy.py` is dead code; remove if so.
- Version drift: Cargo.toml 0.5.0 / README v0.9.0 / git tags v0.7.0 / ui/package.json 0.5.0.
- `ValidationReport::can_execute()` returns true for empty graphs — semantic question.
- chatbot LLM client timeouts/retries policy.
- README badge for tests.yml workflow.
