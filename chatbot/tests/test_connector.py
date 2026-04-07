"""Tests for Stage 3 – Connect: deterministic graph wiring."""

from __future__ import annotations

from chatbot.generation.connector import (
    _find_best_output,
    _types_compatible,
    build_graph,
)


# ---------------------------------------------------------------------------
# Port helper tests
# ---------------------------------------------------------------------------


def test_types_compatible_exact_match() -> None:
    assert _types_compatible("Image", "Image") is True
    assert _types_compatible("Float", "Float") is True


def test_types_compatible_any() -> None:
    assert _types_compatible("Any", "Image") is True
    assert _types_compatible("Image", "Any") is True


def test_types_compatible_image_coercion() -> None:
    assert _types_compatible("Image", "Array<Image>") is True
    assert _types_compatible("Array<Image>", "Image") is True


def test_types_compatible_mismatch() -> None:
    assert _types_compatible("Image", "Float") is False


def test_find_best_output_exact_name() -> None:
    src = {"output_ports": [{"name": "image", "type": "Image"}]}
    dst = {"input_ports": [{"name": "image", "type": "Image"}]}
    result = _find_best_output(src, dst)
    assert result == ("image", "image")


def test_find_best_output_type_match() -> None:
    src = {"output_ports": [{"name": "out", "type": "Image"}]}
    dst = {"input_ports": [{"name": "input_img", "type": "Image"}]}
    result = _find_best_output(src, dst)
    assert result == ("out", "input_img")


def test_find_best_output_no_ports() -> None:
    assert _find_best_output({"output_ports": []}, {"input_ports": [{"name": "x", "type": "Image"}]}) is None
    assert _find_best_output({"output_ports": [{"name": "x", "type": "Image"}]}, {"input_ports": []}) is None


# ---------------------------------------------------------------------------
# build_graph — linear topology
# ---------------------------------------------------------------------------

_CORPUS = {
    "load_image": {
        "id": "load_image",
        "input_ports": [],
        "output_ports": [{"name": "image", "type": "Image"}],
        "parameters": [],
        "category": "Input",
    },
    "gaussian_blur": {
        "id": "gaussian_blur",
        "input_ports": [{"name": "image", "type": "Image"}],
        "output_ports": [{"name": "image", "type": "Image"}],
        "parameters": [{"name": "sigma", "type": "Float", "default": "2.0"}],
        "category": "Blur",
    },
    "save_image": {
        "id": "save_image",
        "input_ports": [{"name": "image", "type": "Image"}],
        "output_ports": [],
        "parameters": [],
        "category": "Output",
    },
    "blend": {
        "id": "blend",
        "input_ports": [
            {"name": "base", "type": "Image"},
            {"name": "overlay", "type": "Image"},
        ],
        "output_ports": [{"name": "image", "type": "Image"}],
        "parameters": [{"name": "opacity", "type": "Float", "default": "0.5"}],
        "category": "Composite",
    },
}


def test_build_linear_graph() -> None:
    selections = [
        {"filter_id": "load_image", "parameters": {"path": "in.png"}},
        {"filter_id": "gaussian_blur", "parameters": {"sigma": 2.0}},
        {"filter_id": "save_image", "parameters": {"path": "out.png"}},
    ]
    plan = {"topology": "linear", "steps": []}
    graph = build_graph(selections, plan, _CORPUS, "blur and save")

    assert graph["version"] == "1.0.0"
    assert len(graph["nodes"]) == 3
    assert len(graph["connections"]) == 2

    # Connections should chain sequentially: n1→n2, n2→n3
    conns = graph["connections"]
    assert conns[0]["from_node"] == "n1"
    assert conns[0]["to_node"] == "n2"
    assert conns[1]["from_node"] == "n2"
    assert conns[1]["to_node"] == "n3"


def test_build_linear_graph_single_node() -> None:
    selections = [{"filter_id": "load_image", "parameters": {}}]
    plan = {"topology": "linear", "steps": []}
    graph = build_graph(selections, plan, _CORPUS, "just load")

    assert len(graph["nodes"]) == 1
    assert len(graph["connections"]) == 0


def test_build_linear_graph_preserves_parameters() -> None:
    selections = [
        {"filter_id": "load_image", "parameters": {"path": "test.png"}},
        {"filter_id": "gaussian_blur", "parameters": {"sigma": 5.0}},
    ]
    plan = {"topology": "linear", "steps": []}
    graph = build_graph(selections, plan, _CORPUS, "blur")

    assert graph["nodes"][0]["parameters"]["path"] == "test.png"
    assert graph["nodes"][1]["parameters"]["sigma"] == 5.0


def test_build_linear_graph_metadata() -> None:
    selections = [{"filter_id": "load_image", "parameters": {}}]
    plan = {"topology": "linear", "steps": []}
    graph = build_graph(selections, plan, _CORPUS, "test query")

    assert graph["metadata"]["query"] == "test query"
    assert graph["metadata"]["topology"] == "linear"
    assert graph["metadata"]["generatedBy"] == "ambara-agentic-pipeline"


# ---------------------------------------------------------------------------
# build_graph — branch topology
# ---------------------------------------------------------------------------


def test_build_branch_graph() -> None:
    selections = [
        {"filter_id": "load_image", "parameters": {}},
        {"filter_id": "load_image", "parameters": {}},
        {"filter_id": "blend", "parameters": {"opacity": 0.5}},
        {"filter_id": "save_image", "parameters": {}},
    ]
    plan = {"topology": "branch", "steps": []}
    graph = build_graph(selections, plan, _CORPUS, "blend two images")

    assert graph["metadata"]["topology"] == "branch"
    # Should have 4 nodes: 2 inputs, 1 merge, 1 output
    assert len(graph["nodes"]) == 4
    # Should have connections from both inputs to merge and merge to output
    assert len(graph["connections"]) >= 3


def test_build_branch_graph_auto_adds_merge() -> None:
    """Branch with no merge filter should auto-add blend."""
    selections = [
        {"filter_id": "load_image", "parameters": {}},
        {"filter_id": "load_image", "parameters": {}},
        {"filter_id": "save_image", "parameters": {}},
    ]
    plan = {"topology": "branch", "steps": []}
    graph = build_graph(selections, plan, _CORPUS, "composite two images")

    filter_ids = [n["filter_id"] for n in graph["nodes"]]
    assert "blend" in filter_ids
