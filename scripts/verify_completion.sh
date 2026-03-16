#!/usr/bin/env bash
set -euo pipefail

PYTHON_BIN="/usr/bin/python3"

PASS=0
FAIL=0

check() {
  local name="$1"
  shift
  if "$@" &>/dev/null; then
    echo "  [OK] $name"
    PASS=$((PASS+1))
  else
    echo "  [FAIL] $name (command: $*)"
    FAIL=$((FAIL+1))
  fi
}

echo ""
echo "============================================="
echo " AMBARA CHATBOT - COMPLETION VERIFICATION "
echo "============================================="
echo ""

echo "-- Rust Core --"
check "cargo build --release" cargo build --release
check "cargo clippy completes" cargo clippy --all-features -- -A warnings
check "cargo test all features" cargo test --all-features
check "list --json works" bash -c "cargo run --quiet -- list --json 2>/dev/null | ${PYTHON_BIN} -c 'import sys, json; d=json.load(sys.stdin); assert len(d)>5'"
check "load-graph --dry-run works" bash -c "echo '{\"nodes\":[],\"connections\":[],\"metadata\":{}}' > /tmp/v.json && cargo run --quiet -- load-graph /tmp/v.json --dry-run >/dev/null"

echo ""
echo "-- Python Backend --"
check "filter corpus exists" test -f build/filter_corpus.json
check "chroma db populated" ${PYTHON_BIN} -c "import chromadb; c=chromadb.PersistentClient('build/chroma_db'); col=c.get_collection('ambara_filters'); assert col.count()>5"
check "20 examples present" ${PYTHON_BIN} -c "import json; ex=json.load(open('chatbot/corpus/examples.json')); assert len(ex)>=20"
check "graph schema valid" ${PYTHON_BIN} -c "import json, jsonschema; s=json.load(open('chatbot/corpus/graph_schema.json')); jsonschema.Draft7Validator.check_schema(s)"
check "api health" curl -sf http://localhost:8765/health
check "filters search" curl -sf 'http://localhost:8765/filters/search?q=blur&top_k=3'
check "graph generate" curl -sf -X POST http://localhost:8765/graph/generate -H 'Content-Type: application/json' -d '{"query":"apply blur"}'
check "pytest corpus" pytest chatbot/tests/test_corpus.py -q
check "pytest retrieval" pytest chatbot/tests/test_retrieval.py -q
check "pytest generation" pytest chatbot/tests/test_generation.py -q
check "pytest api" pytest chatbot/tests/test_api.py -q

echo ""
echo "-- React UI --"
check "npm build" bash -c "cd ui && npm run build >/dev/null"
check "tsc" bash -c "cd ui && npx tsc --noEmit >/dev/null"
check "vitest" bash -c "cd ui && npx vitest run >/dev/null"
check "ChatPanel exists" test -f ui/src/components/chat/ChatPanel.tsx
check "useChatApi exists" test -f ui/src/hooks/useChatApi.ts

echo ""
echo "-- Screenshots --"
check "API screenshot verdict" ${PYTHON_BIN} scripts/screenshotter.py analyze --tag D-01 --expect "ok,filters"
check "UI screenshot verdict" ${PYTHON_BIN} scripts/screenshotter.py analyze --tag F-05 --expect "Chat"
check "E2E screenshot verdict" ${PYTHON_BIN} scripts/screenshotter.py analyze --tag G-01 --expect "PASSED"

echo ""
echo "-- Docs --"
check "chatbot-system doc" test -f docs/chatbot-system.md
check "chatbot-quickstart doc" test -f docs/chatbot-quickstart.md
check "changelog updated" grep -qi "chatbot\|v0.4.0" CHANGELOG.md

echo ""
echo "============================================="
echo "RESULTS: ${PASS} passed, ${FAIL} failed"
echo "============================================="

if [[ "$FAIL" -eq 0 ]]; then
  echo "ALL CHECKS PASSED"
  ${PYTHON_BIN} scripts/auto_loop.py mark SYSTEM PASS "All completion criteria verified"
  exit 0
fi

echo "CHECKS FAILED"
exit 1
