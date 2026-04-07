"""Repair prompt builder for graph self-healing retries."""

from __future__ import annotations

import json
import re
from typing import Any


class RepairPromptBuilder:
    """Create prompts that target specific graph validation errors."""

    def build(
        self,
        query: str,
        failed_graph: str,
        errors: list[str],
        corpus_by_id: dict[str, Any] | None = None,
    ) -> dict:
        """Build a repair prompt with full original graph payload.

        Args:
            query: Original user request.
            failed_graph: Invalid graph JSON string.
            errors: Validation errors to fix.
            corpus_by_id: Optional filter metadata dict keyed by filter_id.

        Returns:
            Prompt dictionary with messages list.

        Raises:
            ValueError: If failed_graph is empty.
        """
        if not failed_graph.strip():
            raise ValueError("failed_graph must not be empty")

        error_list = "\n".join(f"  {i+1}. {e}" for i, e in enumerate(errors))

        # Extract affected filter IDs from graph and errors
        metadata_section = ""
        if corpus_by_id:
            affected_ids = self._extract_affected_filter_ids(failed_graph, errors)
            cards = []
            for fid in sorted(affected_ids):
                meta = corpus_by_id.get(fid)
                if meta:
                    cards.append(self._format_compact_card(fid, meta))
            if cards:
                metadata_section = (
                    "=== VALID FILTER METADATA FOR AFFECTED NODES ===\n"
                    + "\n".join(cards)
                    + "\n\n"
                )

        user_content = (
            f"=== ORIGINAL USER REQUEST ===\n{query}\n\n"
            f"=== VALIDATION ERRORS ({len(errors)}) ===\n{error_list}\n\n"
            f"{metadata_section}"
            f"=== ORIGINAL GRAPH JSON ===\n{failed_graph}\n\n"
            "=== INSTRUCTIONS ===\n"
            "Analyze each error and fix it in the JSON. Use this process:\n"
            "  1. For each error, identify the exact node or connection that is wrong.\n"
            "  2. Determine the minimal change that corrects the error (rename a port,\n"
            "     fix a filter_id, add a missing connection, etc.).\n"
            "  3. After all fixes, mentally re-validate: are all filter_ids valid?\n"
            "     Do all from_port/to_port names match the metadata above?\n"
            "     Are connected port types compatible?\n"
            "  4. Return ONLY the corrected JSON. No explanation, no markdown fences."
        )
        return {
            "messages": [
                {
                    "role": "system",
                    "content": (
                        "You are a strict JSON graph repair assistant for Ambara.\n"
                        "You receive a graph with validation errors and must return\n"
                        "a corrected version. Apply ONLY minimal fixes — do not add\n"
                        "or remove nodes unless required by an error. Preserve the\n"
                        "user's intended pipeline structure. Output raw JSON only."
                    ),
                },
                {"role": "user", "content": user_content},
            ]
        }

    @staticmethod
    def _extract_affected_filter_ids(graph_json: str, errors: list[str]) -> set[str]:
        """Extract filter IDs referenced in the graph that relate to errors."""
        ids: set[str] = set()
        try:
            data = json.loads(graph_json)
            for node in data.get("nodes", []):
                fid = node.get("filter_id")
                if fid:
                    ids.add(fid)
        except (json.JSONDecodeError, TypeError):
            pass
        # Also extract any filter_id mentioned directly in error messages
        for err in errors:
            match = re.search(r"filter_id\s+(\S+)", err)
            if match:
                ids.add(match.group(1))
        return ids

    @staticmethod
    def _format_compact_card(fid: str, meta: dict[str, Any]) -> str:
        """Format a filter's metadata as a compact reference card."""
        name = meta.get("name", fid)
        inputs = meta.get("input_ports", meta.get("inputs", []))
        outputs = meta.get("output_ports", meta.get("outputs", []))
        in_str = ", ".join(
            f"{p.get('name')}:{p.get('type', '?')}" for p in inputs if isinstance(p, dict)
        )
        out_str = ", ".join(
            f"{p.get('name')}:{p.get('type', '?')}" for p in outputs if isinstance(p, dict)
        )
        return f"  [{fid}] {name} | inputs: {in_str or 'none'} | outputs: {out_str or 'none'}"
