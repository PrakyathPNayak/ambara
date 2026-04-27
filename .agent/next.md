# Next loop seed (loop 19)

Top candidate: chatbot LLM client `Retry-After` header support. On HTTP 429
the helper sleeps a flat 2s rather than honoring the server-supplied
Retry-After. Real API-citizen bug. Implementation: parse the header
(seconds or HTTP-date), clamp to a sane max (e.g. 30s), fall back to
the existing 2s constant. New test cases covering: numeric header,
HTTP-date header, missing header, malformed header. Read RFC 7231 §7.1.3
to confirm both formats.

Backup A: CycleDetected variant doc divergence (loop 17 reveal). Document
that the variant carries one of two shapes (offending edge from connect()
vs SCC residue from topological_sort) so future callers can write correct
match arms.

Backup B: Remove unreachable line 156 in llm_client.py (loop 18 reveal).
Cosmetic but warranted; the loop's exit invariants are now provable
by tests.

Other queued items:
- can_execute() has zero production callers — delete or wire.
- Missing git tags v0.7.1 / v0.8.0 / v0.9.0.
- Self-feedback edges architecture (loop 8 reveal).
- comfyui_bridge filter-count smoke replacement (loop 15 reveal).
- FilterNodeData JSON schema-version snapshot (loop 16 reveal).
