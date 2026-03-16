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
                        | GraphGenerator                |
                        | - FilterRetriever             |
                        | - ExampleRetriever            |
                        | - GraphPromptBuilder          |
                        | - LLMClient (mock/fallback)   |
                        | - GraphValidator              |
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

1. Corpus is extracted from `cargo run -- list --json`.
2. Corpus is validated and indexed in ChromaDB.
3. Query retrieves top filters and examples.
4. Prompt is built with schema and allowed filter IDs.
5. LLM response is validated.
6. Repair loop retries up to 3 times if invalid.

## Safety and Hallucination Guardrails

1. Graph schema validation via JSON Schema draft-07.
2. Filter ID whitelist check against `build/filter_id_set.json`.
3. Connection port compatibility checks.
4. Mock backend deterministic fallback for offline operation.

## Operational Notes

1. Start backend: `bash chatbot/api/startup.sh`.
2. UI talks directly to `http://localhost:8765`.
3. Tauri backend remains authoritative for canvas execution and plugin runtime.
4. Current API startup uses FastAPI `on_event` and emits a deprecation warning; migration to lifespan is planned.
