"""Agentic router for the Ambara chatbot.

Replaces the simple keyword-based IntentClassifier with an LLM-powered agent
that uses tool calls to decide what to do.  The agent can:

  - Search for filters and explain them
  - Generate processing graphs
  - Suggest pipelines
  - Explain existing graphs
  - Have conversational follow-ups using session history

The agent follows a ReAct (Reason + Act) loop: it thinks about what to do,
picks a tool, gets the result, and decides whether to answer or keep going.
"""

from __future__ import annotations

import json
import logging
import re
from typing import Any

from chatbot.generation.llm_client import LLMClient
from chatbot.generation.tools import TOOL_SCHEMAS, ToolExecutor, format_tool_schemas_for_prompt
from chatbot.retrieval.code_retriever import CodeRetriever

LOGGER = logging.getLogger(__name__)

MAX_TOOL_ROUNDS = 4  # Maximum tool-use iterations before forcing a final answer

SYSTEM_PROMPT = """\
You are Ambara Assistant, an expert on the Ambara image processing application.
Ambara uses a node-based graph system where filters are connected to form pipelines.

Your capabilities:
{tools}

INSTRUCTIONS:
1. Analyze the user's message and conversation history to understand their intent.
2. If the user wants to BUILD a pipeline or graph, use the generate_graph tool.
3. If the user asks ABOUT a specific filter, use explain_filter or get_filter_details.
4. If the user asks what filters are available for a task, use search_filters.
5. If the user wants to know what categories exist, use list_categories.
6. If the user wants pipeline suggestions without building one, use suggest_pipeline.
7. If the user wants to understand an existing graph, use explain_graph.
8. For general questions you can answer from context, just respond directly.

RESPONSE FORMAT:
- To call a tool, respond with exactly one JSON block:
  ```json
  {{"tool": "<tool_name>", "arguments": {{...}}}}
  ```
- To give a final answer to the user (no more tools needed), respond with:
  ```json
  {{"answer": "<your response to the user>"}}
  ```
- Always respond with ONE of these JSON formats. Never mix them.
- After receiving a tool result, you may call another tool or give a final answer.

GUIDELINES:
- Be concise but informative.  Users are technical and appreciate specific details.
- When generating graphs, describe what you built and how many nodes/filters it uses.
- When explaining filters, mention ports, parameters, defaults, and constraints.
- Reference filter IDs with backtick formatting: `gaussian_blur`.
- If you're unsure which filter to suggest, search first before answering.
- Use conversation history to understand follow-up questions and context.

{catalog}
"""


class Agent:
    """LLM-powered agentic router with tool-use capabilities."""

    def __init__(
        self,
        llm_client: LLMClient,
        retriever: CodeRetriever,
        generator: Any = None,
    ) -> None:
        self.llm = llm_client
        self.retriever = retriever
        self.tool_executor = ToolExecutor(retriever, generator)

        # Build system prompt with tool schemas and filter catalog
        catalog = retriever.get_category_summary()
        tools_text = format_tool_schemas_for_prompt()
        self.system_prompt = SYSTEM_PROMPT.format(tools=tools_text, catalog=catalog)

    def run(
        self,
        user_message: str,
        session_history: list[dict[str, Any]] | None = None,
    ) -> AgentResult:
        """Run the agent loop for a user message.

        Returns an AgentResult with the final reply, any generated graph,
        and metadata about tool calls made.
        """
        if self.llm.backend == "mock":
            return self._mock_response(user_message)

        messages = self._build_messages(user_message, session_history)

        tool_calls_made: list[dict[str, Any]] = []
        graph_result: dict[str, Any] | None = None

        for round_num in range(MAX_TOOL_ROUNDS):
            try:
                raw = self.llm.generate(
                    {"messages": messages},
                    temperature=0.0,
                )
            except RuntimeError as err:
                LOGGER.error("Agent LLM call failed: %s", err)
                return AgentResult(
                    reply=f"I encountered an error communicating with the LLM: {err}",
                    graph=None,
                    tool_calls=[],
                )

            parsed = self._parse_response(raw)

            if parsed.get("answer"):
                return AgentResult(
                    reply=parsed["answer"],
                    graph=graph_result,
                    tool_calls=tool_calls_made,
                )

            if parsed.get("tool"):
                tool_name = parsed["tool"]
                arguments = parsed.get("arguments", {})
                LOGGER.info("Agent tool call [round %d]: %s(%s)", round_num + 1, tool_name, arguments)

                result_str = self.tool_executor.execute(tool_name, arguments)
                tool_calls_made.append({
                    "tool": tool_name,
                    "arguments": arguments,
                    "result_preview": result_str[:500],
                })

                # Check if generate_graph returned a valid graph
                if tool_name == "generate_graph":
                    try:
                        gen_result = json.loads(result_str)
                        if gen_result.get("valid") and gen_result.get("graph"):
                            graph_result = gen_result["graph"]
                    except (json.JSONDecodeError, KeyError):
                        pass

                # Add tool result to conversation for next round
                messages.append({"role": "assistant", "content": raw})
                messages.append({
                    "role": "user",
                    "content": f"Tool result from {tool_name}:\n{result_str[:2000]}",
                })
                continue

            # If we can't parse a tool call or answer, treat the raw response as the answer
            return AgentResult(
                reply=self._clean_response(raw),
                graph=graph_result,
                tool_calls=tool_calls_made,
            )

        # Exhausted tool rounds — force a final answer
        return AgentResult(
            reply="I've gathered information but couldn't formulate a complete answer. "
                  "Please try rephrasing your request.",
            graph=graph_result,
            tool_calls=tool_calls_made,
        )

    def _build_messages(
        self,
        user_message: str,
        session_history: list[dict[str, Any]] | None,
    ) -> list[dict[str, Any]]:
        """Build the message list for the LLM call."""
        messages: list[dict[str, Any]] = [
            {"role": "system", "content": self.system_prompt},
        ]

        # Add recent session history (last 6 turns max to stay within context)
        if session_history:
            recent = session_history[-12:]  # 6 user + 6 assistant turns
            for msg in recent:
                role = msg.get("role", "user")
                content = msg.get("content", "")
                if role in ("user", "assistant") and content:
                    messages.append({"role": role, "content": content})

        messages.append({"role": "user", "content": user_message})
        return messages

    def _parse_response(self, raw: str) -> dict[str, Any]:
        """Parse the LLM response to extract tool call or final answer."""
        text = raw.strip()

        # Strip qwen3 thinking tags
        text = re.sub(r"<think>.*?</think>", "", text, flags=re.DOTALL).strip()

        # Try to find JSON in the response
        # First try: entire response is JSON
        try:
            parsed = json.loads(text)
            if isinstance(parsed, dict):
                return parsed
        except json.JSONDecodeError:
            pass

        # Second try: JSON within markdown code fences
        fence_match = re.search(r"```(?:json)?\s*\n?(.*?)```", text, re.DOTALL)
        if fence_match:
            try:
                parsed = json.loads(fence_match.group(1).strip())
                if isinstance(parsed, dict):
                    return parsed
            except json.JSONDecodeError:
                pass

        # Third try: find first { ... } block
        brace_start = text.find("{")
        if brace_start >= 0:
            brace_end = text.rfind("}")
            if brace_end > brace_start:
                try:
                    parsed = json.loads(text[brace_start:brace_end + 1])
                    if isinstance(parsed, dict):
                        return parsed
                except json.JSONDecodeError:
                    pass

        # Can't parse — return the raw text as an answer
        return {"answer": text}

    def _clean_response(self, raw: str) -> str:
        """Clean up a raw LLM response for display."""
        text = raw.strip()
        text = re.sub(r"<think>.*?</think>", "", text, flags=re.DOTALL).strip()
        # Remove any JSON wrapper if present
        try:
            parsed = json.loads(text)
            if isinstance(parsed, dict) and "answer" in parsed:
                return parsed["answer"]
        except (json.JSONDecodeError, KeyError):
            pass
        return text

    def _mock_response(self, user_message: str) -> AgentResult:
        """Generate a deterministic response in mock mode."""
        q = user_message.lower()

        # Check if it's a graph generation request
        graph_keywords = {"blur", "resize", "crop", "rotate", "save", "load",
                          "pipeline", "workflow", "graph", "apply", "process",
                          "build", "create", "make"}
        if any(kw in q for kw in graph_keywords):
            result_str = self.tool_executor.execute("generate_graph", {"query": user_message})
            try:
                gen_result = json.loads(result_str)
                if gen_result.get("valid") and gen_result.get("graph"):
                    graph = gen_result["graph"]
                    node_count = len(graph.get("nodes", []))
                    return AgentResult(
                        reply=f"I built a {node_count}-node pipeline. Click 'Insert Graph' to load it.",
                        graph=graph,
                        tool_calls=[{"tool": "generate_graph", "arguments": {"query": user_message}}],
                    )
            except (json.JSONDecodeError, KeyError):
                pass

        # Check if it's a question about a specific filter
        for fid in self.retriever.all_filter_ids:
            if fid in q:
                explanation = self.tool_executor.execute("explain_filter", {"filter_id": fid})
                return AgentResult(
                    reply=explanation,
                    graph=None,
                    tool_calls=[{"tool": "explain_filter", "arguments": {"filter_id": fid}}],
                )

        # Default: search and suggest
        results = self.retriever.search(user_message, top_k=3)
        if results:
            descriptions = "; ".join(f"`{f.id}` — {f.description[:60]}" for f in results[:3])
            return AgentResult(
                reply=f"Here are some relevant Ambara filters: {descriptions}. "
                      "Describe your goal and I'll build a pipeline for you.",
                graph=None,
                tool_calls=[{"tool": "search_filters", "arguments": {"query": user_message}}],
            )

        summary = self.retriever.get_category_summary()
        return AgentResult(
            reply=f"I can help you with Ambara's image processing capabilities. {summary}\n\n"
                  "Tell me what you'd like to do and I'll build a pipeline or explain the right filters.",
            graph=None,
            tool_calls=[],
        )


class AgentResult:
    """Result from an agent run."""

    __slots__ = ("reply", "graph", "tool_calls")

    def __init__(
        self,
        reply: str,
        graph: dict[str, Any] | None = None,
        tool_calls: list[dict[str, Any]] | None = None,
    ) -> None:
        self.reply = reply
        self.graph = graph
        self.tool_calls = tool_calls or []

    @property
    def graph_generated(self) -> bool:
        return self.graph is not None
