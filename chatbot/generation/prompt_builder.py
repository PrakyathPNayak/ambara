"""Prompt construction utilities for Ambara graph generation."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
SCHEMA_PATH = ROOT / "chatbot" / "corpus" / "graph_schema.json"


class GraphPromptBuilder:
    """Builds structured chat prompts for graph generation models."""

    def __init__(self, corpus_path: str) -> None:
        """Initialize prompt builder.

        Args:
            corpus_path: Path to filter corpus JSON.

        Returns:
            None.

        Raises:
            OSError: If corpus file cannot be loaded.
        """
        self.corpus_path = Path(corpus_path)
        self.filter_docs: list[dict[str, Any]] = []
        if self.corpus_path.exists():
            self.filter_docs = json.loads(self.corpus_path.read_text())
        self.graph_schema = json.loads(SCHEMA_PATH.read_text())

    def build(
        self,
        query: str,
        filters: list[dict[str, Any]],
        examples: list[dict[str, Any]],
        partial_graph: dict[str, Any] | None = None,
    ) -> dict[str, list[dict[str, str]]]:
        """Build prompt payload with system and user messages.

        Args:
            query: User query.
            filters: Retrieved filter docs.
            examples: Few-shot examples.
            partial_graph: Optional graph to extend.

        Returns:
            OpenAI/Anthropic-like messages payload.

        Raises:
            ValueError: If query is empty.
        """
        if not query.strip():
            raise ValueError("query must not be empty")

        filter_ids = [f.get("id", "") for f in filters]
        system_prompt = (
            "You are Ambara Graph Assistant. Produce ONLY valid JSON matching the SerializedGraph schema. "
            "Do not include markdown fences. Use only filter IDs from the supplied list. "
            "Always include load_image and save_image nodes unless explicitly extending a partial graph. "
            "Set practical defaults for parameters and ensure valid connections. "
            f"Schema: {json.dumps(self.graph_schema)}"
        )

        user_lines = [
            f"User request: {query}",
            f"Allowed filter IDs: {filter_ids}",
            f"Retrieved filters: {json.dumps(filters)}",
            f"Examples: {json.dumps(examples)}",
        ]
        if partial_graph is not None:
            user_lines.append(f"Partial graph to extend: {json.dumps(partial_graph)}")

        return {
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": "\n".join(user_lines)},
            ]
        }
