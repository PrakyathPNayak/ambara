# Next Loop Seed
- Fix `chatbot/tests/test_e2e.py::test_e2e_queries` timeout. It spawns a real uvicorn + invokes `/graph/generate` with default Ollama backend and 20 s per-request timeout. Without a running Ollama+model, it hangs ~280 s and fails.
- Approach: force `LLM_BACKEND=mock` (or equivalent env) in the spawned subprocess so the deterministic mock backend handles the calls; or skip the test when no backend is reachable. The real-server e2e is valuable, so prefer mock-backend over skip.
- After that: add a tests CI workflow gated on PRs, then start fixing the 5 Rust warnings + the tautological `duration_ms >= 0` assert.
