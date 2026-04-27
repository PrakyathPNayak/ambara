# Next loop seed (loop 30)

Loops 25-29 closed the silent-graph-corruption / user-facing-error
formatting cluster in the Tauri + CLI graph paths. Move on to the
next-highest-impact items.

## Candidates

1. **Production unwrap hardening — src/graph/topology.rs:40,42,57,58.**
   Four `.unwrap()` calls on HashMap lookups inside the topological
   sort. They are safe by construction (the keys are inserted in a
   prior loop over the same node set) but produce ugly panics if a
   future refactor breaks the invariant. Replace with
   `.expect("topology: <documented invariant>")` and add a doctest
   or unit test that documents the invariant. Priority-7 (fragile
   assumptions).

2. **`Position::default()` sanity — ui/src-tauri/src/lib.rs.** Verify
   what x/y values it produces on missing fields (almost certainly
   0.0/0.0). If a serialized graph omits position, do nodes overlap
   silently in the UI? Either document the default or add a test that
   pins the contract. Priority-8.

3. **GpuError Debug→Display in src/core/gpu.rs:231** for wgpu errors.
   Audit whether wgpu::BufferAsyncError's Display is actually clean;
   if so, switch to `{}`. Priority-7.

4. **Anthropic API version env var** — chatbot/generation/llm_client.py
   pins ANTHROPIC_VERSION = "2023-06-01" as a module constant. Wrap
   with the `_resolve_str_env` helper introduced in loops 22-24 so
   ops can override without redeploy. Priority-7.

5. **`_RETRY_DELAY_S` / `_RETRY_AFTER_MAX_S` env overrides** — same
   chatbot file, same helper pattern. Priority-7.

## Recommended pick

Candidate 1 (topology.rs unwrap hardening). Priority-7 but it's a
production hot path (every graph execution runs topological sort)
and the invariant is currently undocumented — the next refactor of
`add_node` could silently break the assumption. The fix is small
(four `.expect` calls + a comment block + one test) and the test
would lock in the invariant explicitly.

## Loop 29 result summary
Tauri user-facing error messages now use Display instead of Debug.
Test suite: 329 (166 Rust + 2 UI + 161 Python), all green.
