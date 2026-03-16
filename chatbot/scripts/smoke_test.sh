#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

mkdir -p logs screenshots

bash chatbot/api/startup.sh
sleep 3

curl -sf http://localhost:8765/health > build/smoke_health.json
/usr/bin/python3 scripts/screenshotter.py capture --tag G-04 --url http://localhost:8765/health || true

# Run representative generation requests
curl -sf -X POST http://localhost:8765/graph/generate -H 'Content-Type: application/json' -d '{"query":"apply blur","partial_graph":null}' > build/smoke_gen_1.json
curl -sf -X POST http://localhost:8765/graph/generate -H 'Content-Type: application/json' -d '{"query":"resize and save","partial_graph":null}' > build/smoke_gen_2.json
curl -sf -X POST http://localhost:8765/graph/generate -H 'Content-Type: application/json' -d '{"query":"blend two images","partial_graph":null}' > build/smoke_gen_3.json

echo "SMOKE TEST OK"
