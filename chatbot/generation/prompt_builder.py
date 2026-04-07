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
            "You are Ambara Graph Assistant.\n\n"
            "=== TASK ===\n"
            "Produce a valid SerializedGraph JSON from the user's request.\n\n"
            "=== CHAIN-OF-THOUGHT ===\n"
            "Before generating JSON, reason through:\n"
            "  1. What processing steps does the user need?\n"
            "  2. Which filters from the allowed list accomplish each step?\n"
            "  3. How should filters be wired? (linear chain, batch, or branching DAG)\n"
            "  4. What parameter values did the user specify? Use defaults otherwise.\n"
            "  5. Verify: every filter_id exists in the allowed list, every port name\n"
            "     matches the filter metadata, and every connection is type-compatible.\n\n"
            "=== FORMAT RULES ===\n"
            "- Output ONLY valid JSON matching the SerializedGraph schema.\n"
            "- No markdown fences, no explanatory text.\n"
            "- Use only filter IDs from the supplied allowed list.\n"
            "- Always include load_image and save_image unless extending a partial graph.\n"
            "- Use exact port names from filter metadata (do not invent names).\n"
            "- For batch/folder requests, use a batch-only chain:\n"
            "  load_folder → batch_* transforms → batch_save_images.\n"
            "  Do NOT mix single-image nodes into a batch chain.\n\n"
            "=== BRANCHING ===\n"
            "Graphs are DAGs and MAY branch: a node can have multiple outgoing edges,\n"
            "and merge nodes may have multiple inputs. If the request implies compositing,\n"
            "blending, or comparing images, generate a multi-branch subgraph.\n\n"
            "=== COMMON MISTAKES TO AVOID ===\n"
            "- Inventing port names not in the filter metadata.\n"
            "- Using single-image filters in batch chains.\n"
            "- Forgetting input/output bookend nodes.\n"
            "- Creating cycles (the graph must be a DAG).\n\n"
            f"Schema: {json.dumps(self.graph_schema)}"
        )

        user_lines = [
            f"User request: {query}",
            f"Allowed filter IDs: {filter_ids}",
            f"Retrieved filters: {json.dumps(filters)}",
            f"Examples: {json.dumps(examples)}",
            (
                "Branching guidance: use connections list to represent branches. Example pattern: "
                "A->C(in1) and B->C(in2), then C->D."
            ),
        ]
        if partial_graph is not None:
            user_lines.append(f"Partial graph to extend: {json.dumps(partial_graph)}")

        return {
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": "\n".join(user_lines)},
            ]
        }
