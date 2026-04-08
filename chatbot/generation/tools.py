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
    {
        "name": "set_input_image",
        "description": (
            "Set the input image path for the current pipeline. Use this when "
            "the user provides or references an image file they want to process. "
            "The path will be used as the load_image node's path parameter."
        ),
        "parameters": {
            "path": {"type": "string", "description": "Absolute or relative path to the image file."},
        },
        "required": ["path"],
    },
    {
        "name": "execute_pipeline",
        "description": (
            "Execute a generated graph pipeline through the Ambara processing engine. "
            "Use this AFTER generate_graph + validate_graph to actually run the pipeline "
            "and produce output images. Returns execution status and output paths."
        ),
        "parameters": {
            "graph_json": {"type": "string", "description": "The graph JSON to execute."},
        },
        "required": ["graph_json"],
    },
    {
        "name": "describe_image",
        "description": (
            "Read an image file and return its metadata: dimensions, format, color mode, "
            "file size, and basic statistics. Use this when the user asks you to describe, "
            "analyze, or inspect an attached image."
        ),
        "parameters": {
            "path": {"type": "string", "description": "Absolute path to the image file."},
        },
        "required": ["path"],
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
            "set_input_image": self._set_input_image,
            "execute_pipeline": self._execute_pipeline,
            "describe_image": self._describe_image,
        }
        self._input_image_path: str | None = None

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
        graph = result.graph
        # Inject stored input image path into load_image nodes
        if self._input_image_path and graph and "nodes" in graph:
            for node in graph["nodes"]:
                if node.get("filter_id") == "load_image":
                    params = node.setdefault("parameters", {})
                    if not params.get("path") or params["path"] in ("input.png", "image.png", ""):
                        params["path"] = self._input_image_path
        return json.dumps({
            "graph": graph,
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

    def _set_input_image(self, path: str) -> str:
        """Store input image path for use in generated graphs."""
        import os
        # Validate path exists or is a plausible path
        expanded = os.path.expanduser(path)
        if os.path.isfile(expanded):
            self._input_image_path = expanded
            return json.dumps({
                "success": True,
                "path": expanded,
                "message": f"Input image set to '{expanded}'. This will be used as the load_image path in generated graphs.",
            })
        # Accept the path anyway — it might be set before the image exists
        self._input_image_path = expanded
        return json.dumps({
            "success": True,
            "path": expanded,
            "warning": f"Path '{expanded}' does not exist yet, but it will be used as the load_image path.",
        })

    def _execute_pipeline(self, graph_json: str) -> str:
        """Execute a graph pipeline through Ambara's processing engine."""
        import os
        import subprocess
        import tempfile
        from pathlib import Path

        root = Path(__file__).resolve().parents[2]

        try:
            graph = json.loads(graph_json)
        except json.JSONDecodeError:
            return json.dumps({"success": False, "errors": ["Invalid graph JSON."]})

        # Write graph to temp file
        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as fp:
            path = Path(fp.name)
            fp.write(json.dumps(graph))

        try:
            cmd = ["cargo", "run", "--release", "--", "load-graph", str(path), "--execute"]
            proc = subprocess.run(
                cmd, cwd=root, capture_output=True, text=True,
                check=False, timeout=120,
            )
        except subprocess.TimeoutExpired:
            return json.dumps({
                "success": False,
                "errors": ["Pipeline execution timed out after 120 seconds."],
            })
        finally:
            path.unlink(missing_ok=True)

        if proc.returncode != 0:
            error_msg = proc.stderr.strip() or proc.stdout.strip() or "Execution failed"
            return json.dumps({
                "success": False,
                "errors": [error_msg[:1000]],
            })

        # Parse output
        try:
            payload = json.loads(proc.stdout)
            success = bool(payload.get("success", True))
            errors = [str(e) for e in payload.get("errors", [])]
            output_paths = payload.get("output_paths", [])
        except json.JSONDecodeError:
            success = True
            errors = []
            output_paths = []

        return json.dumps({
            "success": success,
            "output_paths": output_paths,
            "errors": errors,
            "message": "Pipeline executed successfully." if success else "Pipeline execution had errors.",
        })

    def _describe_image(self, path: str) -> str:
        """Read an image file and return metadata and basic analysis."""
        import os

        expanded = os.path.expanduser(path)
        if not os.path.isfile(expanded):
            return json.dumps({"error": f"Image file not found: {expanded}"})

        try:
            from PIL import Image
            from PIL.ExifTags import TAGS

            file_size = os.path.getsize(expanded)
            img = Image.open(expanded)
            width, height = img.size

            info: dict[str, Any] = {
                "path": expanded,
                "format": img.format or os.path.splitext(expanded)[1].lstrip(".").upper(),
                "dimensions": f"{width}x{height}",
                "width": width,
                "height": height,
                "color_mode": img.mode,
                "file_size_bytes": file_size,
                "file_size_human": (
                    f"{file_size / 1024:.1f} KB" if file_size < 1024 * 1024
                    else f"{file_size / (1024 * 1024):.1f} MB"
                ),
            }

            # Extract EXIF data if available
            exif_data = {}
            try:
                raw_exif = img.getexif()
                if raw_exif:
                    for tag_id, value in raw_exif.items():
                        tag_name = TAGS.get(tag_id, str(tag_id))
                        if isinstance(value, (str, int, float)):
                            exif_data[tag_name] = value
            except Exception:
                pass
            if exif_data:
                info["exif"] = {k: v for k, v in list(exif_data.items())[:15]}

            # Basic pixel statistics
            try:
                import statistics as stat_mod
                pixels = list(img.getdata())
                if img.mode in ("RGB", "RGBA"):
                    r_vals = [p[0] for p in pixels[:10000]]
                    g_vals = [p[1] for p in pixels[:10000]]
                    b_vals = [p[2] for p in pixels[:10000]]
                    info["avg_color_rgb"] = [
                        round(stat_mod.mean(r_vals)),
                        round(stat_mod.mean(g_vals)),
                        round(stat_mod.mean(b_vals)),
                    ]
                    info["brightness"] = round(
                        stat_mod.mean(0.299 * r + 0.587 * g + 0.114 * b for r, g, b in zip(r_vals, g_vals, b_vals))
                    )
                elif img.mode == "L":
                    sample = [p for p in pixels[:10000]]
                    info["avg_brightness"] = round(stat_mod.mean(sample))
            except Exception:
                pass

            img.close()
            return json.dumps(info, indent=2)

        except Exception as exc:
            return json.dumps({"error": f"Failed to read image: {exc}"})
