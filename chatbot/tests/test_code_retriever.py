"""Tests for code-as-RAG retriever, agent, and tools."""

from __future__ import annotations

import json
from pathlib import Path

from chatbot.generation.agent import Agent, AgentResult
from chatbot.generation.llm_client import LLMClient
from chatbot.generation.tools import ToolExecutor, format_tool_schemas_for_prompt
from chatbot.retrieval.code_retriever import CodeRetriever


# ---------------------------------------------------------------------------
# CodeRetriever tests
# ---------------------------------------------------------------------------


def test_code_retriever_loads_filters() -> None:
    retriever = CodeRetriever()
    retriever.refresh()
    ids = retriever.all_filter_ids
    assert len(ids) > 50, f"Expected 50+ filters, got {len(ids)}"


def test_code_retriever_known_filter() -> None:
    retriever = CodeRetriever()
    info = retriever.get("gaussian_blur")
    assert info is not None
    assert info.name == "Gaussian Blur"
    assert info.category == "Blur"
    assert len(info.inputs) >= 1
    assert len(info.outputs) >= 1


def test_code_retriever_search() -> None:
    retriever = CodeRetriever()
    results = retriever.search("blur")
    assert len(results) > 0
    ids = [r.id for r in results]
    assert "gaussian_blur" in ids or "box_blur" in ids


def test_code_retriever_categories() -> None:
    retriever = CodeRetriever()
    cats = retriever.categories
    assert "Blur" in cats or "blur" in cats or any("lur" in c for c in cats)
    assert len(cats) >= 5


def test_code_retriever_compatible_next() -> None:
    retriever = CodeRetriever()
    compat = retriever.get_compatible_next("load_image")
    assert len(compat) > 0
    # load_image outputs Image, so many filters should be compatible
    ids = [c.id for c in compat]
    assert any("blur" in i for i in ids) or any("save" in i for i in ids)


def test_code_retriever_build_catalog() -> None:
    retriever = CodeRetriever()
    catalog = retriever.build_catalog()
    assert "gaussian_blur" in catalog.lower() or "Gaussian" in catalog
    assert len(catalog) > 100


def test_code_retriever_export_corpus(tmp_path: Path) -> None:
    retriever = CodeRetriever()
    out = retriever.export_corpus(tmp_path / "corpus.json")
    assert out.exists()
    data = json.loads(out.read_text())
    assert len(data) > 50
    assert all("id" in item for item in data)


def test_code_retriever_to_dict() -> None:
    retriever = CodeRetriever()
    info = retriever.get("load_image")
    assert info is not None
    d = info.to_dict()
    assert d["id"] == "load_image"
    assert "input_ports" in d
    assert "output_ports" in d
    assert "parameters" in d


def test_code_retriever_compact_card() -> None:
    retriever = CodeRetriever()
    card = retriever.get_filter_card("gaussian_blur")
    assert "gaussian_blur" in card
    assert "Gaussian Blur" in card


def test_code_retriever_rich_context() -> None:
    retriever = CodeRetriever()
    ctx = retriever.get_details("gaussian_blur")
    assert "gaussian_blur" in ctx
    assert "Inputs" in ctx or "Input" in ctx


def test_code_retriever_search_by_category() -> None:
    retriever = CodeRetriever()
    blurs = retriever.search_by_category("Blur")
    assert len(blurs) >= 1
    assert all(f.category == "Blur" for f in blurs)


def test_code_retriever_search_by_port_type() -> None:
    retriever = CodeRetriever()
    image_inputs = retriever.search_by_port_type("Image", direction="input")
    assert len(image_inputs) > 10


def test_code_retriever_api_filters() -> None:
    """Test that the new API filters are extracted correctly."""
    retriever = CodeRetriever()
    # These may or may not exist depending on whether the Rust project compiled
    # with the new api.rs, so we check if the source file exists first.
    api_source = Path(__file__).resolve().parents[2] / "src" / "filters" / "builtin" / "api.rs"
    if api_source.exists():
        info = retriever.get("http_image_fetch")
        assert info is not None, "http_image_fetch not found in parsed filters"
        assert info.category == "Api"

        sd = retriever.get("stable_diffusion_generate")
        assert sd is not None
        assert sd.category == "Api"
        assert any(p.name == "prompt" for p in sd.parameters)

        classify = retriever.get("image_classify")
        assert classify is not None

        inference = retriever.get("model_inference")
        assert inference is not None

        style = retriever.get("style_transfer")
        assert style is not None
        assert len(style.inputs) == 2  # content + style


def test_code_retriever_category_summary() -> None:
    retriever = CodeRetriever()
    summary = retriever.get_category_summary()
    assert "filters" in summary.lower()
    assert "categories" in summary.lower() or "category" in summary.lower()


# ---------------------------------------------------------------------------
# ToolExecutor tests
# ---------------------------------------------------------------------------


def test_tool_schemas_format() -> None:
    text = format_tool_schemas_for_prompt()
    assert "search_filters" in text
    assert "generate_graph" in text
    assert "explain_filter" in text


def test_tool_executor_search() -> None:
    retriever = CodeRetriever()
    executor = ToolExecutor(retriever)
    result = executor.execute("search_filters", {"query": "blur"})
    data = json.loads(result)
    assert isinstance(data, list)
    assert len(data) > 0


def test_tool_executor_get_details() -> None:
    retriever = CodeRetriever()
    executor = ToolExecutor(retriever)
    result = executor.execute("get_filter_details", {"filter_id": "gaussian_blur"})
    assert "gaussian_blur" in result.lower() or "Gaussian" in result


def test_tool_executor_list_categories() -> None:
    retriever = CodeRetriever()
    executor = ToolExecutor(retriever)
    result = executor.execute("list_categories", {})
    data = json.loads(result)
    assert isinstance(data, dict)
    assert len(data) >= 5


def test_tool_executor_compatible_filters() -> None:
    retriever = CodeRetriever()
    executor = ToolExecutor(retriever)
    result = executor.execute("get_compatible_filters", {"filter_id": "load_image"})
    data = json.loads(result)
    assert isinstance(data, list)
    assert len(data) > 0


def test_tool_executor_explain_filter() -> None:
    retriever = CodeRetriever()
    executor = ToolExecutor(retriever)
    result = executor.execute("explain_filter", {"filter_id": "gaussian_blur"})
    assert "Gaussian" in result or "blur" in result.lower()
    assert "Parameters" in result or "parameter" in result.lower()


def test_tool_executor_suggest_pipeline() -> None:
    retriever = CodeRetriever()
    executor = ToolExecutor(retriever)
    result = executor.execute("suggest_pipeline", {"goal": "denoise and sharpen an image"})
    assert "pipeline" in result.lower() or "denoise" in result.lower() or "sharpen" in result.lower()


def test_tool_executor_unknown_tool() -> None:
    retriever = CodeRetriever()
    executor = ToolExecutor(retriever)
    result = executor.execute("nonexistent_tool", {})
    data = json.loads(result)
    assert "error" in data


def test_tool_executor_explain_graph() -> None:
    retriever = CodeRetriever()
    executor = ToolExecutor(retriever)
    graph = {
        "nodes": [
            {"id": "n1", "filter_id": "load_image", "parameters": {"path": "test.png"}},
            {"id": "n2", "filter_id": "gaussian_blur", "parameters": {"sigma": 2.0}},
        ],
        "connections": [
            {"from_node": "n1", "from_port": "image", "to_node": "n2", "to_port": "image"}
        ],
    }
    result = executor.execute("explain_graph", {"graph_json": json.dumps(graph)})
    assert "2 nodes" in result
    assert "1 connection" in result


# ---------------------------------------------------------------------------
# Agent tests (mock mode)
# ---------------------------------------------------------------------------


def test_agent_mock_graph_generation() -> None:
    llm = LLMClient(force_mock=True)
    retriever = CodeRetriever()
    from chatbot.generation.graph_generator import GraphGenerator

    generator = GraphGenerator(code_retriever=retriever, force_mock_llm=True)
    agent = Agent(llm_client=llm, retriever=retriever, generator=generator)

    result = agent.run("blur an image and save it")
    assert isinstance(result, AgentResult)
    assert result.reply
    assert result.graph_generated
    assert result.graph is not None


def test_agent_mock_filter_question() -> None:
    llm = LLMClient(force_mock=True)
    retriever = CodeRetriever()
    agent = Agent(llm_client=llm, retriever=retriever)

    result = agent.run("what does gaussian_blur do?")
    assert isinstance(result, AgentResult)
    assert result.reply
    assert "gaussian_blur" in result.reply.lower() or "Gaussian" in result.reply


def test_agent_mock_general_question() -> None:
    llm = LLMClient(force_mock=True)
    retriever = CodeRetriever()
    agent = Agent(llm_client=llm, retriever=retriever)

    result = agent.run("hello")
    assert isinstance(result, AgentResult)
    assert result.reply
    assert len(result.reply) > 10


def test_agent_result_properties() -> None:
    r1 = AgentResult(reply="test", graph=None)
    assert not r1.graph_generated

    r2 = AgentResult(reply="test", graph={"nodes": [], "connections": []})
    assert r2.graph_generated
