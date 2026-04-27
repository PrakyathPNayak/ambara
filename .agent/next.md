# Next loop seed (loop 18)

Top candidate: `GraphError::CycleDetected { nodes }` shape inconsistency (loop 17 reveal). `connect()` populates `nodes = vec![from, to]` (the offending edge); `topological_sort` populates it with all SCC-residue nodes. Callers can't tell which shape they're getting. Either:
  1. Document both shapes in the variant's doc comment so callers know the contract is union-of-two-shapes.
  2. Split into two variants (`CycleAtConnection { from, to }` vs `CycleInGraph { nodes: Vec<NodeId> }`).
Option 2 is cleaner but breaks downstream pattern matches; option 1 is honest about the current state. Read all callers of `GraphError::CycleDetected` first to assess blast radius.

Backup: chatbot LLM client timeout/retry policy review (60s + 1 retry per loop-2 reveal). Read `chatbot/generation/llm_client.py`, decide whether 60s is too long for interactive UX and whether 1 retry on 5xx is the right number.

Other queued items:
- can_execute() has zero production callers — delete or wire into Executor::execute.
- Missing git tags v0.7.1 / v0.8.0 / v0.9.0 — maintainer release-process call.
- Self-feedback edges architecture (loop 8 reveal).
- When real comfyui filters land, replace `filter_count == 0` smoke test (loop 15 reveal).
- Schema-version snapshot test for FilterNodeData JSON shape (loop 16 reveal).
