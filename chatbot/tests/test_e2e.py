"""End-to-end tests for chatbot API and graph generation pipeline."""

from __future__ import annotations

import json
import os
import subprocess
import time
from pathlib import Path

import httpx

from chatbot.generation.graph_validator import GraphValidator

ROOT = Path(__file__).resolve().parents[2]


def _start_server() -> subprocess.Popen:
    env = os.environ.copy()
    # Force the deterministic mock LLM backend so this e2e test does not
    # depend on a live Ollama/OpenAI/Anthropic/Groq endpoint.
    env["AMBARA_FORCE_MOCK_LLM"] = "1"
    return subprocess.Popen(
        ["/usr/bin/python3", "-m", "uvicorn", "chatbot.api.main:app", "--host", "127.0.0.1", "--port", "8765"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
    )


def _wait_ready() -> None:
    deadline = time.time() + 25
    while time.time() < deadline:
        try:
            resp = httpx.get("http://127.0.0.1:8765/health", timeout=2)
            if resp.status_code == 200:
                return
        except Exception:
            time.sleep(0.4)
    raise RuntimeError("API did not become ready")


def test_e2e_queries() -> None:
    proc = _start_server()
    try:
        _wait_ready()
        validator = GraphValidator("chatbot/corpus/graph_schema.json", "build/filter_id_set.json", "build/filter_corpus.json")
        queries = [
            "load an image and apply gaussian blur",
            "resize image to 512x512 and save as PNG",
            "convert to grayscale then invert colors",
            "stack astrophotos and stretch histogram",
            "blend two images with 50% opacity",
            "apply brightness +20 and contrast +15 and saturation -10",
            "crop center 256x256 then rotate 90 degrees",
            "load folder of images and save all with blur applied",
            "add a text watermark node",
            "apply unsharp mask",
        ]

        for query in queries:
            resp = httpx.post("http://127.0.0.1:8765/graph/generate", json={"query": query, "partial_graph": None}, timeout=20)
            assert resp.status_code == 200
            data = resp.json()
            assert "valid" in data
            if data.get("graph"):
                graph_json = json.dumps(data["graph"])
                result = validator.validate_schema(graph_json)
                assert result.valid
                assert data.get("explanation")
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=5)
