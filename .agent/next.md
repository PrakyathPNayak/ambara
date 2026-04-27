# Loop 28 seed

Loops 26-27 fixed silent dup-id rerouting in both CLI and Tauri runtime
paths. Both surfaces now bail on duplicate node ids during validation.

## Highest-priority candidates for loop 28

1. **Duplicate connection / edge detection.** Neither `validate_graph`
   (Tauri) nor `validate_serialized_graph` (CLI) detects duplicate
   connections. Two edges with identical (source, source_handle, target,
   target_handle) tuples may double-deliver values during execution or
   silently no-op depending on graph::ProcessingGraph::connect's
   semantics. Verify connect() behavior, then surface duplicates as a
   warning or error.

2. **`execute_graph` defense in depth (lib.rs:652).** Currently relies on
   `validate_graph` being called first, but the function clones the
   GraphState and re-validates internally — so if the validator misses a
   bug, execution proceeds. Worth running through with a critical eye
   for what slips past validate_graph today.

3. **`apply_graph_state` (state-mutation path) — does it exist for
   GraphState ingestion?** Search ui/src-tauri/src/lib.rs and frontend
   for paths that turn imported GraphState into AppState. If imports
   skip `validate_graph`, the dup-id (and other) bugs reappear on
   import-without-execute. Likely a real gap — investigate.

## Production unwrap hardening (priority 7, queued)
- src/graph/topology.rs:40,42,57,58 — `.expect("populated by node_ids() loop above")`.
- src/core/gpu.rs:225 — `.expect("rx alive on caller stack")`.
- src/execution/cache.rs:211 — document NonZeroUsize fallback.

## Other queued items
- Anthropic API version `ANTHROPIC_VERSION` env var (`_resolve_str_env` helper).
- `_RETRY_DELAY_S` / `_RETRY_AFTER_MAX_S` chatbot env vars.
- LLMConfig dataclass extraction (defer until 4-5 distinct knobs).

## Constraints
- DO NOT push to origin.
- DO NOT auto-create release tags.
- Every commit MUST include the Co-authored-by trailer.
