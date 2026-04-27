# Loop 26 seed

Loop 25 fixed a priority-1 CLI panic in `parse_serialized_graph` — bare-scalar
JSON files no longer crash `ambara load-graph`.

## Open priority-1 candidates to audit next

1. **Audit other `serde_json::Value` IndexMut sites repo-wide.** The same
   panic pattern may exist elsewhere. Run:
   `rg 'Value\["|raw\[|\["[a-z_]+"\] =' --type rust`
2. **`src/execution/cache.rs:211`** — `NonZeroUsize::new(100).unwrap()`. The
   inner unwrap is provably safe (100 > 0), but the OUTER chain
   `.unwrap_or(NonZeroUsize::new(100).unwrap())` warrants documentation.
3. **`src/core/gpu.rs:225`** — `tx.send(result).unwrap()` inside wgpu
   `map_async` callback. Safe by stack order (rx alive until line 229),
   but document the invariant or use `.expect("rx alive on caller stack")`.
4. **`src/graph/topology.rs:40,42,57,58`** — HashMap unwraps. Invariant
   holds because adjacency/in_degree are populated for every node_id
   before connections are processed, AND `connect()` validates both
   endpoints exist, AND `remove_node` cascades to connections. Safe by
   construction but worth `.expect("populated by node_ids() loop above")`
   to harden against future refactors.

## Other queued items (priority 5+)
- Anthropic API version `ANTHROPIC_VERSION` env var (needs `_resolve_str_env`).
- `_RETRY_DELAY_S` / `_RETRY_AFTER_MAX_S` env vars in chatbot/llm_client.
- LLMConfig dataclass extraction (defer until 4-5 distinct knobs).
- CycleDetected variant doc divergence (loop 17).
- Self-feedback edges architecture (loop 8).

## Constraints
- DO NOT push to origin.
- DO NOT auto-create release tags.
- Every commit MUST include the Co-authored-by trailer.
