"""Tests for graph generation retry/fallback behavior.

The new multi-stage pipeline handles failures differently:
- Planning failure → deterministic fallback (no retries)
- Validation failure → Stage 4 repair loop (up to 2 attempts)
- Mock mode → deterministic graph directly
"""

from __future__ import annotations

import json

from chatbot.generation.graph_generator import GraphGenerator


class _FlakyLLM:
    backend = "custom"

    def __init__(self, responses: list[str]) -> None:
        self.responses = responses
        self.calls = 0

    def generate(self, _prompt: dict, temperature: float = 0.0) -> str:
        _ = temperature
        idx = min(self.calls, len(self.responses) - 1)
        self.calls += 1
        return self.responses[idx]


def _valid_graph() -> str:
    return json.dumps(
        {
            "version": "1.0.0",
            "metadata": {},
            "nodes": [
                {"id": "n1", "filter_id": "load_image", "position": {"x": 0, "y": 0}, "parameters": {}},
                {"id": "n2", "filter_id": "save_image", "position": {"x": 1, "y": 1}, "parameters": {}},
            ],
            "connections": [
                {"from_node": "n1", "from_port": "image", "to_node": "n2", "to_port": "image"}
            ],
        }
    )


def test_invalid_json_triggers_fallback() -> None:
    """When the LLM returns garbage, plan parsing fails and deterministic fallback kicks in."""
    generator = GraphGenerator("build/chroma_db", "build/filter_corpus.json", "chatbot/corpus/examples.json", force_mock_llm=True)
    generator.llm_client = _FlakyLLM(["not-json", _valid_graph()])
    result = generator.generate("blur and save")
    assert result.valid
    assert result.graph is not None
    assert "Fallback" in result.explanation


def test_invalid_filter_triggers_fallback() -> None:
    """When the LLM returns a graph (not a plan), plan parsing fails, fallback produces valid output."""
    bad = json.dumps({"nodes": [{"id": "n1", "filter_id": "NONEXISTENT", "position": {"x": 0, "y": 0}, "parameters": {}}], "connections": [], "metadata": {}})
    generator = GraphGenerator("build/chroma_db", "build/filter_corpus.json", "chatbot/corpus/examples.json", force_mock_llm=True)
    generator.llm_client = _FlakyLLM([bad, _valid_graph()])
    result = generator.generate("blur and save")
    assert result.valid
    assert result.graph is not None


def test_persistent_bad_json_still_produces_fallback() -> None:
    """Even with consistently bad LLM output, deterministic fallback ensures a valid graph."""
    generator = GraphGenerator("build/chroma_db", "build/filter_corpus.json", "chatbot/corpus/examples.json", force_mock_llm=True)
    generator.llm_client = _FlakyLLM(["bad-json"] * 5)
    result = generator.generate("blur and save")
    # Deterministic fallback should produce a valid graph.
    assert result.valid
    assert result.graph is not None
