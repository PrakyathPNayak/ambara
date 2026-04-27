# Next loop seed (loop 32)

Loop 31 closed the gpu.rs hardening hole. Move on to the chatbot
env-var work or CLI executor defense-in-depth.

## Candidates

1. **CLI executor edge defense in depth** — `execute_serialized_graph`
   in src/main.rs. validate_serialized_graph already catches edges
   referencing missing nodes, but the executor itself should fail-loud
   if it ever receives a graph that bypassed validation (e.g. via a
   future API caller that skips the validate step). Mirror loop 28's
   Tauri fix: replace any `if let Some(...)` silent-skip on edge
   resolution with explicit `match` that returns an error. Priority-3
   if a silent-skip exists; priority-7 if it's already strict.

2. **Anthropic API version env var** — chatbot/generation/llm_client.py
   hardcodes ANTHROPIC_VERSION = "2023-06-01". Wrap with the
   `_resolve_str_env` helper introduced in loops 22-24. Add a pytest
   that monkeypatches the env var. Priority-7.

3. **`_RETRY_DELAY_S` / `_RETRY_AFTER_MAX_S` env overrides** — same
   chatbot file, same helper pattern. Could batch with #2.

4. **Position::default sanity** — ui/src-tauri/src/lib.rs. What x/y
   does Position::default produce on missing serialized fields?
   If 0/0, do imported nodes overlap silently in the UI? Either pin
   the contract with a test or document. Priority-8.

5. **cache.rs ResultCache new() capacity = 0** — line 211 already
   handles 0 by falling through to NonZeroUsize::new(100). Verify
   with a unit test that cache.new(0) actually produces a working
   100-capacity cache. Priority-8.

## Recommended pick

Candidate 1 (CLI executor defense in depth). If a silent-skip
exists, this is priority-3 (silent corruption); if it's already
strict, the loop produces a regression test that locks the contract.
Either way it's higher value than the chatbot env vars (priority-7).

## Loop 31 result summary
gpu.rs map_async unwrap documented + wgpu errors now formatted via
Display. Test suite: 330 (167 Rust + 2 UI + 161 Python), all green.
