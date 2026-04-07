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
=== ROLE ===
You are Ambara Assistant, an expert on the Ambara image-processing application.
Ambara uses a node-based graph system where filters are connected to form
image-processing pipelines.

=== AVAILABLE TOOLS ===
{tools}

=== CHAIN-OF-THOUGHT REASONING ===
Before choosing a tool or giving an answer, silently reason through these steps:
  1. UNDERSTAND: What is the user actually asking for? Check conversation history
     for follow-ups or clarifications that change the intent.
  2. CLASSIFY: Is this a BUILD request (generate_graph), an INFORMATION request
     (explain_filter, search_filters, get_filter_details, list_categories), a
     SUGGESTION request (suggest_pipeline), an EXPLANATION request (explain_graph),
     or a GENERAL question you can answer directly?
  3. PLAN: Which single tool best addresses the classified intent? If unsure,
     use search_filters first to gather context before answering.
  4. VERIFY: After receiving a tool result, check — does this fully answer the
     user's question? If not, call another tool. If yes, compose the final answer.

=== DECISION RULES (priority order) ===
1. Explicit BUILD/CREATE/MAKE intent → generate_graph.
   IMPORTANT: After generate_graph returns, call validate_graph on the result
   to verify correctness before presenting it to the user.
2. Question about a SPECIFIC filter by name → explain_filter or get_filter_details.
3. "What filters can do X?" → search_filters.
4. "What categories exist?" → list_categories.
5. "Suggest a pipeline for X" (no graph needed) → suggest_pipeline.
6. "Explain this graph" + JSON → explain_graph.
7. Ambiguous requests → search_filters first, then decide.
8. General Ambara knowledge → answer directly from context.

=== RESPONSE FORMAT ===
You MUST respond with exactly ONE JSON object. Two valid forms:

  Tool call:    {{"tool": "<name>", "arguments": {{...}}}}
  Final answer: {{"answer": "<your response>"}}

Never mix them. Never output markdown fences around the JSON. Never output
explanatory text outside the JSON object.

=== WHAT NOT TO DO ===
- Do NOT call generate_graph for questions like "what does gaussian_blur do?".
- Do NOT guess filter IDs. Search first if unsure.
- Do NOT produce empty arguments for required tool parameters.
- Do NOT repeat the same tool call with identical arguments.
- Do NOT fabricate filter names, port names, or parameter names.

=== ANSWER QUALITY ===
- Be concise but informative. Users are technical.
- When generating graphs, describe what you built, how many nodes it uses,
  and the overall topology.
- When explaining filters, include ports, parameters, defaults, and constraints.
- Reference filter IDs in backtick formatting: `gaussian_blur`.
- Use conversation history to maintain coherent multi-turn context.

=== FILTER CATALOG ===
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
        seen_calls: set[str] = set()  # track (tool, args) to avoid duplicates

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

                # Deduplicate tool calls with identical arguments
                call_key = json.dumps({"tool": tool_name, "arguments": arguments}, sort_keys=True)
                if call_key in seen_calls:
                    LOGGER.info("Agent duplicate tool call skipped: %s", tool_name)
                    messages.append({"role": "assistant", "content": raw})
                    messages.append({
                        "role": "user",
                        "content": (
                            f"You already called {tool_name} with these same arguments. "
                            "Use the previous result or try a different approach."
                        ),
                    })
                    continue
                seen_calls.add(call_key)

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
                    "content": f"Tool result from {tool_name}:\n{result_str[:4000]}",
                })
                continue

            # If we can't parse a tool call or answer, treat the raw response as the answer
            return AgentResult(
                reply=self._clean_response(raw),
                graph=graph_result,
                tool_calls=tool_calls_made,
            )

        # Exhausted tool rounds — construct the best reply we can from gathered data
        LOGGER.warning(
            "Agent exhausted %d tool rounds for message: %.100s", MAX_TOOL_ROUNDS, user_message
        )
        if graph_result:
            node_count = len(graph_result.get("nodes", []))
            return AgentResult(
                reply=f"I built a {node_count}-node pipeline for you. "
                      "Click 'Insert Graph' to load it into the canvas.",
                graph=graph_result,
                tool_calls=tool_calls_made,
            )
        # Try one final prompt asking the LLM to summarize what it found
        messages.append({
            "role": "user",
            "content": "Please give a concise final answer based on the information gathered so far.",
        })
        try:
            raw = self.llm.generate({"messages": messages}, temperature=0.0)
            final = self._parse_response(raw)
            if final.get("answer"):
                return AgentResult(
                    reply=final["answer"],
                    graph=graph_result,
                    tool_calls=tool_calls_made,
                )
        except RuntimeError:
            pass
        return AgentResult(
            reply="I gathered some information but couldn't complete the request. "
                  "Could you try being more specific about what you need?",
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
