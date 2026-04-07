#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

mkdir -p build logs

if [[ -f ".venv/bin/activate" ]]; then
  # shellcheck disable=SC1091
  source .venv/bin/activate || true
fi

# Corpus is now built by CodeRetriever at app startup (lifespan hook).
# No separate extraction or embedding step is needed.

/usr/bin/python3 -m uvicorn chatbot.api.main:app --host 0.0.0.0 --port 8765 > logs/chatbot_api.log 2>&1 &
PID=$!
echo "$PID" > build/api.pid

echo "Chatbot API started on :8765 (pid=$PID)"
