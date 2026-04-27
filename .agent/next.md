# Next loop seed (loop 9)

Top candidate: investigate `chatbot/generation/graph_generator_legacy.py` — bootstrap suspected dead code. Steps:
  1. `grep -r graph_generator_legacy chatbot/` (expect: only the file itself, no importers).
  2. Check `chatbot/generation/__init__.py` for re-exports.
  3. If genuinely dead, delete it and confirm `pytest chatbot/tests` still 106/106. Otherwise update or document why it lives.

Backup candidate: address version drift. `Cargo.toml` 0.5.0 / `ui/package.json` 0.5.0 / latest git tag v0.7.0 / README references v0.9.0. Consensus action: bump Cargo.toml + ui/package.json to v0.7.0 to match the actual release tag, and either add a v0.9.0 tag aligned with current state or correct the README. This is a coordination question — start by reading README to confirm which version it claims.

Other queued items:
- `ValidationReport::can_execute()` semantics for empty graphs (returns true; questionable).
- chatbot LLM client timeout/retry policy review.
- `topological_sort` empty-graph + cycle-rejection test (loop 4 noted gap).
- README badge for tests.yml workflow.
- `ui/src-tauri` clippy/test gating in CI.
- Architectural decision: confirm self-feedback edges are intentionally rejected (loop 8 reveal).
