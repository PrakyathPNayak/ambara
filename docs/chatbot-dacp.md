# Chatbot DACP Convergence Record

## DACP-C01: Intent Classification

- Proposed: hybrid keyword heuristics with optional LLM fallback.
- Attack: heuristics miss nuanced phrasing; full LLM adds latency and cost.
- Rebut: use heuristics first for deterministic routing; keep lightweight fallback path.
- Converged: hybrid classifier (`chatbot/api/intent_classifier.py`).

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
