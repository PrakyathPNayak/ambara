# Loop 35 seed

**Loop 34 outcome**: Both retry-loop floats are now env-overridable via `LLM_RETRY_DELAY_S` and `LLM_RETRY_AFTER_MAX_S`. `_resolve_positive_float_env` is available for future float knobs. Test count 337 → 349.

**Env-override theme is now complete for the chatbot.** All operationally-significant LLM client knobs are env-driven:
- Timeouts: `LLM_OLLAMA_TIMEOUT_S`, `OPENAI_TIMEOUT_S` / `GROQ_TIMEOUT_S` / `ANTHROPIC_TIMEOUT_S` (loop ≤28)
- Token budget: `ANTHROPIC_MAX_TOKENS`
- Retry budget: `LLM_MAX_RETRIES`
- Retry timing: `LLM_RETRY_DELAY_S`, `LLM_RETRY_AFTER_MAX_S` (this loop)
- API version: `ANTHROPIC_VERSION` (loop 33)

**Top candidates for loop 35** (move to a new theme):

1. **`ResultCache::new(0)` silent fallback test** (priority 7 — fragile assumption). `src/execution/cache.rs:211` falls back to `NonZeroUsize::new(100).unwrap()` when caller passes 0. Add a unit test pinning that fallback so the literal can never silently drift to 0 (which would `unwrap` panic). Quick, low-risk regression coverage.

2. **Re-read `.agent/bootstrap.md`** to surface unaddressed priority-1 through priority-4 items. Specifically check:
   - `plugins/comfyui_bridge` error paths (was flagged vaguely; needs scoping).
   - `src/execution/cache.rs` TTL eviction logic — is there a test for stale-entry pruning? Are there any silent overflows on size accounting?
   - `chatbot/generation/llm_client.py` retry exhaustion path: when `_post_with_retry` exhausts retries on a 5xx (not exception), it returns the bad response unchecked. Does the caller surface the status code, or does it silently parse a non-2xx body as success?

3. **Audit `src/main.rs` execute_serialized_graph** for any remaining silent-failure paths beyond the duplicate-id check added in loop 32 — e.g., does it validate that node_map lookups in the connection loop succeed?

4. **plugin manifest loader hardening** in `src/plugins/`. Check whether load failures are bubbled to the user or silently dropped.

**Recommendation**: pick #2 — specifically the chatbot retry-exhaustion behavior. That's a potential priority-2 item (missing error handling on a critical path: 5xx retried-and-exhausted should NOT silently return a malformed body to the caller). Worth scoping in OBSERVE before committing to it.

**DO NOT**: chase more env-override knobs in loop 35; the theme is complete. Move to a different priority.
