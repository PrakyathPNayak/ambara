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


def test_sanitizer_fixes_invalid_save_image_to_port() -> None:
    class _SingleLLM:
        backend = "custom"

        def __init__(self, payload: str) -> None:
            self.payload = payload

        def generate(self, _prompt: dict, temperature: float = 0.0) -> str:
            _ = temperature
            return self.payload

    bad_graph = json.dumps(
        {
            "version": "1.0.0",
            "metadata": {},
            "nodes": [
                {"id": "n1", "filter_id": "load_image", "position": {"x": 0, "y": 0}, "parameters": {}},
                {"id": "n2", "filter_id": "batch_brightness", "position": {"x": 1, "y": 0}, "parameters": {}},
                {"id": "n3", "filter_id": "save_image", "position": {"x": 2, "y": 0}, "parameters": {}},
            ],
            "connections": [
                {"from_node": "n1", "from_port": "image", "to_node": "n2", "to_port": "images"},
                {"from_node": "n2", "from_port": "images", "to_node": "n3", "to_port": "images"},
            ],
        }
    )

    generator = GraphGenerator(
        chroma_path="build/chroma_db",
        corpus_path="build/filter_corpus.json",
        examples_path="chatbot/corpus/examples.json",
        force_mock_llm=False,
    )
    generator.llm_client = _SingleLLM(bad_graph)
    result = generator.generate("increase brightness then save")
    assert result.valid
    assert result.graph is not None

    save_edges = [
        c
        for c in result.graph.get("connections", [])
        if c.get("to_node") == "n3"
    ]
    assert save_edges
    assert all(c.get("to_port") == "image" for c in save_edges)


def test_mock_fallback_can_emit_branch_graph_for_blend_queries() -> None:
    generator = GraphGenerator(
        chroma_path="build/chroma_db",
        corpus_path="build/filter_corpus.json",
        examples_path="chatbot/corpus/examples.json",
        force_mock_llm=True,
    )
    result = generator.generate("blend two images and save")
    assert result.valid
    assert result.graph is not None

    nodes = result.graph.get("nodes", [])
    conns = result.graph.get("connections", [])
    merge_nodes = [
        n for n in nodes
        if str(n.get("filter_id")) in ("blend", "overlay")
    ]
    assert merge_nodes
    merge_id = str(merge_nodes[0].get("id"))
    incoming = [c for c in conns if str(c.get("to_node")) == merge_id]
    assert len(incoming) >= 2


def test_validator_rejects_duplicate_input_port_fan_in() -> None:
    validator = GraphValidator(
        "chatbot/corpus/graph_schema.json",
        "build/filter_id_set.json",
        "build/filter_corpus.json",
    )
    graph = {
        "version": "1.0.0",
        "metadata": {},
        "nodes": [
            {"id": "n1", "filter_id": "load_image", "position": {"x": 0, "y": 0}, "parameters": {}},
            {"id": "n2", "filter_id": "load_image", "position": {"x": 0, "y": 1}, "parameters": {}},
            {"id": "n3", "filter_id": "grayscale", "position": {"x": 1, "y": 0}, "parameters": {}},
        ],
        "connections": [
            {"from_node": "n1", "from_port": "image", "to_node": "n3", "to_port": "image"},
            {"from_node": "n2", "from_port": "image", "to_node": "n3", "to_port": "image"},
        ],
    }
    result = validator.validate_all(json.dumps(graph))
    assert not result.valid
    assert any("Duplicate connection into input port" in e for e in result.errors)


def test_batch_crop_resize_intent_enforces_batch_pipeline() -> None:
    class _SingleLLM:
        backend = "custom"

        def __init__(self, payload: str) -> None:
            self.payload = payload

        def generate(self, _prompt: dict, temperature: float = 0.0) -> str:
            _ = temperature
            return self.payload

    wrong_graph = json.dumps(
        {
            "version": "1.0.0",
            "metadata": {},
            "nodes": [
                {"id": "n1", "filter_id": "load_image", "position": {"x": 0, "y": 0}, "parameters": {}},
                {"id": "n2", "filter_id": "crop", "position": {"x": 1, "y": 0}, "parameters": {}},
                {"id": "n3", "filter_id": "resize", "position": {"x": 2, "y": 0}, "parameters": {}},
                {"id": "n4", "filter_id": "save_image", "position": {"x": 3, "y": 0}, "parameters": {}},
            ],
            "connections": [
                {"from_node": "n1", "from_port": "image", "to_node": "n2", "to_port": "image"},
                {"from_node": "n2", "from_port": "image", "to_node": "n3", "to_port": "image"},
                {"from_node": "n3", "from_port": "image", "to_node": "n4", "to_port": "image"},
            ],
        }
    )

    generator = GraphGenerator(
        chroma_path="build/chroma_db",
        corpus_path="build/filter_corpus.json",
        examples_path="chatbot/corpus/examples.json",
        force_mock_llm=False,
    )
    generator.llm_client = _SingleLLM(wrong_graph)
    result = generator.generate("Build me a pipeline for cropping and resizing images in batch")

    assert result.valid
    assert result.graph is not None

    node_filter_ids = [str(node.get("filter_id")) for node in result.graph.get("nodes", [])]
    assert node_filter_ids == ["load_folder", "batch_crop", "batch_resize", "batch_save_images"]

    for conn in result.graph.get("connections", []):
        assert conn.get("from_port") == "images"
        assert conn.get("to_port") == "images"


# ---------------------------------------------------------------------------
# Validator: topology (cycle detection, orphan detection)
# ---------------------------------------------------------------------------

def test_validator_detects_cycle() -> None:
    """Graph with a cycle should fail topology validation."""
    validator = GraphValidator(
        "chatbot/corpus/graph_schema.json",
        "build/filter_id_set.json",
        "build/filter_corpus.json",
    )
    graph = {
        "version": "1.0.0",
        "metadata": {},
        "nodes": [
            {"id": "a", "filter_id": "brightness", "position": {"x": 0, "y": 0}, "parameters": {}},
            {"id": "b", "filter_id": "contrast", "position": {"x": 1, "y": 0}, "parameters": {}},
            {"id": "c", "filter_id": "grayscale", "position": {"x": 2, "y": 0}, "parameters": {}},
        ],
        "connections": [
            {"from_node": "a", "from_port": "image", "to_node": "b", "to_port": "image"},
            {"from_node": "b", "from_port": "image", "to_node": "c", "to_port": "image"},
            {"from_node": "c", "from_port": "image", "to_node": "a", "to_port": "image"},
        ],
    }
    result = validator.validate_topology(json.dumps(graph))
    assert not result.valid
    assert any("cycle" in e.lower() for e in result.errors)


def test_validator_detects_orphan_node() -> None:
    """Node with no connections in a multi-node graph should be flagged."""
    validator = GraphValidator(
        "chatbot/corpus/graph_schema.json",
        "build/filter_id_set.json",
        "build/filter_corpus.json",
    )
    graph = {
        "version": "1.0.0",
        "metadata": {},
        "nodes": [
            {"id": "n1", "filter_id": "load_image", "position": {"x": 0, "y": 0}, "parameters": {}},
            {"id": "n2", "filter_id": "brightness", "position": {"x": 1, "y": 0}, "parameters": {}},
            {"id": "orphan", "filter_id": "grayscale", "position": {"x": 2, "y": 0}, "parameters": {}},
        ],
        "connections": [
            {"from_node": "n1", "from_port": "image", "to_node": "n2", "to_port": "image"},
        ],
    }
    result = validator.validate_topology(json.dumps(graph))
    assert not result.valid
    assert any("orphan" in e.lower() for e in result.errors)


def test_validator_valid_topology() -> None:
    """A simple linear graph should pass topology validation."""
    validator = GraphValidator(
        "chatbot/corpus/graph_schema.json",
        "build/filter_id_set.json",
        "build/filter_corpus.json",
    )
    graph = {
        "version": "1.0.0",
        "metadata": {},
        "nodes": [
            {"id": "n1", "filter_id": "load_image", "position": {"x": 0, "y": 0}, "parameters": {}},
            {"id": "n2", "filter_id": "brightness", "position": {"x": 1, "y": 0}, "parameters": {}},
        ],
        "connections": [
            {"from_node": "n1", "from_port": "image", "to_node": "n2", "to_port": "image"},
        ],
    }
    result = validator.validate_topology(json.dumps(graph))
    assert result.valid


# ---------------------------------------------------------------------------
# Repair prompt builder: metadata enrichment
# ---------------------------------------------------------------------------

def test_repair_prompt_includes_filter_metadata() -> None:
    """Repair prompt should include filter metadata cards when corpus_by_id is given."""
    builder = RepairPromptBuilder()
    corpus = {
        "gaussian_blur": {
            "id": "gaussian_blur",
            "name": "Gaussian Blur",
            "inputs": [{"name": "image", "type": "Image"}],
            "outputs": [{"name": "image", "type": "Image"}],
        },
    }
    prompt = builder.build(
        query="blur image",
        failed_graph='{"nodes":[{"id":"n1","filter_id":"gaussian_blur","position":{"x":0,"y":0},"parameters":{}}],"connections":[],"metadata":{}}',
        errors=["Invalid from_port out for node n1"],
        corpus_by_id=corpus,
    )
    content = str(prompt)
    assert "VALID FILTER METADATA" in content
    assert "gaussian_blur" in content
    assert "Image" in content


def test_repair_prompt_without_corpus_still_works() -> None:
    """Backward compatibility: repair prompt works without corpus_by_id."""
    builder = RepairPromptBuilder()
    prompt = builder.build(
        query="blur",
        failed_graph='{"nodes":[],"connections":[],"metadata":{}}',
        errors=["some error"],
    )
    assert "some error" in str(prompt)
    assert "VALID FILTER METADATA" not in str(prompt)
