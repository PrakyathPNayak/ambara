# Loop 34 seed

**Loop 33 outcome**: ANTHROPIC_VERSION is now env-overridable; `_resolve_str_env` helper available for future string knobs. Chatbot Python tests 161→167. No behavioral change when env unset.

**Loop 33 revealed**:
- `_RETRY_DELAY_S` (default 1.0) and `_RETRY_AFTER_MAX_S` (default 60.0) in llm_client.py are still hardcoded floats. Same risk class as ANTHROPIC_VERSION but lower urgency — retry behavior is internal, not a third-party contract.
- `chatbot/tests/test_llm_retry.py` imports both constants directly and asserts `sleep.assert_called_once_with(_RETRY_DELAY_S)`. Constants must remain at module level; the resolvers should *consume* them as defaults, not replace them.

**Top candidates for loop 34** (ranked by impact):

1. **`_resolve_positive_float_env` + retry-delay env overrides** (priority 6 — API/interface inconsistency: half the knobs are env-overridable, half aren't). Add helper, wrap `_RETRY_DELAY_S`/`_RETRY_AFTER_MAX_S` with `_resolve_retry_delay()` / `_resolve_retry_after_max()`. Constants stay at module level as defaults; resolvers called inside the retry loop. New tests in test_llm_timeouts.py mirroring loop 33's pattern.

2. **`ResultCache::new(0)` regression test** (priority 7 — fragile assumption). `src/execution/cache.rs:211` falls back to `NonZeroUsize::new(100).unwrap()` when caller passes 0. Add a unit test pinning that fallback so the literal can never silently drift to 0.

3. **plugin loader hardening** — `plugins/comfyui_bridge` was flagged in bootstrap.md as having weak error paths. Re-read bootstrap to confirm what's still unaddressed.

4. **Re-read .agent/bootstrap.md priority list** for any silent-failure / missing-error-handling pattern not yet addressed (chatbot retry loop, comfyui_bridge, execution cache TTL eviction).

**Recommendation**: pick #1 — finish the env-override theme started in loop 33 while it's fresh. Pattern is proven: helper + resolver + 6 tests + 4-line patch.

**DO NOT**: change `_RETRY_DELAY_S`/`_RETRY_AFTER_MAX_S` *constants* themselves — only wrap their *usage* with resolvers. Test imports of the constants must keep working.
