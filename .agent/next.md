# Loop 29 seed

Loops 26-28 closed three silent-corruption bugs in graph
import/validation/execution paths (CLI dup-id, Tauri dup-id, Tauri
dangling edges + silent-drop on execute). Both surfaces now reject all
three classes with explicit error messages.

## Highest-priority candidates for loop 29

1. **Mirror dangling-edge detection in CLI `validate_serialized_graph`.**
   It already checks `from_node` / `to_node` against the node-id set
   (lines ~478-486 in src/main.rs), so this MAY already be covered.
   Verify, and add a test if missing.

2. **`execute_serialized_graph` defense in depth.** It returns Err on
   unknown from_node/to_node already (lines ~511-524), so the CLI is
   already resilient on the executor side. Confirm via test if needed.

3. **`Position` default sanity.** What does `Position::default()` set
   x/y to? If it's (NaN, NaN) or some surprising value, deserialized
   graphs with missing position fields could carry forward weird state.
   Quick check.

4. **Audit `execute_graph` connect() error handling.** When connect()
   fails (e.g., type mismatch, port not found, cycle), the error is
   wrapped with `format!("{:?}", e)`. That uses the Debug impl, which
   may produce unhelpful output for end users. Switch to Display
   formatting for graph errors (`{}`).

## Production unwrap hardening (priority 7, queued)
- src/graph/topology.rs:40,42,57,58 — `.expect("populated by node_ids() loop above")`.
- src/core/gpu.rs:225 — `.expect("rx alive on caller stack")`.
- src/execution/cache.rs:211 — document NonZeroUsize fallback.

## Other queued items
- Anthropic API version `ANTHROPIC_VERSION` env var.
- `_RETRY_DELAY_S` / `_RETRY_AFTER_MAX_S` chatbot env vars.
- LLMConfig dataclass extraction (defer until 4-5 distinct knobs).

## Constraints
- DO NOT push to origin.
- DO NOT auto-create release tags.
- Every commit MUST include the Co-authored-by trailer.
