# Next loop seed (loop 25)

Top candidate: the `LLMClient.__init__` env-var reading is
spread across 6 separate `os.getenv` calls and 3 resolver calls.
The "configuration" of the client is implicitly defined by which
env vars it reads. This is fine at 9 knobs but starting to feel
loose. Consider extracting an `LLMConfig` dataclass that:
- documents every env var in one place
- supports test injection without monkeypatching
- separates "what backend is configured" from "what ambient state
  is now"

DEVIL pre-warning: This is structural cleanup, not bug-fix. Priority-9
work. Skip if any priority-1-7 item exists. Reread .agent/bootstrap.md
critical-path list before starting.

Actual highest-priority candidates worth checking first:
- src/graph/topology.rs cycle detection: did loop 17 actually cover
  all branches, or just the SCC-residue one? Reread.
- ComfyUI bridge error handling: what happens on a malformed
  workflow JSON? Read plugins/comfyui_bridge/src/lib.rs.
- The validation/pipeline.rs error-aggregation paths — any
  swallowed errors?

Backup A: Anthropic API version configurable (loop 23 backup).
String resolver pattern. Real but not critical — current
"2023-06-01" still works.

Backup B: CycleDetected variant doc divergence (loop 17). Doc-only.

Backup C: comfyui_bridge filter-count smoke (loop 15) — blocked.

Decision: OBSERVE step in loop 25 should re-run cargo test, re-read
the bootstrap priority list, and check for any priority-1-7 items
that have surfaced. Don't auto-pick the structural cleanup.
