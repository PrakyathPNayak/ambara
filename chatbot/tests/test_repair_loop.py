"""Tests for graph generation self-repair retry behavior."""

from __future__ import annotations

import json

from chatbot.generation.graph_generator import GraphGenerator


class _FlakyLLM:
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


def test_invalid_json_then_valid() -> None:
    generator = GraphGenerator("build/chroma_db", "build/filter_corpus.json", "chatbot/corpus/examples.json", force_mock_llm=True)
    generator.llm_client = _FlakyLLM(["not-json", _valid_graph()])
    result = generator.generate("blur and save")
    assert result.valid
    assert result.retries >= 1


def test_invalid_filter_then_valid() -> None:
    bad = json.dumps({"nodes": [{"id": "n1", "filter_id": "NONEXISTENT", "position": {"x": 0, "y": 0}, "parameters": {}}], "connections": [], "metadata": {}})
    generator = GraphGenerator("build/chroma_db", "build/filter_corpus.json", "chatbot/corpus/examples.json", force_mock_llm=True)
    generator.llm_client = _FlakyLLM([bad, _valid_graph()])
    result = generator.generate("blur and save")
    assert result.valid
    assert result.retries >= 1


def test_fail_after_max_retries() -> None:
    generator = GraphGenerator("build/chroma_db", "build/filter_corpus.json", "chatbot/corpus/examples.json", force_mock_llm=True)
    generator.llm_client = _FlakyLLM(["bad-json"] * 5)
    result = generator.generate("blur and save")
    assert not result.valid
    assert result.retries == 3
