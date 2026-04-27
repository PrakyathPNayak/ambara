# Next loop seed (loop 24)

Top candidate: Anthropic API version is hard-coded to "2023-06-01" in
the request headers. New Anthropic features (extended thinking,
caching, etc) require newer API versions. Make `ANTHROPIC_VERSION`
configurable with the same resolver pattern (string variant — needs a
`_resolve_str_env(var_name, default)` helper).

DEVIL pre-warning: Will need a string resolver, not the positive-int
one. Don't shoehorn — write a small parallel helper if needed, OR
just call `os.getenv(name, default)` directly since the validation
needs are different. Don't over-engineer.

Backup A: Make retry count configurable (`LLM_MAX_RETRIES`). Currently
`_MAX_RETRIES = 1` hard-coded. Operators behind flaky proxies might
want more. Uses the existing positive-int resolver directly — the
cleanest possible application.

Backup B: CycleDetected variant doc divergence (loop 17). Documentation
fix, low priority.

Backup C: comfyui_bridge filter-count smoke replacement (loop 15) —
blocked on real filters landing.

Other queued (lower priority):
- Missing git tags v0.7.1 / v0.8.0 / v0.9.0.
- Self-feedback edges architecture (loop 8).
- FilterNodeData JSON schema-version snapshot (loop 16).
- Cleaner missing-key test fixture (loop 20).
- Anthropic system-message join awkwardness (loop 21).

Decision: take Backup A (retry count). It's the cleanest and most
direct re-use of the loop-23 resolver — drop-in caller, no new
helper code, immediate win for unreliable-network operators. The
Anthropic version knob is real but adds a string-resolver concern
that would dilute focus.
