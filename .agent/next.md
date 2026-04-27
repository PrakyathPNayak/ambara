# Next loop seed (loop 31)

Loop 30 documented and hardened the topology.rs HashMap unwraps,
locking the invariant in with a remove_node regression test.

## Candidates

1. **gpu.rs:225 channel send unwrap** — `tx.send(result).unwrap()`
   inside the wgpu map_async callback. Safe by stack scope (rx held
   through `rx.recv()` on the next line) but fails with an unhelpful
   panic if a future refactor moves the channel out of scope.
   Replace with `.expect("buffer-map callback fired after rx was
   dropped; buffer.slice().map_async lifetime invariant violated")`.
   Priority-7. Harder to test without a live wgpu device, so accept
   the documentation-only improvement.

2. **gpu.rs:231 wgpu error Display** — `format!("Buffer mapping
   failed: {:?}", e)` — same Debug→Display class as loop 29.
   `wgpu::BufferAsyncError` derives `std::error::Error` so Display
   is available. Switch to `{}`.

3. **Anthropic API version env var** — chatbot/generation/llm_client.py
   hardcodes ANTHROPIC_VERSION = "2023-06-01". Wrap with the
   `_resolve_str_env` helper introduced in loops 22-24.
   Priority-7. Requires touching pytest.

4. **`_RETRY_DELAY_S` / `_RETRY_AFTER_MAX_S` env overrides** — same
   chatbot file, same helper pattern. Could batch with #3.

5. **CLI executor edge defense in depth** — verify
   `execute_serialized_graph` in src/main.rs does not silently drop
   edges referencing missing nodes (mirror of loop 28 fix). The
   validate_serialized_graph check should already catch this, but
   the executor itself should fail-loud.

## Recommended pick

Candidates 1 + 2 batched (both in src/core/gpu.rs, same loop).
The Display fix carries a regression test; the unwrap fix is
documentation-only but trivial. Together this closes the gpu.rs
hardening hole opened in next.md.

## Loop 30 result summary
topology.rs unwraps documented + tested via remove_node regression.
Test suite: 330 (167 Rust + 2 UI + 161 Python), all green.
