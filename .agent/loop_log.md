# Loop Log

(append-only)

---
## Loop 1 — UI: feature-check `scrollIntoView`

**Observe**: `cargo test` green (134+2+8). `pytest chatbot/tests` 105/106 (e2e timeout). `cd ui && npm test` 0/2 — `messagesEndRef.current?.scrollIntoView is not a function`.

**Orient**: Highest-leverage problem with red tests is to make them green via a real fix, not a test patch. Component used `?.` (null guard) but didn't guard against missing methods. jsdom in tests, and any older WebView, would break.

**Decide**: Feature-check the method on the resolved DOM node before invoking. Also fix incomplete `useChatApi` mock (missing `error`, `clearError` — would crash on Dismiss click).

**Devil**:
- Correctness: `scrollIntoView` is on every real `HTMLElement`; the check fires only in jsdom/non-conforming envs. Behavior unchanged in production. ✓
- Scope: Could polyfill in test setup instead, but that doesn't make the component robust if a future WebView ships without it. Component fix has wider value. ✓
- Priority: e2e timeout and validation `>= 0` assert remain. Acceptable — red tests must be fixed first per non-negotiables. Next loop will tackle e2e flakiness. ✓

**Act**: 
- `ui/src/components/chat/ChatPanel.tsx`: replace `messagesEndRef.current?.scrollIntoView(...)` with explicit method-existence check.
- `ui/src/components/chat/__tests__/ChatPanel.test.tsx`: add `error: null, clearError: vi.fn()` to the mock.

**Verify**: `npm test` → 2/2 pass. Rust + Python suites unchanged.

**Reveals**: Need to address the e2e ReadTimeout and add a CI workflow that actually runs tests. Next loop: make `test_e2e_queries` skip-or-pass deterministically when no LLM is configured, OR mock the LLM there.

---
## Loop 2 — Chatbot e2e: deterministic mock-backend escape hatch

**Observe**: Loop 1 left UI green, Rust green, but `chatbot/tests/test_e2e.py::test_e2e_queries` still fails — uvicorn subprocess auto-selects ollama, real LLM call, 20s per-request timeout × 10 queries → ReadTimeout.

**Orient**: Highest-leverage option is making the running server use the deterministic mock backend without a code-only test patch. That also delivers a real ops feature: an env-var escape hatch for offline/CI smoke tests.

**Decide**: Honor `AMBARA_FORCE_MOCK_LLM=1` in `LLMClient.__init__` to override auto-selection; set the env in the e2e subprocess.

Candidates considered and rejected:
- Skip when no LLM reachable → hides regressions, weaker.
- Mock `requests` in test → couples test to internal HTTP shape.
- Generic `AMBARA_LLM_BACKEND=mock|openai|...` → broader but scope creep; do narrow first.

**Devil**:
- Correctness: env override only takes effect when explicitly set; existing `force_mock=False` callers unchanged when env unset; `force_mock=True` constructor still works (OR’d). ✓
- Scope: addresses both symptom (timeout) and capability gap (no offline override). Right depth. ✓
- Priority: clears the last red test and adds production value. Higher impact than chasing warnings. ✓

**Act**:
- `chatbot/generation/llm_client.py`: read `AMBARA_FORCE_MOCK_LLM` env var; OR with `force_mock` arg; store on `self.force_mock`. Updated docstring.
- `chatbot/tests/test_e2e.py`: spawn uvicorn with `AMBARA_FORCE_MOCK_LLM=1` in env.

**Verify**:
- `pytest chatbot/tests/test_e2e.py` → 1 passed in 3.93s (was 280s timeout).
- `pytest chatbot/tests` → **106 passed**, 0 failed.
- UI + Rust unchanged from loop 1.

**Reveals**: Test runtime still high (181s) due to many corpus/embedding tests. CI workflow gap remains — no automated test runs on PRs. Next loop: add a minimal CI workflow to actually run `cargo test` + UI vitest + chatbot unit tests on PRs. Skip slow integration paths (corpus build, e2e) on PR runs but keep on main.
