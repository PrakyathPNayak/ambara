# Next loop seed (loop 5)

Top candidate: add a `.github/workflows/tests.yml` that runs `cargo test --lib` + UI vitest + a fast subset of `pytest chatbot/tests` on push/PR. Currently no CI gating on tests — every loop's improvements rely on local discipline. Adding CI is high impact: it locks in all the work done so far and prevents regressions for every contributor.

Considerations:
- Use stable Rust + Node 20 + Python 3.12 to match repo conventions (README + repo dotfiles).
- Skip GPU/embedding-heavy chatbot tests via marker or path filter to keep CI under ~5 min.
- Set `AMBARA_FORCE_MOCK_LLM=1` for the chatbot job (the env var added in loop 2).
- Don't run tauri build (slow, requires platform tooling).

Backup candidate: fix last warning — drop unused `ImageDataRef` import in `src/core/batch.rs:162`. Trivial, but priority 9, so prefer the CI workflow.

Other queued items:
- Cycle-prevention audit in `src/graph/structure.rs` (would_create_cycle, connect, add_node).
- `topological_sort` lacks empty-graph + cycle-rejection tests.
- Verify `chatbot/generation/graph_generator_legacy.py` is dead; remove if so.
- Version drift: Cargo.toml 0.5.0 / README v0.9.0 / git tags v0.7.0 / ui/package.json 0.5.0.
- `ValidationReport::can_execute()` returns true for empty graphs — questionable contract.
- chatbot LLM client timeouts/retries policy.
