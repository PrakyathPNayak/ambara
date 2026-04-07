"""Stage 2 – Select: for each planned step, choose the best filter and parameters.

Uses compact "filter card" representations and focused LLM calls to make
reliable filter selection even with small models.
"""

from __future__ import annotations

import json
import logging
import re
from typing import Any

LOGGER = logging.getLogger(__name__)


def _format_filter_card(doc: dict[str, Any]) -> str:
    """Format a filter corpus entry as a compact human-readable card.

    Example output::

        gaussian_blur — Apply Gaussian blur to smooth image
          inputs:  image(Image)
          outputs: image(Image)
          params:  sigma(Float, default=2.5), kernel_size(Integer, default=3)
    """
    fid = doc.get("id", "?")
    desc = doc.get("description", "")
    inputs = ", ".join(
        f"{p.get('name', '?')}({p.get('type', 'Any')})"
        for p in doc.get("input_ports", [])
    ) or "none"
    outputs = ", ".join(
        f"{p.get('name', '?')}({p.get('type', 'Any')})"
        for p in doc.get("output_ports", [])
    ) or "none"

    param_parts = []
    for p in doc.get("parameters", []):
        name = p.get("name", "?")
        ptype = p.get("type", "Any")
        default = p.get("default", "")
        param_parts.append(f"{name}({ptype}, default={default})")
    params = ", ".join(param_parts) or "none"

    return f"{fid} — {desc}\n  inputs:  {inputs}\n  outputs: {outputs}\n  params:  {params}"


SELECT_EXAMPLES = [
    {
        "step": {"description": "Apply Gaussian blur to smooth the image", "operation": "gaussian_blur"},
        "candidates": [
            "gaussian_blur — Apply Gaussian blur\n  inputs: image(Image)\n  outputs: image(Image)\n  params: sigma(Float, default=2.5)",
            "box_blur — Apply box blur\n  inputs: image(Image)\n  outputs: image(Image)\n  params: radius(Integer, default=2)",
        ],
        "selection": {"filter_id": "gaussian_blur", "parameters": {"sigma": 2.5}},
    },
    {
        "step": {"description": "Resize all images to 512x512", "operation": "batch_resize"},
        "candidates": [
            "batch_resize — Resize batch\n  inputs: images(Array(Image))\n  outputs: images(Array(Image))\n  params: width(Integer, default=256), height(Integer, default=256)",
            "resize — Resize single image\n  inputs: image(Image)\n  outputs: image(Image)\n  params: width(Integer, default=256), height(Integer, default=256)",
        ],
        "selection": {"filter_id": "batch_resize", "parameters": {"width": 512, "height": 512}},
    },
]


def build_select_prompt(
    step: dict[str, Any],
    candidate_cards: list[str],
    query: str,
) -> dict[str, list[dict[str, str]]]:
    """Build prompt for filter selection.

    Args:
        step: A single planned step dict with operation and description.
        candidate_cards: Formatted filter card strings for candidates.
        query: Original user query for context.

    Returns:
        Messages dict for LLM call.
    """
    examples_text = "\n\n".join(
        f"Step: {json.dumps(ex['step'])}\nCandidates:\n"
        + "\n".join(f"  [{i+1}] {c}" for i, c in enumerate(ex["candidates"]))
        + f"\nSelection: {json.dumps(ex['selection'])}"
        for ex in SELECT_EXAMPLES
    )

    system = (
        "You are a filter selector for Ambara image processing pipelines.\n\n"
        "=== TASK ===\n"
        "Given a processing step and candidate filters, select the BEST match\n"
        "and set appropriate parameter values.\n\n"
        "=== CHAIN-OF-THOUGHT ===\n"
        "Before selecting, reason through:\n"
        "  1. MATCH: Which candidate's description best aligns with the step's intent?\n"
        "  2. PORTS: Does the candidate's input/output type fit the surrounding pipeline?\n"
        "  3. PARAMS: Does the user mention specific numeric values (e.g. '512x512',\n"
        "     '50% opacity', '90 degrees')? Extract and map them to parameter names.\n"
        "     If no specific value is mentioned, use the filter's defaults.\n"
        "  4. VERIFY: Is the filter_id spelled exactly as shown in the candidate list?\n\n"
        "=== RULES ===\n"
        "1. Select exactly ONE filter from the candidates.\n"
        "2. Use the filter's EXACT id as filter_id (copy-paste, do not retype).\n"
        "3. Set parameter values appropriate for the user's request; use defaults otherwise.\n"
        "4. Output ONLY valid JSON: {\"filter_id\": \"...\", \"parameters\": {...}}\n"
        "5. No markdown fences, no extra text.\n\n"
        "=== COMMON MISTAKES TO AVOID ===\n"
        "- Selecting a single-image filter when the pipeline is batch (look for batch_ prefix).\n"
        "- Misspelling the filter_id (copy it exactly from the candidate list).\n"
        "- Omitting required parameters or inventing parameter names not in the candidate.\n\n"
        f"=== EXAMPLES ===\n{examples_text}"
    )

    candidates_text = "\n".join(
        f"  [{i+1}] {card}" for i, card in enumerate(candidate_cards)
    )

    user = (
        f'Original request: "{query}"\n'
        f"Step to fulfill: {json.dumps(step)}\n"
        f"Candidate filters:\n{candidates_text}\n\n"
        "Select the best filter and set parameters:"
    )

    return {
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ]
    }


def parse_selection(raw: str) -> dict[str, Any] | None:
    """Parse filter selection from LLM output.

    Args:
        raw: Raw LLM text.

    Returns:
        Dict with filter_id and parameters, or None on failure.
    """
    text = raw.strip()

    # Strip qwen3 <think> reasoning tags.
    text = re.sub(r"<think>.*?</think>", "", text, flags=re.DOTALL).strip()

    # Strip markdown fences.
    fence_match = re.search(r"```(?:json)?\s*\n?(.*?)```", text, re.DOTALL)
    if fence_match:
        text = fence_match.group(1).strip()

    brace_start = text.find("{")
    if brace_start > 0:
        text = text[brace_start:]
    brace_end = text.rfind("}")
    if brace_end >= 0:
        text = text[: brace_end + 1]

    try:
        sel = json.loads(text)
    except json.JSONDecodeError:
        LOGGER.warning("Failed to parse selection JSON: %s", text[:200])
        return None

    if not isinstance(sel, dict) or "filter_id" not in sel:
        return None

    sel.setdefault("parameters", {})
    return sel


def format_filter_card(doc: dict[str, Any]) -> str:
    """Public wrapper for filter card formatting.

    Args:
        doc: Filter corpus entry.

    Returns:
        Compact card string.
    """
    return _format_filter_card(doc)
