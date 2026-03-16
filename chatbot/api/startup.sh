#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

mkdir -p build logs

if [[ -f ".venv/bin/activate" ]]; then
  # shellcheck disable=SC1091
  source .venv/bin/activate || true
fi

if [[ ! -f build/filter_corpus.json ]]; then
  /usr/bin/python3 chatbot/corpus/extractor.py
  /usr/bin/python3 chatbot/corpus/schema_validator.py
fi

if [[ ! -d build/chroma_db ]]; then
  /usr/bin/python3 chatbot/corpus/embedder.py
fi

/usr/bin/python3 -m uvicorn chatbot.api.main:app --host 0.0.0.0 --port 8765 > logs/chatbot_api.log 2>&1 &
PID=$!
echo "$PID" > build/api.pid

echo "Chatbot API started on :8765 (pid=$PID)"
