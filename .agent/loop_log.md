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

## Loop 7 — Promote clippy to a CI gate

OBSERVE: Loop 6 left the codebase at 0 compiler warnings + 0 clippy lints (under `--lib --tests`). Verified `cargo clippy --all-targets --workspace` is also clean (1m21s cold build, exit 0, zero warnings). The `tests.yml` rust job currently runs only test targets.

ORIENT: Adding `cargo clippy --all-targets -- -D warnings` now is exactly the kind of "lock in" step that loops 5-6 set up. Without it, a future contributor could re-introduce dead imports or subtle lints (the kind that took loops 4-6 to clean up), and CI would happily merge the PR. With clippy-deny in CI, that whole class of regression is impossible.

DECIDE candidates:
  1. Add `cargo clippy --all-targets -- -D warnings` to the rust job. (Chosen.)
  2. Add `cargo clippy --workspace --all-targets -- -D warnings`. Slightly broader (covers `plugins/comfyui_bridge` and `ui/src-tauri`); but `ui/src-tauri` is excluded from the standard `cargo` invocation by default in this repo, and its clippy hygiene is best gated when the ui job evolves to include tauri-build smoke. Prefer the narrower scope here.
  3. Add a separate `lint` job. More structure, but for a single command it is overkill; keeping it in the rust job means a single rust-toolchain setup serves both test and clippy.
Picked (1).

DEVIL'S ADVOCATE:
- Correctness: Verified locally that `cargo clippy --all-targets -- -D warnings` exits 0 from a cold build. YAML validated by `python3 -c yaml.safe_load`. Rust toolchain action already pinned to stable, and `Swatinem/rust-cache@v2` covers clippy artifacts.
- Scope: Could a future stable rustc release introduce a new lint that fails our build? Yes — that is the cost of `-D warnings` on stable. Tradeoff accepted: a periodic small fix in exchange for never re-litigating dead imports and approx-constant false positives. The mitigation is that any failure is local to a PR or a daily push, and is fixed by either fixing the lint or `#[allow]`-ing it with a justifying comment.
- Priority: Cycle audit in `graph/structure.rs` is still next-loop. Still defensible: clippy gate is one-line of CI yaml versus a multi-test investigation; it's the order of operations that maximizes leverage per loop.

ACT: Edited `.github/workflows/tests.yml` to add a `cargo clippy --all-targets -- -D warnings` step in the rust job after the test steps. Validated YAML, reproduced gate locally → exit 0. Test status unchanged: Rust 134 / UI 2 / chatbot 106.

REVEALS:
- `ui/src-tauri` is its own workspace member but never tested or clippy-checked in CI. Worth a future loop once the tauri test surface is defined.
- README still has no CI badge; cosmetic but cheap follow-up.
- Cycle-prevention audit in `src/graph/structure.rs` is unblocked and is the highest-impact remaining item on the queue.

## Loop 8 — Cycle-prevention regression coverage

OBSERVE: `would_create_cycle` (`src/graph/structure.rs:378-381`) delegates to `is_reachable(to_node, from_node)` — BFS from `to_node` checking if `from_node` is reachable. `is_reachable` short-circuits true when `start == target` (line 385-387). Walked through the algorithm by hand for self-loops and 2-node back-edges; both are correctly rejected. Existing test `test_cycle_detection` (line 527) covers only the long-chain case (A→B→C, then C→A).

ORIENT: The cycle-prevention algorithm is a priority-1 surface (a corrupt graph would silently violate every downstream invariant: topological_sort, parallel_batches, validation, execution scheduling). It is correct today — but its behavior on the most error-prone corner cases (self-loops, minimum-length cycles) is enforced only by inspection. CI gating without coverage of these corners means a future "performance refactor" of `is_reachable` could silently break the contract.

DECIDE candidates:
  1. Add `test_self_loop_rejected` and `test_two_node_back_edge_rejected`. (Chosen.)
  2. Refactor `is_reachable` to skip enqueuing already-visited nodes (cosmetic perf, no behavior change). Skipped — code is already correct and the refactor isn't justified by a measured problem.
  3. Skip — code is correct, tests are nice-to-have. Rejected because regression coverage on a priority-1 surface is exactly what the operating-law priority order calls "test gaps on existing functionality".

DEVIL'S ADVOCATE:
- Correctness: Traced both cases:
    * Self-loop `connect(n, "output", n, "input")` → `would_create_cycle(n, n)` → `is_reachable(n, n)` short-circuits at line 385 → `Err(CycleDetected)`. Confirmed by test pass.
    * 2-node back-edge: `connect(a, b)` succeeds. Then `connect(b, "output", a, "input")` → `would_create_cycle(b, a)` → `is_reachable(a, b)` → BFS from a visits b, returns true → `Err(CycleDetected)`. Confirmed.
  Tests also assert `connection_count()` to ensure rejected edges are NOT persisted (catches a class of bug where the cycle check fires AFTER the edge is pushed).
- Scope: Tests-only loop. No production change. Symptom (no test) addressed; cause (algorithm correct but unprotected) addressed.
- Priority: Could there be a real cycle-detection bug we're missing? Audited the BFS: `if visited.insert(current) { ... }` correctly idempotently visits each node once; queue can contain duplicates but they're filtered at insertion. Correct. The deeper question — does `connect()` order check ports BEFORE running cycle detection? Lines 257-289 verify ports/types/already-connected first, THEN line 292 cycle-checks. So a connection between nonexistent ports raises a different error than a cycle, which is correct. Audit closed.

ACT: Added `test_self_loop_rejected` and `test_two_node_back_edge_rejected` to `src/graph/structure.rs`. `cargo test --lib graph::structure` → 7/7. Full `cargo test --lib` → 136/136 (was 134/134 + 2 new). `cargo clippy --all-targets -- -D warnings` → clean.

REVEALS:
- The new tests caught nothing today, but they would catch the most likely future regressions in a perf refactor of `is_reachable`. Worth the four lines of YAML in CI plus the two tests.
- Loop 8 raises an architectural question worth queuing: connecting two ports of the SAME node (e.g. node has two inputs `a` and `b`, output wired to its own `a`) — currently rejected by the cycle check. Confirm this is intentional vs a side-effect of `start == target` short-circuit. Most filter graphs do not allow self-feedback by design, but loop scheduling for some processing models (e.g. RNN-shaped pipelines) might. Probably out of scope for ambara; queue as an architectural-doc todo.
- `is_reachable` is `pub` but `would_create_cycle` is private. Public reachability has uses (UI dependency tracking) — confirms the API split is intentional.

## Loop 9 — Remove `graph_generator_legacy.py` dead code

OBSERVE: Bootstrap flagged `chatbot/generation/graph_generator_legacy.py` (622 lines) as suspected dead code. The file's own docstring states: "DEPRECATED: Legacy single-shot graph generation pipeline... No code imports this module. Safe to delete after verifying no external tools depend on it." `CHANGELOG.md:126` documents that the new `graph_generator.py` replaced it and the old version was "backed up as graph_generator_legacy.py". Comprehensive grep across `.py`, `Dockerfile.chatbot`, `docker-compose*.yml`, `scripts/`, `*.md`, `*.toml`, `*.json`, `*.yml`, `*.yaml` confirmed zero importers anywhere except the agent state files and the CHANGELOG entry.

ORIENT: 622 lines of unimported code is more than mere clutter — it imports `graph_validator`, `llm_client`, `prompt_builder`, `repair_prompt_builder`, `GenerationResultModel`, `ExampleRetriever`, and `FilterRetriever`. Every refactor of any of those modules has to keep this file's call sites compilable, or hide the truth that the legacy file's lint output is silently ignored. That is false coupling: the freedom to refactor active code is constrained by code no one runs.

DECIDE candidates:
  1. Delete the file. (Chosen.)
  2. Move to a `legacy/` directory. Reduces visibility but preserves the false-coupling problem (still imported by IDE indexers, still adds maintenance load).
  3. Leave with a stronger deprecation comment. Doesn't unblock the refactor freedom argument.

DEVIL'S ADVOCATE:
- Correctness: Zero importers verified across all relevant file types. Deletion cannot affect runtime. Confirmed by running `pytest chatbot/tests -q` after deletion: 106 passed in 209.59s, identical to pre-deletion. CI workflow added in loop 5 already runs the same suite.
- Scope: One-file deletion. No active code touched.
- Priority: Could there be a hidden consumer? Tutorials / blog posts could reference the file path — but the file's own docstring acknowledges that risk and concludes "Safe to delete". CHANGELOG already records the supersession event so future archaeologists have a breadcrumb. Deletion is reversible (git history preserves the file).

ACT: `git rm chatbot/generation/graph_generator_legacy.py` (622 lines removed). `pytest chatbot/tests -q` → 106/106 pass in 209s. No code/test changes required.

REVEALS:
- Now zero "_legacy" / "_old" / "_backup" files in chatbot/. Repo hygiene improved.
- The 7 imports the legacy file made are all still used by active code — no transitive cleanup is now possible. (Verified by spot-check: `graph_validator` imported by `agent.py`, `llm_client` imported widely, etc.)
- Architectural item still queued: validate `ValidationReport::can_execute()` semantics for empty graphs.

## Loop 10 — Version drift (manifest catch-up to CHANGELOG)

OBSERVE:
- Manifests at v0.5.0 across `Cargo.toml`, `ui/src-tauri/Cargo.toml`, `ui/package.json`, `ui/src-tauri/tauri.conf.json`.
- Latest git tag `v0.7.0`; CHANGELOG documents v0.7.0, v0.7.1, v0.8.0, v0.9.0 as released, plus v0.9.1 "Unreleased".
- Spot-checked CHANGELOG claims: `AMBARA_CORS_ORIGINS` env in `chatbot/api/main.py:99`; WS reconnect in `useChatApi.ts`. Both real → CHANGELOG is the source of truth, manifests are 4 minor versions stale.

ORIENT:
- Real downstream impact: bug reports show wrong version, `cargo audit` semver math is off, tauri bundle metadata lies.
- Missing git tags for v0.7.1/0.8.0/0.9.0 is a release-process gap, not a code gap. Out of scope (creating tags requires deliberate maintainer action).

DECIDE:
- Candidates:
  (a) Bump manifests to v0.9.0 to match latest released CHANGELOG entry. ← chosen
  (b) Bump to v0.7.0 to match latest git tag. Loses 0.8/0.9 information.
  (c) Bump to v0.9.1. Premature — that entry is "Unreleased".

DEVIL:
- Correctness: 4 metadata edits, no logic touched. Verified two CHANGELOG features exist in code; confidence high that v0.9.0 is the right stake.
- Scope: doesn't fix the missing git tags, but creating tags retroactively is a maintainer judgment call. Documented as queued.
- Priority: nothing more impactful pending — clippy gate live, CI gates live, dead code purged. Manifest lies block accurate bug triage.
- Subtle: `Cargo.lock` for top-level workspace pkg doesn't pin its own version; verified `cargo build --lib` produces a clean lockfile (no diff). 136/136 tests still pass.

ACT:
- `Cargo.toml`: 0.5.0 → 0.9.0
- `ui/src-tauri/Cargo.toml`: 0.5.0 → 0.9.0
- `ui/package.json`: 0.5.0 → 0.9.0
- `ui/src-tauri/tauri.conf.json`: 0.5.0 → 0.9.0
- `cargo build --lib` → clean; `cargo test --lib` → 136/136 pass.

REVEALS:
- Missing git tags v0.7.1/0.8.0/0.9.0 — queued as release-process item for maintainer.
- README v0.9.0 reference (line ~309 region) now consistent with manifests.

## Loop 11 — topological_sort contract tests

OBSERVE:
- `topological_sort` (Kahn's, src/graph/topology.rs:26-78) had only one test (`test_topological_sort` chain A→B→C). Empty/single/disconnected cases unverified.
- Cycle branch (line 74) is unreachable through public API: `connect()` rejects all cycles via `would_create_cycle` (loop 8 confirmed). No way to inject a cycle without a test backdoor.

ORIENT:
- Three reachable contract gaps: empty Ok([]), single node [n], disconnected pair {A,B}.
- Cycle branch coverage requires either (a) a `pub(crate)` test backdoor in structure.rs that bypasses cycle check, or (b) marking the branch dead. Both are larger architectural decisions; queued.

DECIDE:
- Three reachable empty/single/disconnected tests. ← chosen
- Backdoor for cycle test. Rejected: increases prod surface for a defensive branch.
- Replace branch with unreachable!(). Rejected: requires proving no future caller can desync `node_count` from connections; not safe today.

DEVIL:
- Correctness: HashMap iteration in Kahn's makes Vec order non-deterministic for disconnected graphs. Used HashSet+contains for the disconnected case; equality only where order is fixed (single node). Empty case asserts is_empty() not equality with vec![].
- Scope: Doesn't close cycle-branch gap, but explicitly logs why and queues it as architectural. Not papering over.
- Priority: Could this loop have done the cycle backdoor? Yes, but that's a real architectural change deserving its own loop and devil step. Splitting is correct.

ACT:
- Added `test_topological_sort_empty_graph`, `test_topological_sort_single_node_no_connections`, `test_topological_sort_disconnected_nodes` to src/graph/topology.rs#tests.
- All three pass; full suite 139/139 (was 136); clippy clean.

REVEALS:
- The cycle-detection branch in `topological_sort` is currently unreachable through the public API. Either the branch is genuinely dead (and could be `unreachable!()` with a safety comment) or it's defensive against a future bypass. Decide architecturally next loop or queue.
- `has_cycle()` is also untestable for the `true` case for the same reason.

## Loop 12 — README CI badge

OBSERVE: README has no badges. tests.yml workflow exists since loop 5. Repo slug PrakyathPNayak/ambara (env context).

ORIENT: Pure visibility win — anyone landing on the README sees CI status without clicking through.

DECIDE: One-line markdown badge under the `# Ambara` H1, linking the badge.svg to the workflow run page. Chosen over (a) badge in a "Status" section (over-engineered for one badge) and (b) multiple badges including license/version (premature; can grow later).

DEVIL:
- Correctness: GitHub serves /actions/workflows/<file>/badge.svg automatically. Filename `tests.yml` and slug confirmed.
- Scope: Doesn't fix the stale "250 tests (144 Rust + 106 Python)" claim in the features list (current Rust lib count is 139, plus ignored/integration tests not yet audited). That deserves its own loop with a full count audit.
- Priority: Smaller than the queued can_execute() semantics fix, but that fix changes behavior + a loop-3 test, so it deserves a dedicated loop too. Splitting is correct.

ACT: Inserted badge line in README.md right under the H1.

REVEALS:
- Stale test-count claim in README features list ("250 tests / 144 Rust"). Audit counts across `cargo test` (lib + doc + integration + workspace), `cd ui && npm test`, and `pytest chatbot/tests` for the next docs loop.

## Loop 13 — Extend Rust CI to cover ui/src-tauri workspace member

OBSERVE:
- Initial DECIDE was can_execute() empty-graph semantics.
- Devil step: grep showed zero production callers of can_execute() — only internal tests reference it. Fixing semantics on unused infrastructure is low leverage. REPRIORITIZED to ui/src-tauri CI gating.
- CI step in tests.yml line 28/34 runs `cargo test --lib` and `cargo clippy --all-targets` without `--workspace`. Workspace members `plugins/comfyui_bridge` and `ui/src-tauri` (the desktop binary that ships to users) had zero CI coverage. A clippy regression or test failure in the tauri bin would slip through.

ORIENT:
- Local verification: `cargo clippy --all-targets --workspace -- -D warnings` clean; `cargo test --workspace --lib` 139 main + 0 plugins + 0 tauri. Workspace expansion is currently safe.
- Tauri requires Linux system deps (libwebkit2gtk-4.1-dev et al.) to compile. CI runner needs them installed before clippy.

DECIDE:
- Add `--workspace` to clippy and test commands.
- Add an `apt-get install` step for tauri system deps before the cargo steps.
- Keep `cargo test --doc` as-is (workspace flag rarely matters for doc tests in this layout).
- Rejected: keeping CI scoped to main crate only (current behavior — leaves real bug surface uncovered).
- Rejected: separate tauri-only job (over-engineered for one binary that uses the same toolchain).

DEVIL:
- Correctness: Tauri v2 system-dep list cross-checked against tauri docs (libwebkit2gtk-4.1-dev is right for ubuntu-latest=24.04). yaml validated.
- Scope: Three coupled changes in one yaml file — all part of the same "extend rust CI to cover ui/src-tauri" stake. Cohesive.
- Priority: Initial pick was can_execute(); reprioritized after devil revealed it has no production callers. This change closes a real coverage hole on the binary that ships to users. Higher impact.
- Subtle: CI runtime increases (~30-60s for apt + extra compile). Acceptable cost for the coverage.

ACT: Edited `.github/workflows/tests.yml` rust job to install tauri deps, then `cargo test --workspace --lib` and `cargo clippy --all-targets --workspace -- -D warnings`. yaml lint passes.

REVEALS:
- `plugins/comfyui_bridge` and `ui/src-tauri` have zero tests today. Once CI gates them, adding even one smoke test per member would ratchet quality. Queued.
- can_execute() has zero production callers (deferred from this loop's initial pick). Either delete the method or wire it into a real call path. Queued.

## Loop 14 — README test-count audit

OBSERVE:
- README claimed "250 tests (144 Rust + 106 Python)" — pre-loop-11/13 numbers, possibly an estimate.
- Authoritative breakdown via `cargo test --workspace` per-target output:
    main lib       139 (+2 ignored)
    src/main.rs      2
    comfyui_bridge   0
    ui_lib           0
    ui main          0
    doc tests        8 (+11 ignored)
  Rust total: 149 active.
  UI vitest:    2
  Python:     106
  Grand total active: 257.

ORIENT: claim was off by 7, omitted UI vitest entirely.

DECIDE: Update to "257 tests (149 Rust + 2 UI + 106 Python)". One-line edit.

DEVIL:
- Correctness: counts from direct tool output, not estimation.
- Scope: README only; no other files reference these numbers.
- Priority: small but unblocks loop-12's reveal queue.
- Subtle: counts drift again on next test addition. Acceptable — README is scope, CI is truth.

ACT: edited README features list to the audited counts.

REVEALS: Cargo test discovers EIGHT test targets across the workspace; only two have any tests (main lib + main bin). plugins/comfyui_bridge and ui/src-tauri are still test-empty (loop 13 reveal still standing).

## Loop 15 — comfyui_bridge smoke tests

OBSERVE:
- `plugins/comfyui_bridge/src/lib.rs` is a 95-line FFI vtable scaffold (10 unsafe extern "C" fns, one `#[no_mangle] pub static PluginVTable`). Zero tests. Loop 13 made CI gate it but there was nothing to assert.

ORIENT:
- The placeholder is meaningful surface — its vtable is the contract the host depends on. ABI version drift, double-free, and "scaffold quietly grew real filters" are real regression risks.

DECIDE:
- Four tests through `ambara_plugin_vtable`: ABI version equality, create→destroy roundtrip, null-handle destroy is no-op, stub reports 0 filters + Ok health. Chosen over (a) skipping (loop 13 reveal stays open) and (b) integration tests against a real ComfyUI server (out of scope; needs network).

DEVIL:
- Correctness: Each call wrapped in unsafe block as required. plugin_destroy must accept null per scaffolds and the existing `if !handle.is_null()` guard at line 19 — verified by reading; test 3 exercises it.
- Scope: 4 tests + one README count bump.
- Priority: Loop 13 reveal called for >0 tests on each workspace member. This loop addresses comfyui_bridge; ui/src-tauri smoke test queued for next.
- Subtle: filter_id_at returning null at index 0 on an empty plugin is by design (line 36-38). Documented in test name.

ACT:
- Added `#[cfg(test)] mod tests` with 4 tests.
- `cargo test -p comfyui_bridge` → 4/4 pass.
- `cargo clippy --all-targets --workspace -- -D warnings` → clean.
- README test count: 149 → 153 Rust, 257 → 261 total.

REVEALS:
- ui/src-tauri still has zero tests — same gating gap.
- The plugin scaffold's `filter_execute` returns `ErrNotSupported`. When real filters are added, the test asserting "0 filters" must be replaced with the real count to avoid masking implementation drift.
