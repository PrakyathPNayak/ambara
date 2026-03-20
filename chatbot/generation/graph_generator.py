"""Multi-stage agentic pipeline for Ambara graph generation.

Architecture (based on HuggingGPT + ReAct patterns):

  Stage 1 - PLAN:    Decompose user query into ordered processing steps.
  Stage 2 - SELECT:  For each step, choose the best filter and parameters.
  Stage 3 - CONNECT: Deterministically wire filters into a valid graph.
  Stage 4 - VALIDATE + REPAIR: Run validation and fix any remaining issues.

This replaces the old single-shot approach where one LLM call had to produce
the entire graph JSON - a task too complex for smaller models like qwen3:8b.
"""

from __future__ import annotations

import json
import logging
import re
from pathlib import Path
from typing import Any

from chatbot.generation.connector import build_graph
from chatbot.generation.graph_validator import GraphValidator
from chatbot.generation.llm_client import LLMClient
from chatbot.generation.planner import build_plan_prompt, parse_plan
from chatbot.generation.repair_prompt_builder import RepairPromptBuilder
from chatbot.generation.selector import build_select_prompt, format_filter_card, parse_selection
from chatbot.models import GenerationResultModel
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


class GraphGenerator:
    """Generate validated SerializedGraph JSON from natural language via
    a multi-stage agentic pipeline.

    Stages:
        1. Plan - decompose query into steps (LLM)
        2. Select - pick filter + params per step (retrieval + LLM)
        3. Connect - wire graph deterministically (code)
        4. Validate + repair (validator + LLM if needed)
    """

    def __init__(
        self,
        chroma_path: str,
        corpus_path: str,
        examples_path: str,
        force_mock_llm: bool = False,
        llm_client: LLMClient | None = None,
    ) -> None:
        self.filter_retriever = FilterRetriever(chroma_path, corpus_path)
        self.repair_builder = RepairPromptBuilder()
        self.llm_client = llm_client or LLMClient(force_mock=force_mock_llm)
        self.validator = GraphValidator(
            str(ROOT / "chatbot" / "corpus" / "graph_schema.json"),
            str(ROOT / "build" / "filter_id_set.json"),
            corpus_path,
        )
        self.corpus_by_id: dict[str, dict[str, Any]] = dict(self.filter_retriever.by_id)

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def generate(self, query: str, partial_graph: dict[str, Any] | None = None) -> GenerationResultModel:
        """Run the full agentic pipeline: Plan -> Select -> Connect -> Validate."""
        backend = getattr(self.llm_client, "backend", "custom")

        if backend == "mock":
            return self._generate_mock(query)

        try:
            return self._generate_agentic(query, partial_graph)
        except Exception as exc:
            LOGGER.exception("Agentic pipeline failed: %s", exc)
            return GenerationResultModel(
                graph=None,
                valid=False,
                errors=[f"Pipeline error: {exc}"],
                retries=0,
                retrieved_filters=[],
                llm_response_raw="",
                explanation="The agentic pipeline encountered an unexpected error.",
            )

    # ------------------------------------------------------------------
    # Stage 1: Plan
    # ------------------------------------------------------------------

    def _plan(self, query: str) -> dict[str, Any] | None:
        """Ask the LLM to decompose the query into processing steps."""
        prompt = build_plan_prompt(query)
        try:
            raw = self.llm_client.generate(prompt, temperature=0.0)
        except RuntimeError as err:
            LOGGER.error("Plan stage LLM call failed: %s", err)
            return None
        plan = parse_plan(raw)
        if plan:
            LOGGER.info(
                "Plan: topology=%s, steps=%d - %s",
                plan.get("topology"),
                len(plan.get("steps", [])),
                plan.get("reasoning", "")[:120],
            )
        return plan

    # ------------------------------------------------------------------
    # Stage 2: Select
    # ------------------------------------------------------------------

    def _select(self, plan: dict[str, Any], query: str) -> list[dict[str, Any]]:
        """For each planned step, select the best filter and set parameters."""
        selections: list[dict[str, Any]] = []

        for step in plan.get("steps", []):
            operation = step.get("operation", "")
            description = step.get("description", "")

            # If the plan already specifies a valid filter_id, use it directly.
            if operation in self.corpus_by_id:
                meta = self.corpus_by_id[operation]
                params = self._select_params_for_known_filter(
                    operation, meta, step, query
                )
                selections.append({
                    "filter_id": operation,
                    "parameters": params,
                })
                continue

            # Retrieve candidates via semantic search.
            search_query = f"{operation} {description}".strip()
            candidates = self.filter_retriever.retrieve(search_query, top_k=5)
            if not candidates:
                LOGGER.warning("No candidates found for step: %s", step)
                continue

            # Format candidates as compact cards.
            cards = [format_filter_card(c) for c in candidates]

            # Ask LLM to select.
            sel_prompt = build_select_prompt(step, cards, query)
            try:
                raw = self.llm_client.generate(sel_prompt, temperature=0.0)
            except RuntimeError as err:
                LOGGER.error("Select stage LLM call failed for step %s: %s", step, err)
                selections.append({
                    "filter_id": candidates[0]["id"],
                    "parameters": _default_params(candidates[0]),
                })
                continue

            sel = parse_selection(raw)
            if sel and sel["filter_id"] in self.corpus_by_id:
                selections.append(sel)
            else:
                LOGGER.warning(
                    "Selection parse failed or invalid filter_id, using first candidate: %s",
                    candidates[0]["id"],
                )
                selections.append({
                    "filter_id": candidates[0]["id"],
                    "parameters": _default_params(candidates[0]),
                })

        return selections

    def _select_params_for_known_filter(
        self,
        filter_id: str,
        meta: dict[str, Any],
        step: dict[str, Any],
        query: str,
    ) -> dict[str, Any]:
        """When the filter is already known, infer parameter values.

        Uses simple pattern matching for common patterns (dimensions, opacity,
        angles). Falls back to defaults when no specific values are mentioned.
        Only uses LLM as a last resort.
        """
        params = meta.get("parameters", [])
        if not params:
            return {}

        defaults = _default_params(meta)

        # Try to extract numeric values from query for common parameter patterns.
        inferred = dict(defaults)
        q = query.lower()
        desc = step.get("description", "").lower()
        context = f"{q} {desc}"

        # Dimensions: "512x512", "256 x 256", "width 800", "height 600"
        dim_match = re.search(r'(\d+)\s*[xX×]\s*(\d+)', context)
        if dim_match and "width" in inferred and "height" in inferred:
            inferred["width"] = int(dim_match.group(1))
            inferred["height"] = int(dim_match.group(2))
            return inferred

        # Opacity: "50% opacity", "opacity 0.7"
        opacity_match = re.search(r'(\d+)%\s*opacity|opacity\s*(\d*\.?\d+)', context)
        if opacity_match and "opacity" in inferred:
            val = opacity_match.group(1) or opacity_match.group(2)
            inferred["opacity"] = float(val) / 100.0 if opacity_match.group(1) else float(val)
            return inferred

        # Angle: "90 degrees", "rotate 45"
        angle_match = re.search(r'(\d+)\s*(?:degree|°)', context)
        if angle_match and "angle" in inferred:
            inferred["angle"] = float(angle_match.group(1))
            return inferred

        # Signed values: "brightness +20", "contrast -10"
        for pname in list(inferred.keys()):
            val_match = re.search(rf'{pname}\s*[+\-]?\s*(\d+\.?\d*)', context)
            if val_match:
                inferred[pname] = float(val_match.group(1))

        return inferred

    # ------------------------------------------------------------------
    # Stage 3: Connect (deterministic)
    # ------------------------------------------------------------------

    def _connect(
        self,
        selections: list[dict[str, Any]],
        plan: dict[str, Any],
        query: str,
    ) -> dict[str, Any]:
        """Wire selected filters into a complete graph."""
        return build_graph(selections, plan, self.corpus_by_id, query)

    # ------------------------------------------------------------------
    # Stage 4: Validate + Repair
    # ------------------------------------------------------------------

    def _validate_and_repair(
        self, graph: dict[str, Any], query: str
    ) -> tuple[dict[str, Any] | None, bool, list[str], int]:
        """Validate and optionally repair the graph.

        Returns (graph, valid, errors, retries).
        """
        graph_json = json.dumps(graph)
        validation = self.validator.validate_all(graph_json)

        if validation.valid:
            return graph, True, [], 0

        # Up to 2 repair attempts.
        for attempt in range(1, 3):
            LOGGER.warning(
                "Validation failed (attempt %d), errors: %s", attempt, validation.errors
            )
            repair_prompt = self.repair_builder.build(query, graph_json, validation.errors)
            try:
                raw = self.llm_client.generate(repair_prompt, temperature=0.0)
            except RuntimeError as err:
                return None, False, [*validation.errors, str(err)], attempt

            try:
                repaired = json.loads(self._clean_json(raw))
            except json.JSONDecodeError:
                continue

            graph_json = json.dumps(repaired)
            validation = self.validator.validate_all(graph_json)
            if validation.valid:
                return repaired, True, [], attempt

        return None, False, validation.errors, 2

    # ------------------------------------------------------------------
    # Full agentic pipeline
    # ------------------------------------------------------------------

    def _generate_agentic(
        self, query: str, partial_graph: dict[str, Any] | None = None
    ) -> GenerationResultModel:
        """Execute the multi-stage agentic pipeline."""

        # Stage 1: Plan.
        plan = self._plan(query)
        if not plan:
            return self._fallback_generate(query, "Planning stage failed to produce a valid plan.")

        # Stage 2: Select.
        selections = self._select(plan, query)
        if not selections:
            return self._fallback_generate(query, "Selection stage found no valid filters.")

        # Ensure selections contain valid filter IDs.
        valid_selections = [
            s for s in selections if s.get("filter_id") in self.corpus_by_id
        ]
        if not valid_selections:
            return self._fallback_generate(query, "No selected filters matched the registry.")

        retrieved_ids = [s["filter_id"] for s in valid_selections]
        LOGGER.info("Selected filters: %s", retrieved_ids)

        # Stage 3: Connect.
        graph = self._connect(valid_selections, plan, query)

        # Stage 4: Validate + Repair.
        final_graph, valid, errors, retries = self._validate_and_repair(graph, query)

        if valid and final_graph:
            node_count = len(final_graph.get("nodes", []))
            conn_count = len(final_graph.get("connections", []))
            filter_names = ", ".join(dict.fromkeys(retrieved_ids[:5]))
            explanation = (
                f"I built a {node_count}-node pipeline using {filter_names} "
                f"with {conn_count} connections. "
                f"Click 'Insert Graph' to load it into the canvas."
            )
            return GenerationResultModel(
                graph=final_graph,
                valid=True,
                errors=[],
                retries=retries,
                retrieved_filters=retrieved_ids,
                llm_response_raw=json.dumps(final_graph),
                explanation=explanation,
            )

        return GenerationResultModel(
            graph=None,
            valid=False,
            errors=errors,
            retries=retries,
            retrieved_filters=retrieved_ids,
            llm_response_raw=json.dumps(graph) if graph else "",
            explanation="Unable to generate a valid graph after repair attempts.",
        )

    # ------------------------------------------------------------------
    # Fallback: keyword-based deterministic generation
    # ------------------------------------------------------------------

    def _fallback_generate(self, query: str, reason: str) -> GenerationResultModel:
        """Build a deterministic graph when the LLM pipeline fails."""
        LOGGER.warning("Falling back to deterministic generation: %s", reason)

        q = query.lower()
        is_batch = any(k in q for k in ("batch", "folder", "multiple images", "all images"))
        is_branch = any(k in q for k in ("blend", "overlay", "merge", "composite", "two images"))

        if is_batch:
            selections = self._deterministic_batch_selections(q)
            plan = {"topology": "batch", "steps": []}
        elif is_branch:
            selections = self._deterministic_branch_selections(q)
            plan = {"topology": "branch", "steps": []}
        else:
            selections = self._deterministic_linear_selections(q)
            plan = {"topology": "linear", "steps": []}

        graph = self._connect(selections, plan, query)
        graph_json = json.dumps(graph)
        validation = self.validator.validate_all(graph_json)

        retrieved_ids = [s["filter_id"] for s in selections]
        return GenerationResultModel(
            graph=graph if validation.valid else None,
            valid=validation.valid,
            errors=validation.errors,
            retries=0,
            retrieved_filters=retrieved_ids,
            llm_response_raw=graph_json,
            explanation=f"Fallback deterministic generation ({reason}). " + (
                f"Built {len(selections)}-node pipeline."
                if validation.valid
                else f"Validation errors: {validation.errors}"
            ),
        )

    def _deterministic_linear_selections(self, q: str) -> list[dict[str, Any]]:
        """Infer a linear pipeline from keywords."""
        # Astro/stacking pipelines need load_folder for multiple frames.
        needs_folder = any(k in q for k in ("stack", "astro", "light frame", "calibrat"))
        if needs_folder:
            selections: list[dict[str, Any]] = [
                {"filter_id": "load_folder", "parameters": _default_params(self.corpus_by_id.get("load_folder", {}))},
            ]
        else:
            selections: list[dict[str, Any]] = [
                {"filter_id": "load_image", "parameters": {"path": ""}},
            ]

        keyword_filters = [
            ("blur", "gaussian_blur"), ("smooth", "gaussian_blur"),
            ("bright", "brightness"), ("dark", "brightness"),
            ("contrast", "contrast"), ("satur", "saturation"),
            ("gray", "grayscale"), ("grey", "grayscale"),
            ("invert", "invert"), ("negative", "invert"),
            ("resiz", "resize"), ("scale", "resize"),
            ("rotate", "rotate"), ("flip", "flip"), ("crop", "crop"),
            ("astro", "histogram_stretch"), ("stretch", "histogram_stretch"),
            ("stack", "image_stack"), ("hot pixel", "hot_pixel_removal"),
            ("dark frame", "dark_frame_subtract"), ("flat field", "flat_field_correct"),
            ("preview", "preview"), ("info", "image_info"),
        ]

        added = set()
        matches: list[tuple[int, str]] = []
        for keyword, fid in keyword_filters:
            pos = q.find(keyword)
            if pos >= 0 and fid not in added and fid in self.corpus_by_id:
                matches.append((pos, fid))
                added.add(fid)
        matches.sort()

        for _, fid in matches:
            meta = self.corpus_by_id[fid]
            selections.append({
                "filter_id": fid,
                "parameters": _default_params(meta),
            })

        if len(selections) == 1:
            selections.append({
                "filter_id": "gaussian_blur",
                "parameters": _default_params(self.corpus_by_id.get("gaussian_blur", {})),
            })

        selections.append({
            "filter_id": "save_image",
            "parameters": _default_params(self.corpus_by_id.get("save_image", {})),
        })
        return selections

    def _deterministic_batch_selections(self, q: str) -> list[dict[str, Any]]:
        """Infer a batch pipeline from keywords."""
        selections: list[dict[str, Any]] = [
            {"filter_id": "load_folder", "parameters": _default_params(self.corpus_by_id.get("load_folder", {}))},
        ]

        keyword_filters = [
            ("blur", "batch_gaussian_blur"), ("smooth", "batch_gaussian_blur"),
            ("bright", "batch_brightness"), ("contrast", "batch_contrast"),
            ("satur", "batch_saturation"), ("gray", "batch_grayscale"),
            ("grey", "batch_grayscale"), ("invert", "batch_invert"),
            ("resiz", "batch_resize"), ("scale", "batch_resize"),
            ("rotate", "batch_rotate"), ("flip", "batch_flip"), ("crop", "batch_crop"),
        ]

        added = set()
        matches: list[tuple[int, str]] = []
        for keyword, fid in keyword_filters:
            pos = q.find(keyword)
            if pos >= 0 and fid not in added and fid in self.corpus_by_id:
                matches.append((pos, fid))
                added.add(fid)
        matches.sort()

        for _, fid in matches:
            selections.append({
                "filter_id": fid,
                "parameters": _default_params(self.corpus_by_id.get(fid, {})),
            })

        if len(selections) == 1:
            selections.append({
                "filter_id": "batch_gaussian_blur",
                "parameters": _default_params(self.corpus_by_id.get("batch_gaussian_blur", {})),
            })

        selections.append({
            "filter_id": "batch_save_images",
            "parameters": _default_params(self.corpus_by_id.get("batch_save_images", {})),
        })
        return selections

    def _deterministic_branch_selections(self, q: str) -> list[dict[str, Any]]:
        """Infer a branch/merge pipeline from keywords."""
        return [
            {"filter_id": "load_image", "parameters": {"path": ""}},
            {"filter_id": "load_image", "parameters": {"path": ""}},
            {"filter_id": "blend", "parameters": _default_params(self.corpus_by_id.get("blend", {}))},
            {"filter_id": "save_image", "parameters": _default_params(self.corpus_by_id.get("save_image", {}))},
        ]

    # ------------------------------------------------------------------
    # Mock mode
    # ------------------------------------------------------------------

    def _generate_mock(self, query: str) -> GenerationResultModel:
        """Generate a deterministic graph in mock mode (no LLM)."""
        q = query.lower()
        if any(k in q for k in ("batch", "folder", "multiple")):
            sels = self._deterministic_batch_selections(q)
            plan = {"topology": "batch", "steps": []}
        elif any(k in q for k in ("blend", "overlay", "merge", "composite")):
            sels = self._deterministic_branch_selections(q)
            plan = {"topology": "branch", "steps": []}
        else:
            sels = self._deterministic_linear_selections(q)
            plan = {"topology": "linear", "steps": []}

        graph = self._connect(sels, plan, query)
        graph_json = json.dumps(graph)
        validation = self.validator.validate_all(graph_json)
        ids = [s["filter_id"] for s in sels]

        return GenerationResultModel(
            graph=graph if validation.valid else None,
            valid=validation.valid,
            errors=validation.errors,
            retries=0,
            retrieved_filters=ids,
            llm_response_raw=graph_json,
            explanation=f"Mock-generated {len(sels)}-node {plan['topology']} pipeline.",
        )

    # ------------------------------------------------------------------
    # Helpers
    # ------------------------------------------------------------------

    @staticmethod
    def _clean_json(text: str) -> str:
        """Strip markdown fences and trailing text from LLM JSON output."""
        text = text.strip()
        fence = re.search(r"```(?:json)?\s*\n?(.*?)```", text, re.DOTALL)
        if fence:
            text = fence.group(1).strip()
        brace = text.find("{")
        if brace > 0:
            text = text[brace:]
        end = text.rfind("}")
        if end >= 0:
            text = text[: end + 1]
        return text
