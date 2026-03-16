"""Main orchestration pipeline for Ambara graph generation with self-repair."""

from __future__ import annotations

import json
import logging
from pathlib import Path
from typing import Any

from chatbot.generation.graph_validator import GraphValidator
from chatbot.generation.llm_client import LLMClient
from chatbot.generation.prompt_builder import GraphPromptBuilder
from chatbot.generation.repair_prompt_builder import RepairPromptBuilder
from chatbot.models import GenerationResultModel
from chatbot.retrieval.example_retriever import ExampleRetriever
from chatbot.retrieval.retriever import FilterRetriever

LOGGER = logging.getLogger(__name__)
ROOT = Path(__file__).resolve().parents[2]


def _default_params(f: dict[str, Any]) -> dict[str, Any]:
    """Build default parameter dict from filter corpus entry."""
    result: dict[str, Any] = {}
    for p in f.get("parameters", []):
        name = p.get("name")
        if not name:
            continue
        default = p.get("default", "")
        if isinstance(default, str):
            if default.startswith('String("') and default.endswith('")'):
                result[name] = default[8:-2]
            elif default in ("true", "false"):
                result[name] = default == "true"
            else:
                try:
                    result[name] = float(default)
                except (ValueError, TypeError):
                    result[name] = default
        else:
            result[name] = default
    return result


def _build_pipeline_from_filters(retrieved_filters: list[dict[str, Any]], query: str) -> str:
    """Build a valid SerializedGraph JSON from retrieved filter docs and the query."""
    def _has_port(f: dict[str, Any], port_kind: str, port_name: str) -> bool:
        return any(
            p.get("name") == port_name
            for p in f.get(f"{port_kind}_ports", [])
            if isinstance(p, dict)
        )

    input_filters = [
        f for f in retrieved_filters
        if f.get("category") == "Input" and _has_port(f, "output", "image")
    ]
    output_filters = [
        f for f in retrieved_filters
        if f.get("category") == "Output" and _has_port(f, "input", "image")
    ]
    processing_filters = [
        f for f in retrieved_filters
        if f.get("category") not in ("Input", "Output", "Utility")
        and _has_port(f, "input", "image") and _has_port(f, "output", "image")
    ]

    input_f: dict[str, Any] = input_filters[0] if input_filters else {
        "id": "load_image",
        "parameters": [{"name": "path", "default": 'String("input.png")'}],
    }
    output_f: dict[str, Any] = output_filters[0] if output_filters else {
        "id": "save_image",
        "parameters": [{"name": "path", "default": 'String("output.png")'}],
    }

    nodes: list[dict[str, Any]] = []
    connections: list[dict[str, Any]] = []
    x = 100

    nodes.append({
        "id": "n1",
        "filter_id": input_f["id"],
        "position": {"x": x, "y": 120},
        "parameters": _default_params(input_f),
    })

    for i, flt in enumerate(processing_filters[:3], 2):
        x += 220
        current_id = f"n{i}"
        prev_id = f"n{i - 1}"
        nodes.append({
            "id": current_id,
            "filter_id": flt["id"],
            "position": {"x": x, "y": 120},
            "parameters": _default_params(flt),
        })
        connections.append({
            "from_node": prev_id, "from_port": "image",
            "to_node": current_id, "to_port": "image",
        })

    x += 220
    out_id = f"n{len(nodes) + 1}"
    prev_id = f"n{len(nodes)}"
    nodes.append({
        "id": out_id,
        "filter_id": output_f["id"],
        "position": {"x": x, "y": 120},
        "parameters": _default_params(output_f),
    })
    connections.append({
        "from_node": prev_id, "from_port": "image",
        "to_node": out_id, "to_port": "image",
    })

    return json.dumps({
        "version": "1.0.0",
        "metadata": {"generatedBy": "ambara-chatbot", "query": query},
        "nodes": nodes,
        "connections": connections,
    })


class GraphGenerator:
    """Generate validated SerializedGraph JSON from natural language requests."""

    def __init__(
        self,
        chroma_path: str,
        corpus_path: str,
        examples_path: str,
        force_mock_llm: bool = False,
    ) -> None:
        """Initialize graph generator components."""
        self.filter_retriever = FilterRetriever(chroma_path, corpus_path)
        self.example_retriever = ExampleRetriever(chroma_path, examples_path)
        self.prompt_builder = GraphPromptBuilder(corpus_path)
        self.repair_builder = RepairPromptBuilder()
        self.llm_client = LLMClient(force_mock=force_mock_llm)
        self.validator = GraphValidator(
            str(ROOT / "chatbot" / "corpus" / "graph_schema.json"),
            str(ROOT / "build" / "filter_id_set.json"),
            corpus_path,
        )

    def generate(self, query: str, partial_graph: dict[str, Any] | None = None) -> GenerationResultModel:
        """Run retrieval, generation, validation, and bounded self-repair."""
        retrieved_filters = self.filter_retriever.retrieve_with_graph_context(
            query=query,
            partial_graph=partial_graph or {"nodes": [], "connections": []},
            top_k=6,
        )
        examples = self.example_retriever.retrieve_examples(query, top_k=3)
        prompt = self.prompt_builder.build(query, retrieved_filters, examples, partial_graph)
        retries = 0

        # `backend` attribute may not exist on test-injected LLM mocks; treat those as
        # real LLMs (backend="custom") so they still exercise the full call+repair path.
        _backend = getattr(self.llm_client, "backend", "custom")

        # For mock/offline Ollama build a query-aware pipeline without calling the LLM.
        # For cloud/custom LLM backends call the real LLM (self-repair loop runs below).
        if _backend in ("mock", "ollama"):
            response_text = _build_pipeline_from_filters(retrieved_filters, query)
        else:
            response_text = self.llm_client.generate(prompt, temperature=0.0)

        while retries <= 3:
            validation = self.validator.validate_all(response_text)
            if validation.valid:
                graph_obj = json.loads(response_text)
                explanation = (
                    f"Generated {len(graph_obj.get('nodes', []))} nodes and "
                    f"{len(graph_obj.get('connections', []))} connections using relevant filters."
                )
                return GenerationResultModel(
                    graph=graph_obj,
                    valid=True,
                    errors=[],
                    retries=retries,
                    retrieved_filters=[f.get("id", "") for f in retrieved_filters],
                    llm_response_raw=response_text,
                    explanation=explanation,
                )

            retries += 1
            if retries > 3:
                return GenerationResultModel(
                    graph=None,
                    valid=False,
                    errors=validation.errors,
                    retries=retries - 1,
                    retrieved_filters=[f.get("id", "") for f in retrieved_filters],
                    llm_response_raw=response_text,
                    explanation="Unable to generate a valid graph after repair attempts.",
                )

            # Non-LLM backends cannot self-repair; return immediately.
            if _backend in ("mock", "ollama"):
                return GenerationResultModel(
                    graph=None,
                    valid=False,
                    errors=validation.errors,
                    retries=retries - 1,
                    retrieved_filters=[f.get("id", "") for f in retrieved_filters],
                    llm_response_raw=response_text,
                    explanation="Unable to generate a valid graph after repair attempts.",
                )

            LOGGER.warning("Generation invalid, retry %s with errors: %s", retries, validation.errors)
            repair_prompt = self.repair_builder.build(query, response_text, validation.errors)
            response_text = self.llm_client.generate(repair_prompt, temperature=0.0)

        return GenerationResultModel(
            graph=None,
            valid=False,
            errors=["Unreachable state in generation loop"],
            retries=retries,
            retrieved_filters=[f.get("id", "") for f in retrieved_filters],
            llm_response_raw=response_text,
            explanation="Generation failed unexpectedly.",
        )
