# Chatbot DACP Convergence Record

## DACP-C01: Intent Classification

- Proposed: hybrid keyword heuristics with optional LLM fallback.
- Attack: heuristics miss nuanced phrasing; full LLM adds latency and cost.
- Rebut: use heuristics first for deterministic routing; keep lightweight fallback path.
- Converged: ~~hybrid classifier (`chatbot/api/intent_classifier.py`)~~ **Superseded by DACP-C08**. Now handled by the LLM-powered agentic router (`chatbot/generation/agent.py`) with tool-calling ReAct loop.

## DACP-C02: ChromaDB vs FAISS

- Proposed: ChromaDB.
- Attack: FAISS can be faster for large corpora.
- Rebut: Ambara corpus is small and local; ChromaDB reduces operational burden.
- Converged: ChromaDB local persistent store.

## DACP-C03: Streaming WebSocket

- Proposed: include `/ws/{session_id}` streaming.
- Attack: added complexity for little gain.
- Rebut: progressive UX aligns with chat expectations and is backward compatible with REST.
- Converged: include WebSocket token stream endpoint.

## DACP-C04: Repair Strategy

- Proposed: LLM re-prompt with validation errors.
- Attack: rule-based patches are simpler and deterministic.
- Rebut: model-driven repair handles broader invalid-output classes; bounded to 3 retries.
- Converged: LLM repair loop with strict validators.

## DACP-C05: Sidecar vs Embedded API

- Proposed: Python sidecar process.
- Attack: extra process lifecycle complexity.
- Rebut: isolates Python/ML dependencies from Rust/Tauri runtime and keeps plugin engine independent.
- Converged: sidecar FastAPI service on port `8765`.

## DACP-C06: Session Storage

- Proposed: in-memory session store with expiry.
- Attack: no persistence across restarts.
- Rebut: initial scope favors low complexity and privacy; upgrade path to SQLite remains open.
- Converged: in-memory `SessionStore` with 30-minute inactivity TTL.

## DACP-C07: Embedding Source

- Proposed: local `sentence-transformers` model.
- Attack: startup load cost and model availability variability.
- Rebut: avoids API cost and supports offline use; deterministic hash fallback keeps tests reliable.
- Converged: local model with cached process-level instance and fallback embedding.

## DACP-C08: Agentic Router Replaces Intent Classifier

- Proposed: Replace keyword-based IntentClassifier with an LLM-powered ReAct agent that uses tool calls (search, generate, explain) to decide how to respond.
- Attack: LLM tool-calling is slower and more expensive than keyword matching; small models may produce malformed tool calls.
- Rebut: The agent falls back gracefully — unparseable responses are treated as direct answers, and mock mode preserves the deterministic keyword-based path. The agent handles nuanced queries (follow-ups, multi-step requests) that the keyword classifier misses entirely.
- Converged: `chatbot/generation/agent.py` with `MAX_TOOL_ROUNDS=4`, graph-aware exhaustion handling, and mock fallback. Old `intent_classifier.py` marked deprecated.

## DACP-C09: Lifespan Context Manager for Startup

- Proposed: Migrate from deprecated `@app.on_event("startup")` to FastAPI `lifespan` context manager. Add corpus init guard so startup failures produce a degraded health status instead of silent empty results.
- Attack: Lifespan refactor changes how the app initializes; if scoping is wrong, all routes break. The current deprecated pattern works.
- Rebut: Module-level globals (`code_retriever`, `chat_llm`, `sessions`) are not lifespan-scoped, so the migration is mechanical. The silent failure is a worse risk — operators see `"status":"ok"` with zero filters. The deprecation warning also clutters production logs.
- Converged: `_lifespan()` async context manager in `main.py`. Health endpoint reports `"starting"`, `"ok"`, or `"degraded"` with error detail. FastAPI version bumped to 0.9.0.

## DACP-C10: WebSocket Auto-Reconnect

- Proposed: Add exponential backoff reconnection to the WebSocket hook (`useChatApi.ts`) so dropped connections recover without page refresh.
- Attack: HTTP fallback already works; reconnection adds state complexity and potential duplicate message risk.
- Rebut: HTTP mode loses token streaming, making UX noticeably worse. The reconnection only re-establishes the socket for future messages — no messages are re-sent. Exponential backoff (1s → 30s, 10 attempts max) prevents server hammering.
- Converged: `useChatApi.ts` reconnect loop with `mountedRef` cleanup guard. UI shows distinct "Reconnecting…" (amber, pulsing) vs "Disconnected" (red, static) badges.

## DACP-C11: Typed Filter Response Models

- Proposed: Add Pydantic response models for `/filters` and `/filters/search` to serve typed OpenAPI docs.
- Attack: The endpoints already return the right shape; extra models are boilerplate.
- Rebut: Without `response_model`, OpenAPI docs show `list[object]` which is useless for frontend codegen and API consumers. The models also act as a contract test — if `to_dict()` output changes, the response validation catches it.
- Converged: `FilterItemModel`, `FilterPortModel`, `FilterParamModel` in `models.py`. Also fixed `/filters/search` calling private `_ensure_loaded()` — now uses public `_ensure_corpus()`.

## DACP-C12: Input Validation and CORS Hardening

- Proposed: Add `max_length` constraints to request fields, `top_k` bounds, and configurable CORS origins.
- Attack: This is a local dev tool; input validation adds friction and complexity.
- Rebut: Defense-in-depth is cheap here (Field constraints, one env var). Unbounded inputs could cause OOM on large payloads or excessive search results. CORS `*` default preserves dev convenience; `AMBARA_CORS_ORIGINS` env var enables lock-down for any non-local deployment.
- Converged: `ChatRequest.message` max 10k chars, `session_id` max 200, `context` max 50 items, `query` max 5k. Search `top_k` bounded 1–50, `q` max 500. CORS from `AMBARA_CORS_ORIGINS` env.

## DACP-C13: LLM Retry for Transient Failures

- Proposed: Add single retry with 2-second backoff for transient HTTP errors (429, 502, 503, 504, connection reset) in `LLMClient`.
- Attack: Retries add latency and can mask persistent failures.
- Rebut: Only retries once, only on transient status codes. On the failure path, 2 seconds is negligible vs losing an entire multi-stage generation. Persistent failures (400, 401, 403) are never retried.
- Converged: `_post_with_retry()` static method used by all four backends (Anthropic, OpenAI, Groq, Ollama).
