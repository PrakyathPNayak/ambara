# Ambara Chatbot System

## Overview

Ambara now includes a Python chatbot sidecar that can translate natural language into `SerializedGraph` JSON pipelines.

The system consists of:

1. React chat UI in `ui/src/components/chat/`.
2. FastAPI backend in `chatbot/api/main.py`.
3. Retrieval-augmented generation pipeline in `chatbot/retrieval/` and `chatbot/generation/`.
4. Corpus and embedding tooling in `chatbot/corpus/`.
5. Rust CLI integration points in `src/main.rs`.

## Architecture

```text
+----------------------+      +--------------------------------+
| React Chat Panel     | ---> | FastAPI /chat + /graph/*       |
| ui/src/components/*  | <--- | chatbot/api/main.py            |
+----------------------+      +--------------------------------+
                                      |
                                      v
                        +-------------------------------+
                        | GraphGenerator (multi-stage)  |
                        |                               |
                        |  Stage 1: PLAN                |
                        |    planner.py → decompose     |
                        |    query into ordered steps   |
                        |                               |
                        |  Stage 2: SELECT              |
                        |    selector.py → pick best    |
                        |    filter + params per step   |
                        |    (retrieval + LLM)          |
                        |                               |
                        |  Stage 3: CONNECT             |
                        |    connector.py → wire graph  |
                        |    deterministically (no LLM) |
                        |                               |
                        |  Stage 4: VALIDATE + REPAIR   |
                        |    GraphValidator + repair     |
                        |    prompt if needed            |
                        |                               |
                        |  Fallback: keyword-based      |
                        |    deterministic generation    |
                        +-------------------------------+
                                      |
                                      v
                        +-------------------------------+
                        | Ambara Rust CLI               |
                        | cargo run -- load-graph ...   |
                        +-------------------------------+
```

## API Reference

### `POST /chat`

Request:

```json
{
  "message": "apply blur and save",
  "session_id": "sess-123",
  "context": []
}
```

Response:

```json
{
  "reply": "Generated 3 nodes and 2 connections using relevant filters.",
  "session_id": "sess-123",
  "graph_generated": true,
  "graph": {"nodes": [], "connections": [], "metadata": {}}
}
```

### `POST /graph/generate`

Generates a graph from natural language.

### `POST /graph/validate`

Validates graph schema, filter IDs, and connection ports.

### `POST /graph/execute`

Executes graph via Rust CLI command:

`cargo run -- load-graph <tmp_file> --execute`

### `GET /filters`

Returns extracted filter corpus.

### `GET /filters/search`

Semantic retrieval endpoint.

### `GET /health`

Readiness endpoint returning status, corpus count, and backend mode.

### `GET /ws/{session_id}`

WebSocket endpoint for token-streaming style responses.

## Retrieval and Generation Flow

The pipeline uses a multi-stage agentic approach (inspired by HuggingGPT and ReAct patterns):

1. **Plan** – LLM decomposes the user query into ordered processing steps using a compact filter catalog.
2. **Select** – For each step, the retriever finds top-5 candidate filters; LLM picks the best one and sets parameters. If the filter ID is already known from the plan, parameters are inferred via regex pattern matching (no LLM call needed).
3. **Connect** – Deterministic code wires the selected filters into a valid graph using port-type compatibility rules. No LLM involved — this eliminates the biggest source of hallucinated port names.
4. **Validate + Repair** – Graph is validated (schema, filter IDs, connections). If invalid, up to 2 LLM-assisted repair attempts are made.
5. **Fallback** – If any stage fails, keyword-based deterministic generation produces a reasonable graph.

## Safety and Hallucination Guardrails

1. Graph schema validation via JSON Schema draft-07.
2. Filter ID whitelist check against `build/filter_id_set.json`.
3. Connection port compatibility checks.
4. Deterministic connection wiring (Stage 3) prevents hallucinated port names.
5. `<think>` tag stripping for qwen3 model compatibility.
6. Mock backend deterministic fallback for offline operation.
7. Parameter inference via regex rather than LLM for known filters.

## Operational Notes

1. Start backend: `bash chatbot/api/startup.sh`.
2. UI talks directly to `http://localhost:8765`.
3. Tauri backend remains authoritative for canvas execution and plugin runtime.
4. Current API startup uses FastAPI `on_event` and emits a deprecation warning; migration to lifespan is planned.
