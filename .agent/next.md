# Next loop seed (loop 20)

Top candidate: DRY refactor of LLMClient's three paid-provider paths
(_generate_anthropic, _generate_openai, _generate_groq). All three
repeat the same shape:
  1. assert API key
  2. build OpenAI-shaped or Anthropic-shaped body
  3. _post_with_retry
  4. raise on >=400
  5. extract content from JSON

The OpenAI/Groq paths are nearly identical (Groq uses an
OpenAI-compatible endpoint). Refactor opportunity:
- Extract `_call_openai_compatible(url, headers, body)` for OpenAI/Groq
- Keep Anthropic separate (its body shape differs)
- Add per-provider tests covering 200 happy path + 4xx error path

DEVIL warning: this refactor touches paid-API call paths. Ensure tests
cover both the happy path AND the error path BEFORE refactoring; the
loop-18 retry tests cover the helper but not the response-shape
extraction. Add response-shape tests first if missing.

Backup A: CycleDetected variant doc divergence (loop 17).

Backup B: can_execute() zero callers — delete or wire into Executor.

Backup C: Ollama 60s timeout — likely too short for CPU-only local
models. Bump to 180s and document the rationale, or make configurable
via env var.

Other queued:
- Missing git tags v0.7.1 / v0.8.0 / v0.9.0.
- Self-feedback edges architecture (loop 8 reveal).
- comfyui_bridge filter-count smoke replacement (loop 15 reveal).
- FilterNodeData JSON schema-version snapshot (loop 16 reveal).
