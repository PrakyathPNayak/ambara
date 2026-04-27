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

## Loop 4 — Strengthen `test_parallel_batches`

OBSERVE: Rust 134/134 + UI 2/2 + chatbot 106/106 (last verified Loop 2). Cargo test compile warnings: `ImageDataRef` unused, `c`/`d` unused in `topology.rs:253-254`. The `c`/`d` warning was a tell: those vars were created in `test_parallel_batches` but never wired into the graph. The test only ran a single A→B connection and then asserted `!batches.is_empty()` — vacuously true unless `parallel_batches` panics. The test name promises something it does not deliver.

ORIENT: This is priority-3 territory: `parallel_batches` is the algorithm that drives parallel execution scheduling, and its only test verifies essentially nothing. A regression in the depth grouping (e.g. miscounting source nodes, miscomputing depth for diamond/fan-out) would silently pass.

DECIDE candidates:
  1. Fan-out + isolated source: `A→B, A→C, D` standalone → batches `{A,D}`, `{B,C}`. Tests fan-out parallelism AND that isolated source/leaf nodes get depth 0. (Chosen.)
  2. Two parallel chains: `A→B, C→D` → batches `{A,C}`, `{B,D}`. Tests parallelism but not fan-out.
  3. Just remove unused vars, keep weak assertion. Doesn't add coverage.
Picked (1): the fan-out branch in `connect()` (one output to many inputs) was untested, and isolated nodes exercise the `connections_to(node).next().is_none() → depth 0` source-detection branch. PassthroughNode has only one input port (`PortType::Any`), confirmed in `core/node.rs:453`, so a true diamond merge isn't expressible — the test comment that misled the original author is now corrected.

DEVIL'S ADVOCATE:
- Correctness: Walked through `parallel_batches` (`topology.rs:84-120`). Source detection at line 99 uses `connections_to(node).next().is_none()`, which is true for both A and D in the new graph. depth(A)=0, depth(D)=0, depth(B)=max_parent_depth(A)+1=1, depth(C)=1. Batches grouped by depth, empty buckets retained then dropped by the `retain` filter. Expected exactly `[{A,D}, {B,C}]`. Cargo test pass confirms.
- Scope: Symptom was unused vars; root cause was a placeholder test pretending to cover parallel-batch logic. Fix addresses cause, not symptom.
- Priority: Could there be a real bug in `parallel_batches`? Read again — looks correct. But the test now actively defends it. Considered going deeper to audit `topological_sort` cycle behavior; that test (`test_topological_sort`) is already strong (asserts ordering of pos1<pos2<pos3). Single-loop scope respected.

ACT: Replaced `test_parallel_batches` with the fan-out + isolated source pattern, used `c`/`d`, and asserted exact batch composition (size + membership). `cargo test --lib graph::topology` → 4/4. Full lib test → 134/134. UI vitest → 2/2. Warnings 3 → 1 (only `ImageDataRef` unused-import remains).

REVEALS:
- `core/batch.rs:162` `ImageDataRef` unused-import — last remaining warning, queued.
- `topological_sort` has no test for the empty graph or for graphs with cycles being rejected — possible follow-up.
- No test workflow on PRs; tests-as-PR-gates is queued.
- `chatbot/generation/graph_generator_legacy.py` may be dead code.

## Loop 5 — Add CI test workflow

OBSERVE: Loop 1-4 fixes (jsdom scrollIntoView guard, e2e mock-LLM env override, validation contract asserts, parallel-batches coverage) all rest on local-only verification. The only CI workflow `.github/workflows/build-release.yml` runs on tags/dispatch and only does `npm run tauri build` — never runs tests. A future contributor (or a stale branch) can regress any of those fixes without anything catching it.

ORIENT: This is a meta-priority loop. Adding CI doesn't fix a bug today, but it locks in every previous fix and gates every future change against the test suites that already exist. Highest leverage among open candidates.

DECIDE candidates:
  1. Three-job tests workflow (Rust + UI + Chatbot) on push/PR. (Chosen.)
  2. Rust-only — easy and fast, but UI + chatbot already pass locally and are at higher regression risk.
  3. Rust+UI+Chatbot+Clippy with `-D warnings` — would currently fail because of the leftover `ImageDataRef` warning in `core/batch.rs:162`. Add clippy in a later loop after warnings are zero.
Picked (1): broadest protection that doesn't fail-out-of-the-gate. Set `AMBARA_FORCE_MOCK_LLM=1` (added in loop 2) on the chatbot job and explicitly blank LLM API keys to keep CI deterministic. Cache HuggingFace + cargo + npm to keep wall time reasonable.

DEVIL'S ADVOCATE:
- Correctness: Each command is verified locally. `cargo test --lib` (134/134), `cd ui && npm test` (2/2), `pytest chatbot/tests` (106/106 in ~3 min). YAML parses (validated with `python3 -c yaml.safe_load`). `npm ci` requires `ui/package-lock.json` — confirmed present. `cache-dependency-path` set correctly for both. `--no-fail-fast` so one failing rust test doesn't hide others.
- Scope: Could the workflow itself be wrong? Possible failure modes:
    * `dtolnay/rust-toolchain@stable` and `Swatinem/rust-cache@v2` — both widely used, stable.
    * HuggingFace cache may be cold on first run; chatbot job will be slow once but cached after. Acceptable.
    * `pip install -r chatbot/requirements.txt` includes `chromadb` and `sentence-transformers` — heavy but installable on Ubuntu CI.
    * Removed dependence on missing `chatbot/requirements-dev.txt` by `pip install pytest` directly.
- Priority: Could there be a real bug we're masking by doing meta-work? The cycle-prevention audit in `graph/structure.rs` is still pending — but adding CI is exactly the right precondition for invasive code audits, so the order is defensible.

ACT: Created `.github/workflows/tests.yml` with three jobs (`rust`, `ui`, `chatbot`), `concurrency` group to cancel superseded PR runs, lockfile-aware caching, deterministic chatbot env. YAML validated via `python3 -c "yaml.safe_load(...)"`. No production code touched, so test status unchanged: Rust 134 / UI 2 / chatbot 106.

REVEALS:
- `core/batch.rs:162` `ImageDataRef` import still warning — once cleared, can add `cargo clippy -- -D warnings` to CI.
- Workflow has no `cargo build --release` job; not needed for test gating, but a smoke build on PR could catch tauri-config breakage. Queue.
- chatbot job will be slow on cold runners until HF cache warms. Acceptable cost for first deployment of CI.
- No `tests.yml` badge in README. Cosmetic, low priority.

## Loop 6 — Zero Rust warnings (compiler + clippy)

OBSERVE: After loop 4, only the `ImageDataRef` unused-import warning at `src/core/batch.rs:162` remained. Confirmed exactly one occurrence in the file (`grep -c ImageDataRef` → 1) — pure dead import with no cfg-conditional uses. Bonus check: ran `cargo clippy --lib --tests` for the first time. Three identical warnings (`approx_constant` on `3.14` literals at `src/core/types.rs:694`, `:696`, `:729`); the floats are not π in intent — they are arbitrary non-integer test values. False positives, but `clippy::all` is `warn` per `src/lib.rs:106`.

ORIENT: Without the loop-5 CI workflow being clippy-enforced, no future loop can rely on `-D warnings`. Clearing both compiler warnings AND clippy warnings is the unblocker; once at zero, a single-line addition to `tests.yml` can promote clippy to a gate.

DECIDE candidates:
  1. Remove `ImageDataRef` only (minimal scope, leaves 3 clippy false positives).
  2. Remove `ImageDataRef` + change `3.14` test literals to `2.5` (still trivial; preserves test intent — these were never PI-relevant).
  3. Remove `ImageDataRef` + add `#[allow(clippy::approx_constant)]` to those tests (preserves the literal but signals "yes we know"). Worse than (2) because the literal carries no semantic meaning.
Picked (2): single edit per file, total of 4 lines changed across 2 files; preserves test intent (`Value::Float(N) where N is non-integer`) and unlocks the clippy gate without an allow-list growing over time.

DEVIL'S ADVOCATE:
- Correctness: `ImageDataRef` was imported but unreferenced (verified by grep count). The `3.14 → 2.5` change preserves test semantics: both tests are checking that a non-integer float matches `PortType::Float` and does not match `PortType::Integer`. The exact float value is irrelevant — only "non-integer" matters. `2.5` is exactly representable in IEEE-754, so no rounding artifacts could surprise a future maintainer either.
- Scope: I expanded scope from "fix one warning" to "fix one warning + three clippy lints". Defensible because all four are zero-risk single-character edits in tests, and the bigger payoff (CI clippy gate) requires all four. Did NOT expand to silencing-by-allow, did NOT touch any product code.
- Priority: Cycle audit in `graph/structure.rs` is still queued at priority 1 if a real bug exists there. Did this loop block it? No — the audit benefits from clippy-clean baseline, and the audit itself is a multi-test affair that deserves its own loop with proper devil-step on each new test.

ACT: Removed `ImageDataRef` from `src/core/batch.rs:162` import list. Replaced two `3.14` literals with `2.5` in `src/core/types.rs:694` and `:696` (test_port_type_matching), and one in `:729` (test_value_type_inference). `cargo test --lib` → 134/134, `cargo clippy --lib --tests` → 0 warnings.

REVEALS:
- Now safe to add `cargo clippy --lib --tests -- -D warnings` to the rust job in `tests.yml`. Queue for next loop.
- Doc tests (`cargo test --doc`) and integration tests (`cargo test --test '*'`) not yet run under clippy. Most likely also clean but worth a verification before tightening CI.
- `Cargo.toml` still 0.5.0; README references v0.9.0; tags up to v0.7.0. Version drift unresolved.
