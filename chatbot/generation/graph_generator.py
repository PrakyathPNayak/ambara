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


class GraphGenerator:
    """Generate validated SerializedGraph JSON from natural language requests."""

    def __init__(
        self,
        chroma_path: str,
        corpus_path: str,
        examples_path: str,
        force_mock_llm: bool = False,
    ) -> None:
        """Initialize graph generator components.

        Args:
            chroma_path: Chroma database path.
            corpus_path: Filter corpus path.
            examples_path: Few-shot examples path.
            force_mock_llm: Force deterministic mock model.

        Returns:
            None.

        Raises:
            OSError: If dependencies cannot be loaded.
        """
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
        """Run retrieval, generation, validation, and bounded self-repair.

        Args:
            query: User graph request.
            partial_graph: Optional graph to extend.

        Returns:
            Generation result model.

        Raises:
            RuntimeError: If generator cannot produce parseable JSON after retries.
        """
        retrieved_filters = self.filter_retriever.retrieve_with_graph_context(
            query=query,
            partial_graph=partial_graph or {"nodes": [], "connections": []},
            top_k=6,
        )
        examples = self.example_retriever.retrieve_examples(query, top_k=3)

        prompt = self.prompt_builder.build(query, retrieved_filters, examples, partial_graph)
        retries = 0
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
