# Next loop seed (loop 23)

Top candidate: Anthropic max_tokens hard-coded at 4096 (loop 22 reveal).
For graph-generation prompts that's enough; for chat-response prompts
on long contexts it can truncate mid-sentence. Make configurable via
`ANTHROPIC_MAX_TOKENS` env var with same resolver pattern as
`_resolve_ollama_timeout`.

DEVIL warning: Test the env-var resolver alone (not the full
_generate_anthropic) to keep the test boundary tight. Mirror the
loop-22 test structure.

Backup A: CycleDetected variant doc divergence (loop 17). Document
that the variant carries one of two shapes (offending edge from
connect() vs SCC residue from topological_sort). Pure documentation
fix, low leverage but completes a long-queued item.

Backup B: comfyui_bridge filter-count smoke replacement (loop 15).
When real filters land, replace `filter_count == 0` smoke test.
Currently no real filters → blocked.

Other queued (lower priority):
- Missing git tags v0.7.1 / v0.8.0 / v0.9.0.
- Self-feedback edges architecture (loop 8).
- FilterNodeData JSON schema-version snapshot (loop 16).
- Cleaner missing-key test fixture (loop 20).

Decision: take the Anthropic max_tokens fix. Symmetric with loop 22's
work and uses the established resolver pattern.
