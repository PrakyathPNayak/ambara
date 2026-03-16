"""Repair prompt builder for graph self-healing retries."""

from __future__ import annotations

import json


class RepairPromptBuilder:
    """Create prompts that target specific graph validation errors."""

    def build(self, query: str, failed_graph: str, errors: list[str]) -> dict:
        """Build a repair prompt with full original graph payload.

        Args:
            query: Original user request.
            failed_graph: Invalid graph JSON string.
            errors: Validation errors to fix.

        Returns:
            Prompt dictionary with messages list.

        Raises:
            ValueError: If failed_graph is empty.
        """
        if not failed_graph.strip():
            raise ValueError("failed_graph must not be empty")

        user_content = {
            "instruction": "Fix exactly the listed validation issues and return corrected JSON only.",
            "query": query,
            "errors": errors,
            "original_graph": failed_graph,
        }
        return {
            "messages": [
                {
                    "role": "system",
                    "content": "You are a strict JSON graph repair assistant. Return only corrected JSON.",
                },
                {"role": "user", "content": json.dumps(user_content)},
            ]
        }
