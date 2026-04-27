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

## Loop 3 — Replace tautological asserts in validation tests

OBSERVE: Rust 134/134 + UI 2/2 + chatbot 106/106 green. cargo test compile shows 5 warnings pre-loop. Two tests (`test_empty_graph_validation` in `src/validation/pipeline.rs`, `test_validation_pipeline` in `src/lib.rs`) use vacuous asserts: `report.duration_ms >= 0` (u64, always true) and `errors.is_empty() || !errors.is_empty()` (A || !A). They cover the validation pipeline — a critical path — but verify nothing.

ORIENT: Highest-impact bug hiding in plain sight is silent test no-ops on critical-path code. A regression that broke `ValidationPipeline::validate` for empty graphs (e.g. panicked, returned errors, lost the warning) would not be caught by these tests today.

DECIDE: Replace both asserts with the documented contract derived from `StructuralValidation::validate`: empty graph → `success=true`, `errors` empty, at least one warning ("Graph is empty"), `can_execute()=true`. Also dropped now-unused `use std::collections::HashMap` in `src/lib.rs` tests module (was only there for prior aspirational code). Candidates considered: (a) delete tests (loses coverage), (b) leave bad asserts and add new tests (clutter), (c) upgrade in place (chosen).

DEVIL'S ADVOCATE:
- Correctness: New asserts must match real behavior. Verified by reading `StructuralValidation::validate` (lines 39-54): empty graph short-circuits with one warning, no errors. Subsequent stages don't run on empty graphs because there are no nodes/connections to inspect, so success stays true. Cargo test pass after change confirms.
- Scope: This is a test-quality fix; it doesn't change product code. Could mask a deeper issue that the empty-graph-emits-warning behavior is itself wrong (a graph with zero outputs maybe shouldn't be "executable"). But that's a contract question, not a regression — out of scope. Filed for future loop.
- Priority: Was there something more important? Cycle prevention in `graph/structure.rs` is still unaudited (potential priority 1). But that's a multi-file investigation; this loop's fix is small, surgical, and removes two silent-pass tests that were actively hiding regressions on every previous loop. Reasonable single-loop pick.

ACT: Edited `src/validation/pipeline.rs::test_empty_graph_validation` and `src/lib.rs::test_validation_pipeline` to assert the real contract; dropped unused `HashMap` import. `cargo test --lib` → 134/134. Warning count 5 → 3 (HashMap gone; remaining: `ImageDataRef`, two unused vars `c`/`d` in topology tests).

REVEALS:
- Unused `c`/`d` in `graph/topology.rs` tests at lines ~253-254 likely indicate a test that was reduced and lost coverage — high priority next loop (priority 3 territory if it masks a topology bug).
- `chatbot/generation/graph_generator_legacy.py` still suspected dead code.
- No CI test workflow yet; warnings + tests are not gated on PRs.
- `ValidationReport::can_execute()` returns `success`, which is true for empty graphs. That semantic might be wrong (you cannot execute a graph with no nodes), but is the documented contract today.
