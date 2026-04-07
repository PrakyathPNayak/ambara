"""Unit tests for the agentic router."""

from __future__ import annotations

import json
from unittest.mock import MagicMock, patch

import pytest

from chatbot.generation.agent import Agent, AgentResult, MAX_TOOL_ROUNDS


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


def _make_agent(llm_backend: str = "mock") -> Agent:
    """Build an Agent with a mock LLM client and minimal retriever."""
    llm = MagicMock()
    llm.backend = llm_backend
    llm.model_name = "test-model"

    retriever = MagicMock()
    retriever.all_filter_ids = ["gaussian_blur", "load_image", "save_image"]
    retriever.get_category_summary.return_value = "Blur (1): gaussian_blur"
    retriever.search.return_value = []

    generator = MagicMock()
    return Agent(llm_client=llm, retriever=retriever, generator=generator)


# ---------------------------------------------------------------------------
# AgentResult
# ---------------------------------------------------------------------------


def test_agent_result_graph_generated_true() -> None:
    r = AgentResult(reply="ok", graph={"nodes": []})
    assert r.graph_generated is True


def test_agent_result_graph_generated_false() -> None:
    r = AgentResult(reply="ok", graph=None)
    assert r.graph_generated is False


def test_agent_result_defaults() -> None:
    r = AgentResult(reply="hi")
    assert r.tool_calls == []
    assert r.graph is None


# ---------------------------------------------------------------------------
# Mock mode (deterministic branch)
# ---------------------------------------------------------------------------


def test_mock_mode_graph_keywords() -> None:
    """Messages containing graph keywords should trigger generate_graph in mock mode."""
    agent = _make_agent("mock")
    # The tool executor returns a generation result; mock it
    agent.tool_executor.execute = MagicMock(return_value=json.dumps({
        "valid": True,
        "graph": {"nodes": [{"id": "n1"}], "connections": []},
    }))
    result = agent.run("build a pipeline to blur images")
    assert result.graph_generated is True
    assert result.graph is not None
    agent.tool_executor.execute.assert_called_once()


def test_mock_mode_filter_match() -> None:
    """A message containing a known filter ID should explain that filter."""
    agent = _make_agent("mock")
    agent.tool_executor.execute = MagicMock(return_value="Gaussian blur applies a smoothing kernel.")
    result = agent.run("tell me about gaussian_blur")
    assert "gaussian" in result.reply.lower() or "blur" in result.reply.lower()
    assert result.graph is None


def test_mock_mode_fallback_search() -> None:
    """Unknown messages fall back to search in mock mode."""
    agent = _make_agent("mock")
    agent.retriever.search.return_value = []
    result = agent.run("what's the meaning of life?")
    assert result.reply  # Should get the category summary fallback
    assert result.graph is None


# ---------------------------------------------------------------------------
# LLM-based agent loop
# ---------------------------------------------------------------------------


def test_llm_direct_answer() -> None:
    """LLM responds with a direct answer — no tool calls."""
    agent = _make_agent("anthropic")
    agent.llm.generate.return_value = json.dumps({"answer": "Ambara has 42 filters."})

    result = agent.run("how many filters?")
    assert result.reply == "Ambara has 42 filters."
    assert result.tool_calls == []
    assert result.graph is None


def test_llm_tool_then_answer() -> None:
    """LLM calls search_filters, receives result, then produces an answer."""
    agent = _make_agent("anthropic")
    agent.llm.generate.side_effect = [
        json.dumps({"tool": "search_filters", "arguments": {"query": "blur"}}),
        json.dumps({"answer": "Found gaussian_blur."}),
    ]
    agent.tool_executor = MagicMock()
    agent.tool_executor.execute.return_value = json.dumps([{"id": "gaussian_blur"}])

    result = agent.run("search for blur filters")
    assert result.reply == "Found gaussian_blur."
    assert len(result.tool_calls) == 1
    assert result.tool_calls[0]["tool"] == "search_filters"


def test_llm_graph_generation_tool() -> None:
    """LLM calls generate_graph tool and returns a graph."""
    graph = {"nodes": [{"id": "n1"}, {"id": "n2"}], "connections": []}
    agent = _make_agent("anthropic")
    agent.llm.generate.side_effect = [
        json.dumps({"tool": "generate_graph", "arguments": {"query": "blur pipeline"}}),
        json.dumps({"answer": "Built a 2-node pipeline."}),
    ]
    agent.tool_executor = MagicMock()
    agent.tool_executor.execute.return_value = json.dumps({"valid": True, "graph": graph})

    result = agent.run("build blur pipeline")
    assert result.graph_generated is True
    assert result.graph is not None
    assert len(result.graph["nodes"]) == 2
    assert result.reply == "Built a 2-node pipeline."


def test_llm_error_returns_error_reply() -> None:
    """When LLM raises RuntimeError, agent returns a graceful error message."""
    agent = _make_agent("anthropic")
    agent.llm.generate.side_effect = RuntimeError("connection refused")

    result = agent.run("hello")
    assert "error" in result.reply.lower()
    assert result.graph is None


def test_llm_exhaustion_with_graph() -> None:
    """When tool rounds are exhausted but a graph exists, return it."""
    graph = {"nodes": [{"id": "n1"}], "connections": []}
    agent = _make_agent("anthropic")

    # Every generate call returns a tool call — exhaust all rounds
    tool_responses = [
        json.dumps({"tool": "search_filters", "arguments": {"query": "x"}})
    ] * MAX_TOOL_ROUNDS
    # After exhaustion, agent makes one more call for summary
    tool_responses.append(json.dumps({"answer": "summary"}))
    agent.llm.generate.side_effect = tool_responses

    agent.tool_executor = MagicMock()

    # First call generates graph, rest are searches
    def tool_exec(name: str, args: dict) -> str:
        if name == "generate_graph":
            return json.dumps({"valid": True, "graph": graph})
        return "[]"

    agent.tool_executor.execute.side_effect = tool_exec

    # Patch tool calls so the first one is generate_graph
    agent.llm.generate.side_effect = [
        json.dumps({"tool": "generate_graph", "arguments": {"query": "blur"}}),
    ] + [
        json.dumps({"tool": "search_filters", "arguments": {"query": "x"}})
    ] * (MAX_TOOL_ROUNDS - 1) + [
        json.dumps({"answer": "done"}),  # final summary call
    ]

    result = agent.run("complex request")
    # Should have the graph from the generate_graph call
    assert result.graph is not None


def test_llm_exhaustion_no_graph_makes_summary_call() -> None:
    """When exhausted with no graph, agent asks LLM for a final summary."""
    agent = _make_agent("anthropic")
    tool_calls = [
        json.dumps({"tool": "search_filters", "arguments": {"query": "x"}})
    ] * MAX_TOOL_ROUNDS
    tool_calls.append(json.dumps({"answer": "Based on my research, try gaussian_blur."}))
    agent.llm.generate.side_effect = tool_calls
    agent.tool_executor = MagicMock()
    agent.tool_executor.execute.return_value = "[]"

    result = agent.run("some complex question")
    assert "gaussian_blur" in result.reply


# ---------------------------------------------------------------------------
# _parse_response
# ---------------------------------------------------------------------------


def test_parse_json_direct() -> None:
    agent = _make_agent("mock")
    assert agent._parse_response('{"answer": "yes"}') == {"answer": "yes"}


def test_parse_json_in_code_fence() -> None:
    agent = _make_agent("mock")
    raw = '```json\n{"tool": "search_filters", "arguments": {"query": "blur"}}\n```'
    parsed = agent._parse_response(raw)
    assert parsed["tool"] == "search_filters"


def test_parse_json_with_surrounding_text() -> None:
    agent = _make_agent("mock")
    raw = 'Let me search for that.\n{"tool": "search_filters", "arguments": {"query": "blur"}}\nDone.'
    parsed = agent._parse_response(raw)
    assert parsed["tool"] == "search_filters"


def test_parse_unparseable_returns_answer() -> None:
    agent = _make_agent("mock")
    parsed = agent._parse_response("Just a plain text response with no JSON.")
    assert parsed.get("answer") == "Just a plain text response with no JSON."


def test_parse_strips_thinking_tags() -> None:
    agent = _make_agent("mock")
    raw = '<think>Let me reason about this.</think>{"answer": "42"}'
    parsed = agent._parse_response(raw)
    assert parsed == {"answer": "42"}


# ---------------------------------------------------------------------------
# _build_messages
# ---------------------------------------------------------------------------


def test_build_messages_no_history() -> None:
    agent = _make_agent("mock")
    msgs = agent._build_messages("hello", None)
    assert msgs[0]["role"] == "system"
    assert msgs[-1] == {"role": "user", "content": "hello"}
    assert len(msgs) == 2


def test_build_messages_with_history() -> None:
    agent = _make_agent("mock")
    history = [
        {"role": "user", "content": "previous question"},
        {"role": "assistant", "content": "previous answer"},
    ]
    msgs = agent._build_messages("follow-up", history)
    assert len(msgs) == 4  # system + 2 history + user
    assert msgs[-1]["content"] == "follow-up"


def test_build_messages_truncates_long_history() -> None:
    agent = _make_agent("mock")
    history = [{"role": "user", "content": f"msg{i}"} for i in range(20)]
    msgs = agent._build_messages("final", history)
    # system + 12 recent history + user = 14
    assert len(msgs) == 14
