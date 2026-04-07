"""DEPRECATED: Legacy single-shot graph generation pipeline.

This module has been superseded by the multi-stage agentic pipeline in
``graph_generator.py`` (Plan -> Select -> Connect -> Validate+Repair).
Kept for reference only. No code imports this module.
Safe to delete after verifying no external tools depend on it.
"""

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


def _port_defs(meta: dict[str, Any], kind: str) -> list[dict[str, Any]]:
    """Return input/output port definitions for a filter metadata record."""
    return [p for p in meta.get(f"{kind}_ports", []) if isinstance(p, dict)]


def _port_names(meta: dict[str, Any], kind: str) -> list[str]:
    return [str(p.get("name")) for p in _port_defs(meta, kind) if p.get("name")]


def _port_type_map(meta: dict[str, Any], kind: str) -> dict[str, str]:
    return {
        str(p.get("name")): str(p.get("type", "Any"))
        for p in _port_defs(meta, kind)
        if p.get("name")
    }


def _best_dst_port(src_port: str, src_meta: dict[str, Any], dst_meta: dict[str, Any]) -> str:
    """Pick best destination port by name/type compatibility."""
    dst_names = _port_names(dst_meta, "input")
    if not dst_names:
        return "image"
    if src_port in dst_names:
        return src_port

    src_types = _port_type_map(src_meta, "output")
    dst_types = _port_type_map(dst_meta, "input")
    src_type = src_types.get(src_port)

    if src_type:
        for name, dtype in dst_types.items():
            if dtype == src_type or dtype == "Any":
                return name

    for preferred in ("image", "images", "base", "overlay", "blend", "input"):
        if preferred in dst_names:
            return preferred

    return dst_names[0]


def _sanitize_graph_obj(raw: dict[str, Any], corpus_by_id: dict[str, dict[str, Any]]) -> dict[str, Any]:
    """Repair common LLM graph issues while preserving branching structure."""
    obj = dict(raw) if isinstance(raw, dict) else {}
    obj.setdefault("version", "1.0.0")
    if not isinstance(obj.get("metadata"), dict):
        obj["metadata"] = {}

    raw_nodes = obj.get("nodes") if isinstance(obj.get("nodes"), list) else []
    fixed_nodes: list[dict[str, Any]] = []

    for i, node in enumerate(raw_nodes, 1):
        if not isinstance(node, dict):
            continue
        node_id = str(node.get("id") or f"n{i}")
        filter_id = str(node.get("filter_id") or "")
        pos = node.get("position") if isinstance(node.get("position"), dict) else {}
        x = pos.get("x") if isinstance(pos.get("x"), (int, float)) else float(80 + (i - 1) * 180)
        y = pos.get("y") if isinstance(pos.get("y"), (int, float)) else 120.0
        params = node.get("parameters") if isinstance(node.get("parameters"), dict) else {}

        fixed_nodes.append(
            {
                **node,
                "id": node_id,
                "filter_id": filter_id,
                "position": {"x": x, "y": y},
                "parameters": params,
            }
        )

    obj["nodes"] = fixed_nodes
    node_by_id = {n["id"]: n for n in fixed_nodes}

    raw_connections = obj.get("connections") if isinstance(obj.get("connections"), list) else []
    fixed_connections: list[dict[str, Any]] = []
    seen: set[tuple[str, str, str, str]] = set()
    used_input_ports: dict[str, set[str]] = {}

    for conn in raw_connections:
        if not isinstance(conn, dict):
            continue
        from_node = str(conn.get("from_node") or "")
        to_node = str(conn.get("to_node") or "")
        if from_node not in node_by_id or to_node not in node_by_id:
            continue

        src_meta = corpus_by_id.get(node_by_id[from_node].get("filter_id", ""), {})
        dst_meta = corpus_by_id.get(node_by_id[to_node].get("filter_id", ""), {})

        src_names = _port_names(src_meta, "output")
        from_port = str(conn.get("from_port") or "")
        if src_names:
            if from_port not in src_names:
                from_port = "image" if "image" in src_names else src_names[0]
        else:
            from_port = from_port or "image"

        to_port = str(conn.get("to_port") or "")
        dst_names = _port_names(dst_meta, "input")
        if dst_names:
            if to_port not in dst_names:
                to_port = _best_dst_port(from_port, src_meta, dst_meta)
            # Avoid over-connecting a single input port when alternatives exist.
            used = used_input_ports.setdefault(to_node, set())
            if to_port in used:
                alternatives = [p for p in dst_names if p not in used]
                if alternatives:
                    to_port = alternatives[0]
                else:
                    # No available input slot on this destination node.
                    continue
        else:
            to_port = to_port or "image"

        edge_key = (from_node, from_port, to_node, to_port)
        if edge_key in seen:
            continue
        seen.add(edge_key)
        used_input_ports.setdefault(to_node, set()).add(to_port)

        fixed_connections.append(
            {
                **conn,
                "from_node": from_node,
                "from_port": from_port,
                "to_node": to_node,
                "to_port": to_port,
            }
        )

    obj["connections"] = fixed_connections
    return obj


def _enforce_branch_intent(
    graph: dict[str, Any], query: str, corpus_by_id: dict[str, dict[str, Any]]
) -> dict[str, Any]:
    """Inject a proper branch merge when query clearly asks for compositing."""
    q = query.lower()
    if not any(k in q for k in ("blend", "overlay", "merge", "composite", "two images", "compare")):
        return graph

    nodes = graph.get("nodes") if isinstance(graph.get("nodes"), list) else []
    conns = graph.get("connections") if isinstance(graph.get("connections"), list) else []
    if not nodes:
        return graph

    # If already has a real blend/overlay node, keep as-is.
    if any(str(n.get("filter_id")) in ("blend", "overlay") for n in nodes if isinstance(n, dict)):
        return graph

    node_ids = {str(n.get("id")) for n in nodes if isinstance(n, dict)}
    save_nodes = [n for n in nodes if isinstance(n, dict) and str(n.get("filter_id")) == "save_image"]
    load_nodes = [n for n in nodes if isinstance(n, dict) and str(n.get("filter_id")) == "load_image"]
    if len(load_nodes) < 2:
        return graph

    branch_meta = corpus_by_id.get("blend") or corpus_by_id.get("overlay")
    if not branch_meta:
        return graph
    branch_id = str(branch_meta.get("id"))

    save_node = save_nodes[0] if save_nodes else None
    if save_node is None:
        new_save_id = "save_auto"
        suffix = 1
        while new_save_id in node_ids:
            suffix += 1
            new_save_id = f"save_auto_{suffix}"
        save_node = {
            "id": new_save_id,
            "filter_id": "save_image",
            "position": {"x": 700, "y": 130},
            "parameters": _default_params(corpus_by_id.get("save_image", {})),
        }
        nodes.append(save_node)
        node_ids.add(new_save_id)

    blend_node_id = "blend_auto"
    suffix = 1
    while blend_node_id in node_ids:
        suffix += 1
        blend_node_id = f"blend_auto_{suffix}"

    blend_node = {
        "id": blend_node_id,
        "filter_id": branch_id,
        "position": {"x": 420, "y": 130},
        "parameters": _default_params(branch_meta),
    }
    nodes.append(blend_node)

    left = str(load_nodes[0].get("id"))
    right = str(load_nodes[1].get("id"))
    save_id = str(save_node.get("id"))

    # Keep existing edges but ensure a valid branch path exists.
    conns.extend(
        [
            {"from_node": left, "from_port": "image", "to_node": blend_node_id, "to_port": "base"},
            {"from_node": right, "from_port": "image", "to_node": blend_node_id, "to_port": "blend"},
            {"from_node": blend_node_id, "from_port": "image", "to_node": save_id, "to_port": "image"},
        ]
    )

    graph["nodes"] = nodes
    graph["connections"] = conns
    return graph


def _batch_ops_from_query(query: str, corpus_by_id: dict[str, dict[str, Any]]) -> list[str]:
    """Infer ordered batch operation filter IDs from query text."""
    q = query.lower()
    intents: list[tuple[int, str]] = []
    keyword_to_filter = [
        ("crop", "batch_crop"),
        ("resize", "batch_resize"),
        ("resiz", "batch_resize"),
        ("scale", "batch_resize"),
    ]
    for keyword, filter_id in keyword_to_filter:
        idx = q.find(keyword)
        if idx >= 0 and filter_id in corpus_by_id:
            intents.append((idx, filter_id))

    intents.sort(key=lambda pair: pair[0])
    ordered_ops: list[str] = []
    for _, filter_id in intents:
        if filter_id not in ordered_ops:
            ordered_ops.append(filter_id)
    return ordered_ops


def _build_batch_graph(
    query: str,
    operation_ids: list[str],
    corpus_by_id: dict[str, dict[str, Any]],
    generated_by: str,
) -> dict[str, Any] | None:
    """Build a deterministic batch pipeline graph with valid images ports."""
    load_meta = corpus_by_id.get("load_folder")
    save_meta = corpus_by_id.get("batch_save_images")
    if not load_meta or not save_meta or not operation_ids:
        return None

    nodes: list[dict[str, Any]] = [
        {
            "id": "b1",
            "filter_id": "load_folder",
            "position": {"x": 100, "y": 150},
            "parameters": _default_params(load_meta),
        }
    ]

    for idx, filter_id in enumerate(operation_ids, start=2):
        op_meta = corpus_by_id.get(filter_id)
        if not op_meta:
            continue
        nodes.append(
            {
                "id": f"b{idx}",
                "filter_id": filter_id,
                "position": {"x": 100 + (idx - 1) * 220, "y": 150},
                "parameters": _default_params(op_meta),
            }
        )

    save_id = f"b{len(nodes) + 1}"
    nodes.append(
        {
            "id": save_id,
            "filter_id": "batch_save_images",
            "position": {"x": 100 + len(nodes) * 220, "y": 150},
            "parameters": _default_params(save_meta),
        }
    )

    connections: list[dict[str, Any]] = []
    for idx in range(1, len(nodes)):
        connections.append(
            {
                "from_node": nodes[idx - 1]["id"],
                "from_port": "images",
                "to_node": nodes[idx]["id"],
                "to_port": "images",
            }
        )

    return {
        "version": "1.0.0",
        "metadata": {
            "generatedBy": generated_by,
            "query": query,
            "topology": "batch-linear",
        },
        "nodes": nodes,
        "connections": connections,
    }


def _enforce_batch_intent(
    graph: dict[str, Any], query: str, corpus_by_id: dict[str, dict[str, Any]]
) -> dict[str, Any]:
    """Force deterministic batch chain when query clearly asks for batch edits."""
    q = query.lower()
    batch_markers = ("batch", "multiple images", "all images", "folder")
    if not any(marker in q for marker in batch_markers):
        return graph

    operation_ids = _batch_ops_from_query(query, corpus_by_id)
    batch_graph = _build_batch_graph(
        query=query,
        operation_ids=operation_ids,
        corpus_by_id=corpus_by_id,
        generated_by="ambara-chatbot-batch-intent",
    )
    if batch_graph is None:
        return graph
    return batch_graph


def _build_pipeline_from_filters(
    retrieved_filters: list[dict[str, Any]], query: str, corpus_by_id: dict[str, dict[str, Any]]
) -> str:
    """Build deterministic fallback graph when running explicit mock mode.

    Supports both linear and branched templates so mock-mode tests can cover
    non-trivial graph topology.
    """

    def has_port(f: dict[str, Any], kind: str, name: str) -> bool:
        return name in _port_names(f, kind)

    input_filters = [
        f for f in retrieved_filters if f.get("category") == "Input" and has_port(f, "output", "image")
    ]
    output_filters = [
        f for f in retrieved_filters if f.get("category") == "Output" and has_port(f, "input", "image")
    ]
    processing_filters = [
        f
        for f in retrieved_filters
        if f.get("category") not in ("Input", "Output", "Utility")
        and has_port(f, "input", "image")
        and has_port(f, "output", "image")
    ]

    input_f = input_filters[0] if input_filters else corpus_by_id.get("load_image", {"id": "load_image", "parameters": []})
    output_f = output_filters[0] if output_filters else corpus_by_id.get("save_image", {"id": "save_image", "parameters": []})

    q = query.lower()
    wants_batch = any(k in q for k in ["batch", "multiple images", "all images", "folder"])
    wants_branch = any(k in q for k in ["blend", "overlay", "merge", "composite", "two images", "compare"])

    if wants_batch:
        operation_ids = _batch_ops_from_query(query, corpus_by_id)
        batch_graph = _build_batch_graph(
            query=query,
            operation_ids=operation_ids,
            corpus_by_id=corpus_by_id,
            generated_by="ambara-chatbot-mock",
        )
        if batch_graph is not None:
            return json.dumps(batch_graph)

    if wants_branch:
        branch_filter = None
        for fid in ("blend", "overlay"):
            f = corpus_by_id.get(fid)
            if f and has_port(f, "input", "base") and has_port(f, "input", "blend"):
                branch_filter = f
                break

        if branch_filter:
            graph_obj = {
                "version": "1.0.0",
                "metadata": {"generatedBy": "ambara-chatbot-mock", "query": query, "topology": "branch"},
                "nodes": [
                    {"id": "n1", "filter_id": input_f["id"], "position": {"x": 80, "y": 70}, "parameters": _default_params(input_f)},
                    {"id": "n2", "filter_id": input_f["id"], "position": {"x": 80, "y": 190}, "parameters": _default_params(input_f)},
                    {"id": "n3", "filter_id": branch_filter["id"], "position": {"x": 330, "y": 130}, "parameters": _default_params(branch_filter)},
                    {"id": "n4", "filter_id": output_f["id"], "position": {"x": 560, "y": 130}, "parameters": _default_params(output_f)},
                ],
                "connections": [
                    {"from_node": "n1", "from_port": "image", "to_node": "n3", "to_port": "base"},
                    {"from_node": "n2", "from_port": "image", "to_node": "n3", "to_port": "blend"},
                    {"from_node": "n3", "from_port": "image", "to_node": "n4", "to_port": "image"},
                ],
            }
            return json.dumps(graph_obj)

    nodes: list[dict[str, Any]] = [
        {"id": "n1", "filter_id": input_f["id"], "position": {"x": 100, "y": 120}, "parameters": _default_params(input_f)}
    ]
    connections: list[dict[str, Any]] = []

    for i, flt in enumerate(processing_filters[:3], 2):
        nodes.append(
            {
                "id": f"n{i}",
                "filter_id": flt["id"],
                "position": {"x": 100 + (i - 1) * 220, "y": 120},
                "parameters": _default_params(flt),
            }
        )
        connections.append(
            {"from_node": f"n{i-1}", "from_port": "image", "to_node": f"n{i}", "to_port": "image"}
        )

    out_idx = len(nodes) + 1
    nodes.append(
        {
            "id": f"n{out_idx}",
            "filter_id": output_f["id"],
            "position": {"x": 100 + (out_idx - 1) * 220, "y": 120},
            "parameters": _default_params(output_f),
        }
    )
    connections.append(
        {
            "from_node": f"n{out_idx-1}",
            "from_port": "image",
            "to_node": f"n{out_idx}",
            "to_port": "image",
        }
    )

    return json.dumps(
        {
            "version": "1.0.0",
            "metadata": {"generatedBy": "ambara-chatbot-mock", "query": query, "topology": "linear"},
            "nodes": nodes,
            "connections": connections,
        }
    )


class GraphGenerator:
    """Generate validated SerializedGraph JSON from natural language requests."""

    def __init__(
        self,
        chroma_path: str,
        corpus_path: str,
        examples_path: str,
        force_mock_llm: bool = False,
        llm_client: LLMClient | None = None,
    ) -> None:
        self.filter_retriever = FilterRetriever(chroma_path, corpus_path)
        self.example_retriever = ExampleRetriever(chroma_path, examples_path)
        self.prompt_builder = GraphPromptBuilder(corpus_path)
        self.repair_builder = RepairPromptBuilder()
        self.llm_client = llm_client or LLMClient(force_mock=force_mock_llm)
        self.validator = GraphValidator(
            str(ROOT / "chatbot" / "corpus" / "graph_schema.json"),
            str(ROOT / "build" / "filter_id_set.json"),
            corpus_path,
        )
        self.corpus_by_id = dict(self.filter_retriever.by_id)

    def _sanitize_graph_json(self, text: str) -> str:
        """Best-effort normalization of LLM output before validation."""
        try:
            obj = json.loads(text)
        except json.JSONDecodeError:
            return text
        fixed = _sanitize_graph_obj(obj, self.corpus_by_id)
        return json.dumps(fixed)

    def generate(self, query: str, partial_graph: dict[str, Any] | None = None) -> GenerationResultModel:
        """Run retrieval, generation, validation, and bounded self-repair."""
        retrieved_filters = self.filter_retriever.retrieve_with_graph_context(
            query=query,
            partial_graph=partial_graph or {"nodes": [], "connections": []},
            top_k=8,
        )
        examples = self.example_retriever.retrieve_examples(query, top_k=4)
        prompt = self.prompt_builder.build(query, retrieved_filters, examples, partial_graph)

        backend = getattr(self.llm_client, "backend", "custom")
        if backend == "mock":
            response_text = _build_pipeline_from_filters(retrieved_filters, query, self.corpus_by_id)
        else:
            try:
                response_text = self.llm_client.generate(prompt, temperature=0.0)
            except RuntimeError as err:
                return GenerationResultModel(
                    graph=None,
                    valid=False,
                    errors=[str(err)],
                    retries=0,
                    retrieved_filters=[f.get("id", "") for f in retrieved_filters],
                    llm_response_raw="",
                    explanation="No real LLM backend is currently reachable.",
                )

        retries = 0
        while retries <= 3:
            response_text = self._sanitize_graph_json(response_text)
            try:
                response_obj = json.loads(response_text)
                response_obj = _enforce_batch_intent(response_obj, query, self.corpus_by_id)
                response_obj = _enforce_branch_intent(response_obj, query, self.corpus_by_id)
                response_obj = _sanitize_graph_obj(response_obj, self.corpus_by_id)
                response_text = json.dumps(response_obj)
            except json.JSONDecodeError:
                pass
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

            if backend == "mock":
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
            try:
                response_text = self.llm_client.generate(repair_prompt, temperature=0.0)
            except RuntimeError as err:
                return GenerationResultModel(
                    graph=None,
                    valid=False,
                    errors=[*validation.errors, str(err)],
                    retries=retries,
                    retrieved_filters=[f.get("id", "") for f in retrieved_filters],
                    llm_response_raw=response_text,
                    explanation="Repair failed because the real LLM backend is unreachable.",
                )

        return GenerationResultModel(
            graph=None,
            valid=False,
            errors=["Unreachable state in generation loop"],
            retries=retries,
            retrieved_filters=[f.get("id", "") for f in retrieved_filters],
            llm_response_raw=response_text,
            explanation="Generation failed unexpectedly.",
        )
