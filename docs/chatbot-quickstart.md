# Chatbot Quickstart

## 1. Install Python Dependencies

```bash
/usr/bin/python3 -m pip install --break-system-packages \
  pillow pytesseract requests fastapi uvicorn chromadb sentence-transformers \
  openai anthropic tiktoken pydantic python-dotenv rich typer \
  pytest pytest-asyncio httpx wiremock jsonschema
```

## 2. Start API

The chatbot API builds the filter corpus automatically from Rust source code on startup (no manual extraction or embedding step needed).

```bash
cd /home/prakyathpnayak/Documents/programming/rust/ambara
bash chatbot/api/startup.sh
curl -s http://localhost:8765/health
```

Expected:

```json
{"status":"ok", "filters_loaded": 60, ...}
```

If status is `"degraded"`, the `error` field in the response will explain why.

## 3. Run a First Query

```bash
curl -s -X POST http://localhost:8765/graph/generate \
  -H 'Content-Type: application/json' \
  -d '{"query":"load image, blur, and save","partial_graph":null}'
```

## 4. Open UI

```bash
cd ui
npm run dev
```

Open the app and use the Chat panel in the left sidebar.

## 5. Insert Generated Graph

From assistant messages that include graph output:

1. Click `Load into Canvas` in graph preview, or
2. Click `Insert Graph`.

## Troubleshooting

1. If chatbot API fails to start, inspect `logs/chatbot_api.log`.
2. If the health endpoint reports `"degraded"`, check the `error` field and `logs/chatbot_api.log`.
3. If UI tests fail with `document is not defined`, ensure `jsdom` is installed and Vitest runs through Vite config.
4. If `tauri dev` fails due watch limits on Linux, use existing documented watcher-limit workaround in README.
