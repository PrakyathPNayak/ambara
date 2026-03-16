"""Tests targeting hallucination prevention and graceful failure behavior."""

from __future__ import annotations

from chatbot.generation.graph_generator import GraphGenerator
from chatbot.generation.graph_validator import GraphValidator


def test_fake_filter_names_fail_validation() -> None:
    validator = GraphValidator("chatbot/corpus/graph_schema.json", "build/filter_id_set.json", "build/filter_corpus.json")
    fake = '{"nodes":[{"id":"n1","filter_id":"fake_filter","position":{"x":0,"y":0},"parameters":{}}],"connections":[],"metadata":{}}'
    result = validator.validate_filter_ids(fake)
    assert not result.valid


def test_large_request_returns_bounded_graph() -> None:
    generator = GraphGenerator("build/chroma_db", "build/filter_corpus.json", "chatbot/corpus/examples.json", force_mock_llm=True)
    result = generator.generate("apply 30 operations and save")
    assert result.valid
    assert len(result.graph.get("nodes", [])) < 20


def test_contradictory_constraints_best_effort() -> None:
    generator = GraphGenerator("build/chroma_db", "build/filter_corpus.json", "chatbot/corpus/examples.json", force_mock_llm=True)
    result = generator.generate("make image both grayscale and full color at once")
    assert result.graph is not None


def test_spanish_query() -> None:
    generator = GraphGenerator("build/chroma_db", "build/filter_corpus.json", "chatbot/corpus/examples.json", force_mock_llm=True)
    result = generator.generate("carga una imagen aplica desenfoque y guarda")
    assert result.valid
