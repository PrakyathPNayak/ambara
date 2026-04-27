# Next loop seed (loop 16)

Top candidate: ui/src-tauri smoke test. The crate has a lib (`ui_lib`) and a bin (`ui`). Inspect `ui/src-tauri/src/lib.rs` for any pure command/handler that's testable without a running webview. Likely candidates: tauri command function bodies that take primitive args and return Result. If everything is gated on a Tauri runtime, fall back to a build-only assertion via `cargo check -p ui` (already exercised in CI but no explicit test row).

Backup: ARCH cycle-branch decision in `topological_sort` (loop 11). Pick `unreachable!()` vs test backdoor with a full devil step.

Other queued items:
- can_execute() has zero production callers — delete or wire into Executor::execute.
- Missing git tags v0.7.1 / v0.8.0 / v0.9.0 — maintainer release-process call.
- chatbot LLM client timeout/retry policy review (60s + 1 retry).
- Self-feedback edges architecture (loop 8 reveal).
- When real filters land in comfyui_bridge, replace `filter_count == 0` smoke test (loop 15 reveal).
