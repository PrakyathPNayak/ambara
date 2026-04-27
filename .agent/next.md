# Loop 27 seed

Loop 26 fixed silent rerouting on duplicate node ids in CLI graph
validation. Tauri `import_graph_json` path NOT yet audited.

## Highest-priority candidates for loop 27

1. **Audit Tauri `apply_graph_state` for the same dup-id bug.** When the
   frontend imports a hand-edited graph via `import_graph_json` →
   `apply_graph_state`, does the lib.rs path also tolerate duplicate node
   ids and silently rewire connections? `ui/src-tauri/src/lib.rs:440`
   onwards is the entry point. Trace through where the imported
   GraphState meets the ProcessingGraph rebuild.

2. **`execute_serialized_graph` does NOT call `validate_serialized_graph`
   internally.** It relies on the CLI caller to validate first. If
   another caller emerges that skips validation, the dup-id rewiring
   reappears. Consider folding validation INTO execution as a defense in
   depth, or asserting validation invariants at the top.

3. **Duplicate connection detection.** `validate_serialized_graph` does
   not detect duplicate connections (same from/to endpoints). The graph
   `connect()` call may or may not deduplicate — verify and add a
   validation error if duplicates are not collapsed downstream.

## Queued from before
- Production unwrap hardening: topology.rs:40,42,57,58, gpu.rs:225,
  cache.rs:211 (priority 7 — `.expect("...")` swaps).
- Anthropic API version `ANTHROPIC_VERSION` env var.
- `_RETRY_DELAY_S` / `_RETRY_AFTER_MAX_S` chatbot env vars.
- LLMConfig dataclass extraction (defer until 4-5 distinct knobs).

## Constraints
- DO NOT push to origin.
- DO NOT auto-create release tags.
- Every commit MUST include the Co-authored-by trailer.
