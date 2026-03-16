"""Tests for prompt building, validation, and graph generation pipeline."""

from __future__ import annotations

import json

from chatbot.generation.graph_generator import GraphGenerator
from chatbot.generation.graph_validator import GraphValidator
from chatbot.generation.llm_client import LLMClient
from chatbot.generation.prompt_builder import GraphPromptBuilder
from chatbot.generation.repair_prompt_builder import RepairPromptBuilder


def test_prompt_builder() -> None:
    builder = GraphPromptBuilder("build/filter_corpus.json")
    prompt = builder.build(
        "make image blurry",
        [{"id": "gaussian_blur", "description": "Blurs an image"}],
        [],
    )
    assert len(prompt["messages"]) >= 2
    assert "gaussian_blur" in str(prompt)


def test_llm_mock_json() -> None:
    client = LLMClient(force_mock=True)
    payload = client.generate({"messages": [{"role": "user", "content": "test"}]})
    obj = json.loads(payload)
    assert "nodes" in obj and "connections" in obj


def test_graph_validator_good_bad() -> None:
    validator = GraphValidator(
        "chatbot/corpus/graph_schema.json",
        "build/filter_id_set.json",
        "build/filter_corpus.json",
    )
    good = '{"nodes":[],"connections":[],"metadata":{}}'
    assert validator.validate_schema(good).valid
    bad = '{"invalid_field": true}'
    assert not validator.validate_schema(bad).valid


def test_repair_prompt_builder() -> None:
    builder = RepairPromptBuilder()
    prompt = builder.build(
        query="blur image",
        failed_graph='{"nodes":[{"filter_id":"NONEXISTENT"}],"connections":[],"metadata":{}}',
        errors=["filter_id NONEXISTENT not found in registry"],
    )
    assert "NONEXISTENT" in str(prompt)


def test_end_to_end_generate_mock() -> None:
    generator = GraphGenerator(
        chroma_path="build/chroma_db",
        corpus_path="build/filter_corpus.json",
        examples_path="chatbot/corpus/examples.json",
        force_mock_llm=True,
    )
    result = generator.generate("apply gaussian blur and save")
    assert result.valid
    assert result.graph is not None
