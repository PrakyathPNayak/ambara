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

## Loop 16 — ui/src-tauri smoke tests

OBSERVE:
- ui/src-tauri/src/lib.rs is 899 lines with ~16 #[tauri::command] fns; src/main.rs is 6 lines (binary entry). Zero tests.
- Several command fns are pure (no `State<>`/Tauri runtime): `get_external_api_capabilities`, `export_graph_json`, `import_graph_json`. Real production surface — the desktop bin's external API contract.

ORIENT: Pure command fns are testable today. The export→import roundtrip is high-impact: it's the contract every external API client crosses.

DECIDE: Four tests:
  1. capabilities advertise api_version "v1" + the correct boolean profile (regression-catches accidental capability flips);
  2. export_graph_json → import_graph_json roundtrip preserves graph content (compared as serde_json::Value because no PartialEq);
  3. import accepts raw GraphState for backward compat (the explicit fallback at line 446);
  4. import rejects malformed JSON.

DEVIL:
- Correctness: First attempt misnamed FilterNodeData fields (used `filter_id` + HashMap-typed `parameters`); compiler caught it. Real shape is `filter_type` String, `category` String, `parameters: Vec<ParameterValue>`, `is_valid`/`error_message` Option<>. Fixed.
- Scope: 4 tests + sample_graph helper. Cohesive within tauri lib.
- Priority: Closes loop-13 reveal for ui/src-tauri. Roundtrip test catches the highest-impact regression class on this surface (envelope schema drift).
- Subtle: serde_json::Value comparison is order-sensitive on object keys; both sides go through the same Serialize impl, so order is deterministic. Safe.

ACT:
- Added `#[cfg(test)] mod tests` with 4 tests at end of ui/src-tauri/src/lib.rs.
- `cargo test -p ui` → 4/4 lib pass, 0 main, 0 doc.
- `cargo clippy --all-targets --workspace -- -D warnings` clean.
- README test count: 153 → 157 Rust, 261 → 265 total.

REVEALS:
- The `FilterNodeData` shape exposed to JS (camelCase via serde) is wide (8 fields); test coverage on its (de)serialization is currently driven only by the roundtrip test. A schema-version test (snapshot of one canonical filter node JSON) would catch silent field-rename breakage.

## Loop 17 — ARCH: cover the unreachable cycle branch in topological_sort

OBSERVE:
- src/graph/topology.rs:67-75 cycle branch unreachable through public API (loop 11 reveal). `connect()` is the only mutator of `self.connections`; cycle check at line 292 rejects all cycles. Confirmed by re-reading.

ORIENT: Two valid options — `unreachable!()` removes dead code but loses defensive value; test backdoor preserves the safety net AND lets it be tested. The latter has lower risk because `#[cfg(test)]` is fully gated out of release.

DECIDE: Test backdoor (option 2). Add `#[cfg(test)] pub(crate) fn force_unchecked_connect` to structure.rs; add a cycle-injection test in topology.rs.

DEVIL:
- Correctness: `#[cfg(test)]` ensures the helper compiles only under `cargo test` — never ships in published artifacts. `pub(crate)` keeps it crate-internal. Verified by reading and by clippy --release implicit behavior.
- Scope: One helper + one test. Cohesive within the loop's stake.
- Priority: Closes loop-11's unreachable-branch reveal definitively. Higher leverage than `unreachable!()` because:
    a) preserves the safety net for future maintainers,
    b) gives us actual coverage on the branch.
- Subtle: `force_unchecked_connect` skips port-existence and type checks too. That's correct for the cycle test (we just need a connection topology). Documented in the doc comment.

ACT:
- Added `force_unchecked_connect` (cfg(test), pub(crate)) to src/graph/structure.rs after `connect()`.
- Added `test_topological_sort_detects_injected_cycle` to graph/topology.rs#tests, asserting `Err(CycleDetected)` with both node IDs and `has_cycle() == true`.
- Workspace lib totals: 140 + 4 + 4 = 148 (was 147). README test count bumped to 158 Rust / 266 total.
- clippy --all-targets --workspace clean.

REVEALS:
- The CycleDetected error includes a node list, but the cycle-check call site at structure.rs:294 sets it to `vec![from_node, to_node]` (just the offending edge endpoints). The topological_sort branch at topology.rs:68 sets it to all remaining-in-degree>0 nodes. Two different shapes for the same error variant — semantic drift documented for future review.
- ui/src-tauri tests now run reliably under the workspace CI; cycle backdoor pattern could be reused to test other defensive branches if any exist.

## Loop 18 — Test gap on chatbot LLM retry policy

OBSERVE:
- Existing chatbot tests mock at the GraphGenerator/LLMClient layer; nothing exercised _post_with_retry directly. Every paid-provider call (Anthropic/OpenAI/Groq/Ollama) routes through this function. Priority-3 test gap on a critical path.

ORIENT: Two candidates surfaced —
1. CycleDetected variant doc divergence (loop 17 reveal): doc-fix only, priority 9.
2. LLM retry policy lacks tests: priority 3 on a path that runs every paid call.
(2) wins on priority order.

DECIDE: Add 7 unit tests for `LLMClient._post_with_retry`:
- 200 immediate return (no sleep)
- 503 → 200 retry path
- 503 → 503 final-attempt fall-through (returns 5xx for caller to raise)
- 400 non-retryable passthrough (no sleep)
- 401 / 403 auth failure passthrough (must not burn budget retrying)
- ConnectionError → 200 retry path
- Timeout → Timeout exhaustion → RuntimeError

DEVIL:
- Correctness: I walked through every branch of the for-loop manually before writing tests; the existing logic is correct. Tests confirm by execution. Edge: if requests.post itself raises a non-RequestException (e.g. ValueError from a bad header), the loop won't catch it — that's fine because the public API wraps the helper inside `try: ... except RuntimeError`. Out of scope.
- Scope: Could also fix Retry-After ignoring (real enhancement) or remove the dead code at line 156 (cosmetic). Skipped — the test coverage gap is the higher-leverage fix and locks current behavior in before any refactor.
- Priority: Cannot find anything higher-priority untested. Confirmed.

ACT:
- New file chatbot/tests/test_llm_retry.py with 7 tests, all using `unittest.mock.patch` (already used in test_agent.py / test_session.py — no new deps).
- Patches `chatbot.generation.llm_client.requests.post` and `time.sleep` to keep tests deterministic and fast (2.03s total).
- All pass.
- Python collected: 113 (was 106). README updated to 273 / 113 Python.

REVEALS:
- Line 156 of llm_client.py (`raise RuntimeError(f"... after retries: {last_err}")`) is unreachable — the loop always exits via earlier `return` or `raise`. Cosmetic; queued.
- _RETRYABLE_STATUS does NOT include 401/403, but the helper has no auth-aware short-circuit either. Tests pinned the correct current behavior (no retry on 4xx), but the implementation's correctness here is incidental — it falls out of "not in {429,502,503,504}" rather than from explicit auth handling. Documenting this is fine.
- Retry-After header is ignored on 429. Real client-citizen issue; queued for future.

## Loop 19 — Honor Retry-After header on transient HTTP failures

OBSERVE: Loop-18 reveal — `_post_with_retry` ignored server-supplied
`Retry-After` headers on 429/503, sleeping a flat 2s instead. Real
API-citizen bug; with provider rate limits this can pre-emptively
trigger further throttling.

ORIENT: Two candidates from next.md — Retry-After parsing (priority 5,
real bug) vs CycleDetected doc divergence (priority 9). Retry-After
wins on priority order AND has tests already in place from loop 18 to
guard against regressions during the change.

DECIDE:
1. Add `_parse_retry_after(header_value)` static method:
   - numeric seconds (RFC 7231 §7.1.3 delta-seconds form)
   - HTTP-date (RFC 7231 §7.1.3 HTTP-date form)
   - clamp to `_RETRY_AFTER_MAX_S = 30.0`
   - return None for missing/malformed → caller falls back to constant
   - negative/past date → 0.0 (don't sleep but proceed to retry)
2. Wire into `_post_with_retry`'s 5xx branch only — `RequestException`
   path can't carry a header.
3. Remove confirmed-dead `last_err` tracking + dead final raise. Replace
   the unreachable line with a `# pragma: no cover` defensive guard so
   coverage tooling doesn't flag it.

DEVIL:
- Correctness:
  * `parsedate_to_datetime` may return None or raise on garbage. Guard
    both. `email.utils` doesn't raise TypeError per docs but defensive
    catch is cheap. Confirmed by exercise: malformed test passes.
  * `target.tzinfo is None` check: HTTP-date parsing always returns
    tz-aware in Python 3.10+, but RFC 850 dates without timezone have
    been seen; assume UTC in that case. Pragmatic choice.
  * Negative numeric value → clamp to 0 not None. Server-allowed
    semantics: "you may retry now."
- Scope: I considered also adding exponential backoff on
  RequestException, but loop-18 tests already pin the constant-delay
  contract. Changing that would invalidate them and is unnecessary —
  one retry is enough for transient connection errors.
- Priority: Could the LLMClient's actual behavior under HTTP errors
  hide a worse bug? I read every caller again — all three paid
  providers do `if response.status_code >= 400: raise`. The pattern is
  consistent. No deeper bug.

ACT:
- chatbot/generation/llm_client.py:
  * Imports: datetime, timezone, parsedate_to_datetime
  * New constant `_RETRY_AFTER_MAX_S = 30.0`
  * New `_parse_retry_after` staticmethod
  * `_post_with_retry`: read `response.headers.get("Retry-After")`,
    pass through parser, fall back to `_RETRY_DELAY_S`, log actual delay
  * Removed dead `last_err` tracking; final raise marked `pragma: no cover`
- chatbot/tests/test_llm_retry.py:
  * Updated `_resp` helper to accept headers
  * 7 new tests: numeric, clamp, HTTP-date, malformed, missing,
    negative, exception-path-uses-constant
- Suite: 280 tests (was 273); 158 Rust + 2 UI + 120 Python.
- Verified: all 14 retry tests pass; all 14 generation tests pass.

REVEALS:
- LLMClient's three paid providers (`_generate_anthropic`,
  `_generate_openai`, `_generate_groq`) all repeat the same boilerplate
  body construction and post-then-status-check pattern. Real DRY
  opportunity. Each path is ~25 lines. Queued.
- The Ollama path uses `60` second timeout for a local server. Likely
  aggressive — local models on CPU can take longer. Queued for review.

## Loop 20 — Pin paid-provider response-shape contract before refactor

OBSERVE: Loop-19 reveal queued a DRY refactor of the three paid-provider
paths (Anthropic / OpenAI / Groq). Loop 19's devil step warned: pin the
response-extraction contract with tests BEFORE refactoring. Loop 18's
retry tests cover the helper but not the per-provider shape extraction.
Doing the refactor without these tests = unverifiable correctness.

ORIENT: This is the gating-test loop for the planned refactor. Critical
because:
- Each provider has a unique response shape that the refactor must
  preserve exactly.
- Each provider has unique error semantics (missing key / 4xx / 5xx).
- The Ollama path has special unreachable-wrap semantics.

DECIDE: 18 unit tests across:
- 4 backend-selection tests (pin auto-selection ordering)
- 5 Anthropic tests (extract / system-split / 4xx / empty-content / missing-key)
- 3 OpenAI tests (extract / 5xx / missing-key)
- 3 Groq tests (extract / 4xx / missing-key)
- 3 Ollama tests (extract / empty-content-warning / unreachable-wrap)

Mock at the `_post_with_retry` boundary so we exercise only the
shape-extraction layer, not the network or retry layer.

DEVIL:
- Correctness:
  * caplog initially didn't capture the Ollama warning — pytest config
    (or LOGGER pre-handler) prevented propagation. Switched to mocking
    LOGGER.warning directly. Cleaner anyway: tests should pin the
    LOGGER call, not the captured stderr.
  * Backend-selection tests use clean_env fixture to avoid leaking
    user's actual env keys (would silently flip backend=anthropic).
  * Test_anthropic_separates_system_from_user_messages reaches into
    `call_args.args[2]` which is the body argument. If a refactor
    moves body to a kwarg this test breaks loud — desirable signal,
    not a regression.
- Scope: Why not also test prompt building? That's a separate layer
  with its own tests (`prompt_builder.py`). Not in scope here.
- Priority: This is the gate for loop-21's refactor. Higher priority
  than the refactor itself precisely because it unblocks it.

ACT:
- New file chatbot/tests/test_llm_providers.py with 18 tests.
- Adjusted Ollama warning test to mock LOGGER.warning directly.
- Removed unused `os` import.
- All 18 pass in 0.78s.
- Suite: 298 tests (was 280); Python 138.

REVEALS:
- The auto-selection ordering (Anthropic > Groq > OpenAI > Ollama)
  is now documented by tests. Any future provider addition should
  insert with conscious priority.
- `client.backend = "anthropic"; client.anthropic_key = None` is the
  pattern for testing missing-key errors after construction. Hacky
  but pragmatic — refactor target?
- The Ollama unreachable-wrap (line 119-123 of llm_client.py)
  catches RuntimeError from `_post_with_retry` and re-raises with
  a different message. Pinned by `test_ollama_unreachable_wraps_*`.
  Refactor must preserve this exact wrap.

## Loop 21 — DRY refactor of OpenAI-compatible providers

OBSERVE: Loop-19 reveal queued, loop-20 unblocked (32 tests pinning the
contract). _generate_openai (line 263-296) and _generate_groq (line
331-364) are 95% identical — Groq's API is OpenAI-compatible by design.

ORIENT: This is the seeded loop, gated on green tests. All 32 contract
tests confirmed green pre-refactor.

DECIDE: Extract `_call_openai_compatible(url, key, prompt, temperature,
provider)` and have OpenAI/Groq paths become 4-line wrappers around it.
Anthropic stays separate (different body shape, different response
extraction). Also remove the noop `try: ... except RuntimeError: raise`
in all three paid paths (no semantic change).

DEVIL:
- Correctness: walked through both paths character-by-character. Body,
  headers, response shape, error format are byte-identical except for
  url, key, provider label. Helper takes those three plus prompt +
  temperature. Caller checks key presence and produces correct
  missing-key error message — verified pinned by tests
  test_openai_raises_when_key_missing / test_groq_raises_when_key_missing.
- Scope: Could also extract Anthropic's pattern, but the body and
  response shapes are sufficiently different that doing so would
  require a parameterized strategy (extractor closure) — bigger
  surface, less benefit. Skipped.
- Priority: Confirmed top of queue. The refactor's value is
  reduced cognitive load on every future provider addition. Each new
  OpenAI-compatible provider is now one method, not three.
- Subtle: The noop try/except removal was unrelated to the seeded
  refactor but cohesive — it was confirmed dead by reading. Kept in
  loop scope.

ACT:
- Added `_call_openai_compatible` (43 lines, fully documented).
- _generate_openai: 26 → 9 lines.
- _generate_groq:   26 → 9 lines.
- _generate_anthropic / _generate_ollama: removed noop try/except blocks.
- Suite: 298 unchanged (no new tests; existing 32 contract tests proved
  the refactor preserves behavior).

REVEALS:
- Anthropic's body building (filter+join system messages) is awkward.
  Could be extracted to a helper if a second Anthropic-shape provider
  is ever added. Premature today.
- The bare `_post_with_retry` calls in all three paths use timeout=60
  hard-coded. If any provider needs a different timeout, this will
  need parameterization. Not warranted today.
- Test count (138 Python) verified unchanged — refactor is purely
  internal restructuring.

## Loop 22 — Configurable Ollama timeout for CPU-bound local models

OBSERVE: Loop 19 reveal — `_post_with_retry(... 60 ...)` hard-coded a
60s timeout for Ollama. Local CPU inference on 7-8B models routinely
takes >60s for the first token on cold start. Real bug for the default
backend.

ORIENT: Reprioritized away from the seeded `can_execute()` deletion
after re-investigation showed it has internal callers (pipeline.rs:88,
128) AND is part of pub API of the lib crate — the loop-13 queue note
was wrong. Reading the code, `can_execute()` is also a clarity win
over `success` at call sites. Don't delete.

DECIDE: Add `_resolve_ollama_timeout()` reading `OLLAMA_TIMEOUT_S` env
var with default 180s. Fall back to default for missing/blank/
non-integer/non-positive values, with a logged warning for the latter
two. Wire into the Ollama call site only — paid providers stay 60s
(network APIs reliably first-token in well under that).

DEVIL:
- Correctness:
  * `int(raw)` raises ValueError on floats too. That's fine —
    seconds are integers per the env-var convention.
  * Negative / zero values fall back rather than passing through.
    Verified: passing 0 to requests.post would mean "use system default"
    which is implementation-dependent; falling back is safer.
  * Whitespace-only env value: `.strip()` then `not raw` catches it.
- Scope: Could parameterize all three providers with their own env
  vars. Premature — paid providers are reliably fast and 60s is fine.
  Adding env vars for them adds API surface to test without benefit.
- Priority: Real user-facing bug for the default backend. Higher than
  `can_execute()` cleanup (priority-9 cosmetic). Higher than the
  CycleDetected doc fix (priority-9 documentation).
- Subtle: The Ollama path is the only one where `_post_with_retry`
  is called inside a try/except RuntimeError that wraps the error —
  that wrap survives because timeout is just one exit path.

ACT:
- chatbot/generation/llm_client.py:
  * Added `_PAID_PROVIDER_TIMEOUT_S = 60`,
    `_OLLAMA_DEFAULT_TIMEOUT_S = 180`.
  * Added `_resolve_ollama_timeout()` (33 lines, fully documented).
  * All three call sites updated: paid use constant, Ollama uses
    resolver.
- chatbot/tests/test_llm_timeouts.py: 7 new tests (default unset /
  blank / positive int / non-integer / zero / negative / default >
  paid).
- .env.example: documented OLLAMA_TIMEOUT_S env var.
- Suite: 305 tests (was 298). Python 145. All 53 LLM tests pass in
  0.83s.

REVEALS:
- The Anthropic/OpenAI/Groq paths share the constant but Anthropic's
  `max_tokens=4096` is also hard-coded. If users hit max-token
  truncation, that's a separate bug surface. Queued.
- The `_resolve_ollama_timeout` warning logs include the rejected
  value, useful for debugging misconfiguration. Pattern worth
  reusing for any future env-var resolver.

## Loop 23 — Configurable Anthropic max_tokens + extracted env-var resolver

OBSERVE: Loop 22 reveal — Anthropic's `max_tokens` was hard-coded at
4096. Anthropic's Messages API silently truncates output exceeding
this budget. For long chat responses on graphs of 30+ nodes, that's a
real failure mode (mid-sentence cutoff, no error). Second hard-coded
knob in two loops: time to extract.

ORIENT: Two patterns now want the same env-var-with-fallback shape:
`OLLAMA_TIMEOUT_S` (loop 22) and `ANTHROPIC_MAX_TOKENS`. DRY win is
real. Without extraction, every future env-var knob duplicates the
same blank-handling, parse-handling, sign-checking, and warning-format
code. Three knobs would become unmaintainable.

DECIDE: Extract `_resolve_positive_int_env(var_name, default, *, unit)`
as the generic helper. Reimplement `_resolve_ollama_timeout` on top of
it. Add `_resolve_anthropic_max_tokens`. Wire the latter into
`_generate_anthropic`'s body construction.

DEVIL:
- Correctness: The `unit` parameter is purely cosmetic (warning
  suffix). It does not affect parsing or the returned int. Verified
  by `test_resolver_unit_appears_in_warning`. The Anthropic resolver
  passes no unit; the Ollama one passes "s". Both verified end-to-end.
- Scope: Could also extract OpenAI/Groq temperature handling, but
  those aren't env-driven currently. Not in scope. Could also have
  generalized to `_resolve_int_env(allow_zero=True)` for retry counts
  but no such knob exists today — premature.
- Priority: Real critical-path bug (silent truncation on paid API).
  Better than backup candidates (CycleDetected doc, comfyui_bridge
  smoke).
- Subtle: Anthropic API docs note `max_tokens` is REQUIRED on every
  request; cannot be omitted. Default of 4096 preserves existing
  behavior exactly — pinned by `test_anthropic_max_tokens_default`
  asserting `body["max_tokens"] == 4096`.
- Subtle: We do NOT enforce an upper bound. Anthropic's per-model
  ceiling varies (8192 for Haiku, 64K for Sonnet 3.5 with extended
  output, etc). Letting the API reject is clearer than a stale
  client-side limit.

ACT:
- chatbot/generation/llm_client.py:
  * Added `_ANTHROPIC_DEFAULT_MAX_TOKENS = 4096`.
  * Added `_resolve_positive_int_env(var_name, default, *, unit="")`
    — generic resolver with full docstring.
  * Reimplemented `_resolve_ollama_timeout` as a thin wrapper.
  * Added `_resolve_anthropic_max_tokens`.
  * `_generate_anthropic`: `"max_tokens": _resolve_anthropic_max_tokens()`.
- chatbot/tests/test_llm_timeouts.py: rewrote to cover the generic
  resolver (7 tests) + Ollama wiring (4) + Anthropic resolver (4).
  15 tests total (was 7).
- chatbot/tests/test_llm_providers.py: added 2 integration tests
  asserting the env var actually flows into the request body
  (default=4096, override=8192).
- .env.example: documents ANTHROPIC_MAX_TOKENS.

Suite: 315 tests (158 Rust + 2 UI + 155 Python). All green.

REVEALS:
- The resolver pattern is now ready for a third use. Likely next
  candidates: max retries (`LLM_MAX_RETRIES`), Anthropic version
  pin (`ANTHROPIC_VERSION`, currently hard-coded "2023-06-01").
- The Anthropic system-message join (loop 21 reveal) is still
  awkward but no second consumer materialized; staying queued.
- All three env-var helpers now share warning format. If a future
  loop adds structured logging, swap one `LOGGER.warning(...)` and
  all three downstream knobs benefit.

## Loop 24 — Configurable retry budget; generic min_value resolver

OBSERVE: Loop 23 reveal — `_MAX_RETRIES = 1` is hard-coded. Operators
behind flaky proxies, unreliable VPNs, or transient-prone upstreams
have no way to raise it. Conversely, integration test environments
that mock all network may want 0 to fail fast. Real config knob,
predicted by loop 23's "ready for a third use" reveal.

ORIENT: Retry counts have a different validity domain than
timeouts/token budgets — 0 is meaningful (disable retries) but
negative is not. The loop-23 resolver rejected 0. Three options:
  (a) duplicate the resolver with `<` instead of `<=` — DRY violation.
  (b) accept the duplication — fast but compounds for any future
      knob with bespoke bounds.
  (c) generalize to `_resolve_int_env(min_value=N)` and reimplement
      `_resolve_positive_int_env` as a wrapper.
Picked (c). Symmetrical with how Rust's `Range`/`RangeFrom` factor
out lower-bound choice.

DECIDE: Three changes:
1. Add `_resolve_int_env(var_name, default, *, min_value=1, unit="")`.
   Reimplement `_resolve_positive_int_env` as a thin wrapper.
2. Add `_resolve_max_retries()` calling it with `min_value=0`.
3. Convert `_post_with_retry` from `@staticmethod` to instance method
   reading `self.max_retries`. Set `self.max_retries` in `__init__`.

DEVIL:
- Correctness: Static→instance conversion breaks all existing
  retry-test call sites. They call `LLMClient._post_with_retry(...)`
  on the class. Must update every call site to `LLMClient()._post_with_retry(...)`.
  Verified by sed-rewriting the file (22 occurrences). All 18
  pre-existing retry tests pass post-rewrite without further edits.
- Correctness: Warning-message wording changed from "non-positive"
  to "below minimum N" for the generic helper. Old test asserting
  on "non-positive" must be updated. Done.
- Scope: Could also have made `_RETRY_DELAY_S` and `_RETRY_AFTER_MAX_S`
  configurable — both are reasonable knobs. Holding for now;
  current defaults are sound for paid-API ratios and bouncing
  too many env vars at users harms ergonomics.
- Priority: Real operator knob. Specifically, the loop-19 Retry-After
  honor goes to waste at max_retries=0 — but that's the operator's
  explicit choice and is correct.
- Subtle: `_resolve_positive_int_env` now passes through `_resolve_int_env`
  with `min_value=1`. The shared warning format means existing log
  consumers see the new "below minimum 1" wording for non-positive
  values. Acceptable — the message is more informative anyway and
  the only consumer is humans reading logs.

ACT:
- chatbot/generation/llm_client.py:
  * Added `_resolve_int_env(var_name, default, *, min_value=1, unit="")`.
  * Reimplemented `_resolve_positive_int_env` as a wrapper.
  * Added `_resolve_max_retries()`.
  * Renamed `_MAX_RETRIES` → `_DEFAULT_MAX_RETRIES`.
  * `LLMClient.__init__` now sets `self.max_retries = _resolve_max_retries()`.
  * `_post_with_retry` is now an instance method; reads `self.max_retries`.
- chatbot/tests/test_llm_retry.py:
  * Sed-rewritten 22 call sites: `LLMClient._post_with_retry(...)`
    → `LLMClient()._post_with_retry(...)`. Module docstring updated
    to explain the convention and pin default semantics.
  * Added 4 new tests for LLM_MAX_RETRIES env var: default=1,
    override=3 with 4-attempt verification, =0 disables retry on
    5xx, =0 disables retry on ConnectionError.
- chatbot/tests/test_llm_timeouts.py:
  * Updated `test_resolver_rejects_zero` assertion text:
    "non-positive" → "below minimum".
  * Added `test_resolver_min_value_zero_accepts_zero` and
    `test_resolver_min_value_zero_rejects_negative`.
- .env.example: documents LLM_MAX_RETRIES.

Suite: 321 tests (158 Rust + 2 UI + 161 Python). All green.

REVEALS:
- The instance-method conversion of `_post_with_retry` opens the
  door for per-client retry policy: a future test could mutate
  `client.max_retries = N` after construction without env-var
  manipulation. Cleaner pattern than the env-var-only approach.
- Three resolver functions now exist; if a fourth lands the
  tower-of-thin-wrappers gets noisy. Watch for that. Threshold
  is probably 4-5 distinct knobs before considering a small
  config object.
- The Anthropic API version pin "2023-06-01" is still hard-coded
  (loop 23 backup A). String resolver pattern would be useful but
  no critical bug yet.

## Loop 25 — Fix CLI panic on bare-scalar JSON in `parse_serialized_graph`
**Status**: COMMITTED

OBSERVE: Loop-24 next.md mandated re-checking priority-1-7 items. Surveyed
production unwraps: gpu.rs:225 (mpsc send to live local rx — safe by stack
order), topology.rs:40-58 (HashMap unwraps backed by `connect()` validation
+ `remove_node` cascade — invariant holds for normal flow). Then audited
`parse_serialized_graph` in src/main.rs:439. Found a **priority-1 panic**.

ORIENT: `parse_serialized_graph` is called by the CLI `load-graph`
subcommand on user-supplied files. The fallback path indexes
`serde_json::Value` via `IndexMut` to inject defaults for missing
`version`/`metadata`/`metadata.tags`. But `Value::index_mut` panics if
the parsed JSON is not an object (bare string, number, bool, null,
array). Verified empirically — `cannot access key "version" in JSON
string` panic at serde_json/value/index.rs:102. The function's signature
claims `Result<_, serde_json::Error>` but actually panics, so callers
have no way to recover.

DECIDE: Add `if !raw.is_object()` guard before any `IndexMut` mutation,
and re-call `from_value` to surface the original deserialization error.
Add 4 regression tests covering: full payload, missing-fields injection,
bare-scalar/array rejection (5 payloads, none panic), invalid JSON.

DEVIL:
1. Correctness — could the guard mis-classify a valid payload? No. A
   serialized graph is always a JSON object (`SerializedGraph` is a
   struct). Non-objects cannot deserialize regardless of which path.
2. Scope — is this fixing a symptom? No, the root cause is unconditional
   `IndexMut`. Fix is at the actual mutation point. Could also look at
   the inner `raw["metadata"]["tags"]` — but that's gated by
   `raw["metadata"].is_object()` already (line 448 ensures metadata is
   set to `{}` if missing/non-object before tags is accessed).
3. Priority — is there a higher-impact bug? Surveyed: no other
   production unwraps had unprovable invariants this loop. CLI panic on
   user file input is priority 1. Confirmed.

ACT:
- src/main.rs:439-465 — added `if !raw.is_object() { return ... }` guard
  with explanatory comment.
- src/main.rs:tests — 4 new tests:
  - parse_serialized_graph_accepts_full_payload
  - parse_serialized_graph_injects_missing_top_level_fields
  - parse_serialized_graph_rejects_bare_scalar_without_panic (5 payloads)
  - parse_serialized_graph_rejects_invalid_json
- README.md — test count 321→325, Rust 158→162.

VERIFY:
- cargo test --workspace: 140 lib + 4 bin + 6 ui_lib + 4 ui-tauri-lib +
  8 doc = 162 Rust tests, all green.

NEXT: see .agent/next.md.

## Loop 26 — Detect duplicate node IDs in serialized-graph validation
**Status**: COMMITTED

OBSERVE: Per loop-25 next.md, audited remaining `serde_json::Value`
IndexMut sites — filters/builtin/api.rs:622 and plugins/loader.rs:210
are both safe by construction (the target Value is built from
`json!({})` literally). Then audited `validate_serialized_graph` and
`execute_serialized_graph` in main.rs and found a silent correctness
bug.

ORIENT: `validate_serialized_graph` collects node ids into a HashSet
without checking the boolean returned by `insert`. A serialized graph
with duplicate node ids therefore passes validation. Then in
`execute_serialized_graph`, `node_map.insert(node.id, new_id)` for the
second occurrence overwrites the first, so any connection referencing
that id silently routes to whichever duplicate was inserted last; the
first is orphaned. This is a priority-3 silent semantic corruption on
hand-edited or maliciously crafted graph files.

DECIDE: Detect duplicate ids inline by checking the bool from
`HashSet::insert` and emitting "Duplicate node id: {id}". The CLI bails
on any validation error, so the rerouting can never reach
`execute_serialized_graph`.

DEVIL:
1. Correctness — does emitting the error early break valid duplicate
   *connection* edges? No. The check is on `node.id`, not on
   connection endpoints. Multiple connections with the same endpoints
   are still allowed (graph::ProcessingGraph::connect dedupes if it
   matters).
2. Scope — could the bug also live in the Tauri import path
   (ui/src-tauri/src/lib.rs `import_graph_json`)? That path returns the
   raw GraphState to the frontend, which is a different shape and not
   handed to the executor without a separate add_node round-trip in
   `apply_graph_state`. Out of scope for this loop. Captured for next.md.
3. Priority — fixing CLI is correct: it's the path that goes directly
   into `execute_serialized_graph`. The Tauri path requires its own
   audit later.

ACT:
- src/main.rs:466-487 — replace `node_ids.insert(node.id);` with
  `if !node_ids.insert(node.id) { errors.push("Duplicate node id: ...") }`.
- src/main.rs tests — add `validate_serialized_graph_flags_duplicate_node_ids`
  using two SerializedNode entries with the same `NodeId`.
- README.md — test count 325→326, Rust 162→163.

VERIFY:
- cargo test --bin ambara: 7 tests, all green (was 6, +1 new).
- cargo test --workspace: 163 Rust tests, all green.

NEXT: see .agent/next.md.

## Loop 27 — Detect duplicate node ids in Tauri validate_graph
**Status**: COMMITTED

OBSERVE: Loop-26 next.md mandated auditing the Tauri `apply_graph_state`
/ `execute_graph` paths for the same dup-id silent-rerouting bug.
Confirmed at ui/src-tauri/src/lib.rs:686-706: `node_id_map.insert(
ui_node.id.clone(), added_node_id)` overwrites for repeated `ui_node.id`,
then connections referencing the duplicated id silently route to the
last-inserted node. `validate_graph` at lib.rs:606 had no dup-id check.

ORIENT: Same bug class as loop 26, different surface (Tauri runtime vs
CLI). `execute_graph` calls `validate_graph` first and bails on errors,
so fixing validation closes the runtime path. Direct unit-test calls
into `execute_graph` already supply unique ids, so no regression risk.

DECIDE: Mirror loop 26's CLI fix in `validate_graph`, emitting a
`DuplicateNodeId` ValidationError with the offending id. Add a
regression test that clones a node into the sample graph and asserts
the error surfaces.

DEVIL:
1. Correctness — does the UI ever produce duplicate ids legitimately?
   No; React-Flow node ids are unique by construction client-side. Only
   malformed imports or JS bugs would produce duplicates, and those
   should be surfaced.
2. Scope — should `execute_graph` also defensively re-check?
   `execute_graph` already invokes `validate_graph` and bails on any
   error (lib.rs:657-668). Single source of truth is fine; layered
   defense not justified for this loop.
3. Priority — same priority-3 silent corruption as loop 26. Doing it
   immediately closes the parallel attack surface.

ACT:
- ui/src-tauri/src/lib.rs:606 — duplicate-id detection block in
  `validate_graph`, before the existing required-input check.
- ui/src-tauri/src/lib.rs tests — `validate_graph_flags_duplicate_node_ids`
  appended.
- README.md — test count 326→327, Rust 163→164.

VERIFY:
- cargo test --workspace: 164 Rust tests, all green (ui_lib went 4→5).

NEXT: see .agent/next.md.
