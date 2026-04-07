"""Tool definitions for the Ambara agentic chatbot.

Each tool is a callable that the agent can invoke.  Tools operate on the
CodeRetriever and GraphGenerator to answer user questions, search for
filters, generate graphs, and explain pipeline concepts.
"""

from __future__ import annotations

import json
import logging
from typing import Any

from chatbot.retrieval.code_retriever import CodeRetriever

LOGGER = logging.getLogger(__name__)


# ---------------------------------------------------------------------------
# Tool registry
# ---------------------------------------------------------------------------

TOOL_SCHEMAS: list[dict[str, Any]] = [
    {
        "name": "search_filters",
        "description": (
            "Search the Ambara filter library by keyword or concept.  "
            "Returns a list of matching filters with their id, name, "
            "description, inputs, outputs, and parameters."
        ),
        "parameters": {
            "query": {"type": "string", "description": "Search keywords (e.g. 'blur', 'resize', 'edge detection')."},
            "top_k": {"type": "integer", "description": "Max results to return.", "default": 5},
        },
        "required": ["query"],
    },
    {
        "name": "get_filter_details",
        "description": (
            "Get detailed information about a specific filter including its "
            "ports, parameters, constraints, and implementation source code."
        ),
        "parameters": {
            "filter_id": {"type": "string", "description": "The filter ID (e.g. 'gaussian_blur')."},
        },
        "required": ["filter_id"],
    },
    {
        "name": "list_categories",
        "description": (
            "List all available filter categories and the filters in each category."
        ),
        "parameters": {},
        "required": [],
    },
    {
        "name": "get_compatible_filters",
        "description": (
            "Find filters that can connect after a given filter (output port type matching)."
        ),
        "parameters": {
            "filter_id": {"type": "string", "description": "The filter to find downstream connections for."},
        },
        "required": ["filter_id"],
    },
    {
        "name": "generate_graph",
        "description": (
            "Generate an Ambara processing graph from a natural language description.  "
            "Use this when the user wants to BUILD a pipeline, not just learn about filters."
        ),
        "parameters": {
            "query": {"type": "string", "description": "Natural language description of the desired pipeline."},
        },
        "required": ["query"],
    },
    {
        "name": "explain_filter",
        "description": (
            "Provide a detailed explanation of what a filter does, how to use it, "
            "what parameters it accepts, and common use cases."
        ),
        "parameters": {
            "filter_id": {"type": "string", "description": "The filter ID to explain."},
        },
        "required": ["filter_id"],
    },
    {
        "name": "suggest_pipeline",
        "description": (
            "Suggest a sequence of filters that would accomplish a high-level "
            "image processing goal.  Returns a description of the pipeline, "
            "not the actual graph JSON."
        ),
        "parameters": {
            "goal": {"type": "string", "description": "High-level goal (e.g. 'reduce noise in astrophotography')."},
        },
        "required": ["goal"],
    },
    {
        "name": "explain_graph",
        "description": (
            "Explain what an existing graph does step by step, given its JSON."
        ),
        "parameters": {
            "graph_json": {"type": "string", "description": "The graph JSON to explain."},
        },
        "required": ["graph_json"],
    },
    {
        "name": "validate_graph",
        "description": (
            "Validate a generated graph JSON for correctness: checks schema, "
            "filter IDs, port connections, type compatibility, and topology (cycles/orphans). "
            "Use this AFTER generate_graph to verify the result."
        ),
        "parameters": {
            "graph_json": {"type": "string", "description": "The graph JSON to validate."},
        },
        "required": ["graph_json"],
    },
]


def format_tool_schemas_for_prompt() -> str:
    """Format tool schemas as a compact string for inclusion in LLM prompts."""
    lines = []
    for tool in TOOL_SCHEMAS:
        params = ", ".join(
            f"{k}: {v['type']}" + (" (required)" if k in tool.get("required", []) else "")
            for k, v in tool.get("parameters", {}).items()
        )
        lines.append(f"- {tool['name']}({params}): {tool['description']}")
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# Tool implementations
# ---------------------------------------------------------------------------

class ToolExecutor:
    """Executes tool calls using a CodeRetriever and optional graph generator."""

    def __init__(self, retriever: CodeRetriever, generator: Any = None) -> None:
        self.retriever = retriever
        self.generator = generator
        self._dispatch = {
            "search_filters": self._search_filters,
            "get_filter_details": self._get_filter_details,
            "list_categories": self._list_categories,
            "get_compatible_filters": self._get_compatible_filters,
            "generate_graph": self._generate_graph,
            "explain_filter": self._explain_filter,
            "suggest_pipeline": self._suggest_pipeline,
            "explain_graph": self._explain_graph,
            "validate_graph": self._validate_graph,
        }

    def execute(self, tool_name: str, arguments: dict[str, Any]) -> str:
        """Execute a tool call and return the result as a string."""
        handler = self._dispatch.get(tool_name)
        if not handler:
            return json.dumps({"error": f"Unknown tool: {tool_name}"})
        try:
            result = handler(**arguments)
            return result if isinstance(result, str) else json.dumps(result, indent=2)
        except Exception as exc:
            LOGGER.exception("Tool %s failed", tool_name)
            return json.dumps({"error": f"Tool '{tool_name}' failed: {exc}"})

    def _search_filters(self, query: str, top_k: int = 5) -> list[dict[str, Any]]:
        results = self.retriever.search(query, top_k=top_k)
        return [
            {
                "id": f.id,
                "name": f.name,
                "description": f.description,
                "category": f.category,
                "inputs": [{"name": p.name, "type": p.port_type} for p in f.inputs],
                "outputs": [{"name": p.name, "type": p.port_type} for p in f.outputs],
                "parameters": [
                    {"name": p.name, "type": p.port_type, "default": p.default, "description": p.description}
                    for p in f.parameters
                ],
            }
            for f in results
        ]

    def _get_filter_details(self, filter_id: str) -> str:
        return self.retriever.get_details(filter_id)

    def _list_categories(self) -> dict[str, list[str]]:
        return self.retriever.categories

    def _get_compatible_filters(self, filter_id: str) -> list[dict[str, str]]:
        results = self.retriever.get_compatible_next(filter_id)
        return [{"id": f.id, "name": f.name, "category": f.category} for f in results]

    def _generate_graph(self, query: str) -> str:
        if not self.generator:
            return json.dumps({"error": "Graph generator not available."})
        result = self.generator.generate(query)
        return json.dumps({
            "graph": result.graph,
            "valid": result.valid,
            "errors": result.errors,
            "explanation": result.explanation,
        })

    def _explain_filter(self, filter_id: str) -> str:
        info = self.retriever.get(filter_id)
        if not info:
            return f"Filter '{filter_id}' not found in the Ambara library."

        lines = [
            f"**{info.name}** (`{info.id}`)",
            f"*Category: {info.category}*",
            "",
            info.description,
            "",
        ]

        if info.inputs:
            lines.append("**Input Ports:**")
            for p in info.inputs:
                lines.append(f"  - `{p.name}` ({p.port_type}): {p.description}")

        if info.outputs:
            lines.append("**Output Ports:**")
            for p in info.outputs:
                lines.append(f"  - `{p.name}` ({p.port_type}): {p.description}")

        if info.parameters:
            lines.append("**Parameters:**")
            for p in info.parameters:
                constraint = f" [{p.constraint}]" if p.constraint else ""
                ui = f" (UI: {p.ui_hint})" if p.ui_hint else ""
                lines.append(f"  - `{p.name}` ({p.port_type}, default={p.default}){constraint}{ui}: {p.description}")

        # Suggest connected filters
        compatible = self.retriever.get_compatible_next(filter_id)
        if compatible:
            sample = [f.id for f in compatible[:6]]
            lines.append(f"\n**Can connect to:** {', '.join(sample)}")

        return "\n".join(lines)

    def _suggest_pipeline(self, goal: str) -> str:
        # Search for relevant filters and build a natural language suggestion
        results = self.retriever.search(goal, top_k=8)
        if not results:
            return "I couldn't find relevant filters for that goal."

        # Group by likely pipeline order (input → processing → output)
        processing = [f for f in results if f.category not in ("Input", "Output", "Utility")]

        lines = [f"**Suggested pipeline for: {goal}**\n"]
        step = 1

        # Always start with an input step
        input_filters = [f for f in results if f.category in ("Input", "Utility")]
        if input_filters:
            f = input_filters[0]
            lines.append(f"{step}. **{f.name}** (`{f.id}`): {f.description}")
        else:
            lines.append(f"{step}. **Load Image** (`load_image`): Load the input image")
        step += 1

        for f in processing[:5]:
            lines.append(f"{step}. **{f.name}** (`{f.id}`): {f.description}")
            step += 1

        # Always end with an output step
        output_filters = [f for f in results if f.category in ("Output",)]
        if output_filters:
            f = output_filters[0]
            lines.append(f"{step}. **{f.name}** (`{f.id}`): {f.description}")
        else:
            lines.append(f"{step}. **Save Image** (`save_image`): Save the result")

        return "\n".join(lines)

    def _explain_graph(self, graph_json: str) -> str:
        try:
            graph = json.loads(graph_json)
        except json.JSONDecodeError:
            return "Invalid JSON provided."

        nodes = graph.get("nodes", [])
        connections = graph.get("connections", [])

        lines = [f"This graph has **{len(nodes)} nodes** and **{len(connections)} connections**.\n"]
        lines.append("**Step-by-step:**")

        # Build connection map
        conn_map: dict[str, list[str]] = {}
        for c in connections:
            src = c.get("from_node", "")
            dst = c.get("to_node", "")
            conn_map.setdefault(src, []).append(dst)

        for i, node in enumerate(nodes, 1):
            fid = node.get("filter_id", "unknown")
            nid = node.get("id", "?")
            params = node.get("parameters", {})
            info = self.retriever.get(fid)
            desc = info.description if info else fid
            param_str = ", ".join(f"{k}={v}" for k, v in params.items()) if params else "defaults"
            targets = conn_map.get(nid, [])
            conn_str = f" → {', '.join(targets)}" if targets else ""
            lines.append(f"  {i}. `{nid}` ({fid}): {desc} [{param_str}]{conn_str}")

        return "\n".join(lines)

    def _validate_graph(self, graph_json: str) -> str:
        """Validate graph JSON using the full GraphValidator pipeline."""
        from chatbot.generation.graph_validator import GraphValidator
        import os

        build_dir = os.path.join(os.path.dirname(__file__), "..", "..", "build")
        schema_path = os.path.join(build_dir, "filter_corpus.json")  # reuse for schema
        filter_ids_path = os.path.join(build_dir, "filter_id_set.json")
        corpus_path = os.path.join(build_dir, "filter_corpus.json")

        # Find the actual JSON schema file
        graph_schema = os.path.join(build_dir, "filter_registry_snapshot.json")
        if not os.path.exists(graph_schema):
            graph_schema = schema_path

        try:
            validator = GraphValidator(
                schema_path=graph_schema,
                filter_id_set_path=filter_ids_path,
                corpus_path=corpus_path,
            )
            result = validator.validate_all(graph_json)
            if result.valid:
                return json.dumps({"valid": True, "message": "Graph is valid — all checks passed."})
            return json.dumps({"valid": False, "errors": result.errors})
        except Exception as exc:
            return json.dumps({"valid": False, "errors": [f"Validation error: {exc}"]})
