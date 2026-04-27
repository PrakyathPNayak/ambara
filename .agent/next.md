# Next loop seed (loop 12)

Top candidate: README CI badge for `tests.yml`. One-line addition near the README header pointing at `https://github.com/PrakyathPNayak/ambara/actions/workflows/tests.yml/badge.svg`. Confirm the workflow filename and the repo slug before writing. Verifies CI visibility for contributors.

Backup candidate: `ValidationReport::can_execute()` empty-graph semantics. Currently returns `success`, which is `true` for empty graphs (loop 3 reveal). Read `src/validation/pipeline.rs` and `src/lib.rs` ValidationReport, decide if can_execute should require ≥1 node, then change behavior + adjust loop-3's empty-graph contract test if needed.

Other queued items:
- ARCH: cycle branch in `topological_sort` is unreachable through public API (loop 11 reveal). Either mark dead with `unreachable!()` + safety comment OR add a `#[cfg(test)] pub(crate) fn force_unchecked_connect` to structure.rs. Pick one, in a dedicated loop with full devil step.
- Missing git tags v0.7.1 / v0.8.0 / v0.9.0 — maintainer release-process call.
- chatbot LLM client timeout/retry policy review (60s + 1 retry).
- `ui/src-tauri` clippy/test gating in CI (separate workspace).
- Architectural decision: confirm self-feedback edges are intentionally rejected (loop 8 reveal).
