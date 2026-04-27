# Next loop seed (loop 22)

Top candidate: cycle through still-queued items by priority order.

Highest remaining: Backup C from loop 19 — Ollama 60s timeout is
likely too short for CPU-only local models (qwen3:8b on CPU can take
several minutes for the first token). Either:
- Bump to a saner default (e.g. 180s) and document the rationale.
- Make configurable via env var (OLLAMA_TIMEOUT_S).

Priority-3 (real bug) but low impact (only affects local-Ollama users
on CPU). Adjust to priority-5.

Backup A: CycleDetected variant doc divergence (loop 17). Documentation
fix to clarify the variant carries one of two shapes. Low risk, low
leverage. Priority-9.

Backup B: can_execute() zero callers — delete or wire into Executor.
Priority-6 API consistency. Read all callers (none in production) then
delete; the dead code is misleading.

Other queued (lower priority):
- Missing git tags v0.7.1 / v0.8.0 / v0.9.0 — release-process call.
- Self-feedback edges architecture (loop 8).
- comfyui_bridge filter-count smoke replacement (loop 15).
- FilterNodeData JSON schema-version snapshot (loop 16).
- Cleaner missing-key test fixture (loop 20).

Decision: take Backup B (`can_execute()` deletion) — it's a true
priority-6 bug (dead code that misleads readers), simple to fix, and
has been queued since loop 13. Outranks Ollama timeout (priority-5
enhancement to a niche path).
