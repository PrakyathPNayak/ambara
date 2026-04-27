# Next loop seed (loop 21)

Top candidate (now unblocked): DRY refactor of LLMClient's three paid
paths. Loop 20 added 18 shape-extraction tests + 7 retry tests
(loop 18) + 7 Retry-After tests (loop 19) = 32 tests pinning the
contract. The refactor can proceed with high confidence.

Plan:
1. Extract `_call_openai_compatible(url, headers, body, provider)`
   shared between OpenAI and Groq (their endpoints are nearly
   identical — Groq is OpenAI-compatible by design).
2. Keep `_generate_anthropic` separate (different body shape /
   response shape).
3. The `try: ... except RuntimeError: raise` pattern in each paid
   path is a noop — remove it (it doesn't change semantics).
4. Run all 32 tests to verify no behavior change.

Devil note: the `_call_openai_compatible` extraction must preserve:
- "OpenAI request failed: STATUS TEXT" vs "Groq request failed: STATUS TEXT"
  prefix (different per-provider). Pass `provider` in.
- The 200-content extraction is identical in both: `data["choices"][0]["message"]["content"]`.

Backup A: CycleDetected variant doc divergence (loop 17).
Backup B: can_execute() zero callers — delete or wire.
Backup C: Ollama 60s timeout is likely too short for CPU-only models
(loop 19 reveal).

Other queued:
- Missing git tags v0.7.1 / v0.8.0 / v0.9.0.
- Self-feedback edges architecture (loop 8 reveal).
- comfyui_bridge filter-count smoke replacement (loop 15 reveal).
- FilterNodeData JSON schema-version snapshot (loop 16 reveal).
- Cleaner missing-key test fixture (loop 20 reveal).
