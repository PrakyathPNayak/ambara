# Next loop seed (loop 33)

Loop 32 closed the last silent-corruption gap in CLI executor
(duplicate-node-id node_map overwrite).

The CLI + Tauri graph load/validate/execute paths are now
defense-in-depth complete for:
- duplicate node ids (loops 26, 27, 32)
- dangling edge endpoints (loops 26, 28)
- silent edge drops (loop 28)
- bare-scalar JSON panic (loop 25)
- typed-error Debug-vs-Display formatting (loops 29, 31)
- topology HashMap unwraps (loop 30)
- gpu.rs map_async unwrap and wgpu Display (loop 31)

## Candidates for loop 33

1. **Anthropic API version env var** — chatbot/generation/llm_client.py
   hardcodes ANTHROPIC_VERSION = "2023-06-01" as a module constant.
   Wrap with `_resolve_str_env` (env-var resolver introduced in
   loops 22-24). Add a pytest that monkeypatches the env var.
   Priority-7.

2. **`_RETRY_DELAY_S` and `_RETRY_AFTER_MAX_S` env overrides** — same
   chatbot file. Same `_resolve_*_env` helper pattern (already used
   for the timeout/temperature knobs in loops 22-24). Probably
   batch with #1.

3. **cache.rs::ResultCache::new(0) contract** — line 211 falls
   through to NonZeroUsize::new(100) when capacity = 0. No test
   verifies this fallback. Add a test that asserts
   `ResultCache::new(0)` produces a working cache with the
   100-fallback capacity. Priority-8.

4. **Position::default sanity in ui/src-tauri/src/lib.rs** — what
   x/y does it produce on missing serialized fields? If 0/0,
   imported nodes overlap silently in the UI. Document the contract
   with a test or a serde default. Priority-8.

5. **Look for new high-priority issues**: re-read .agent/bootstrap.md
   priority list and grep for any silent-failure / missing-error-
   handling patterns I haven't addressed yet (e.g. the chatbot's
   request retry loop, the plugin loader, the comfyui_bridge
   crate, the execution cache TTL eviction).

## Recommended pick

Candidate 1 + 2 batched. The chatbot env-var hardening pattern is
already established in loops 22-24; finishing the remaining knobs
in one loop closes the chatbot configuration story. Priority-7 but
high coverage gain (3 new env vars, 3 new tests).

If candidate 1+2 reveal a deeper issue (e.g. retry loop has a
silent-failure path), defer to that immediately.

## Loop 32 result summary
CLI executor now rejects duplicate node ids defense-in-depth.
Test suite: 331 (168 Rust + 2 UI + 161 Python), all green.
