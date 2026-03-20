"""Stage 3 – Connect: deterministically wire selected filters into a valid graph.

This stage is 100% code — no LLM calls.  It takes the ordered list of
selected filters from Stage 2 and produces valid SerializedGraph JSON by
applying port-type compatibility rules.
"""

from __future__ import annotations

import logging
from typing import Any

LOGGER = logging.getLogger(__name__)


def _output_ports(meta: dict[str, Any]) -> list[dict[str, Any]]:
    return [p for p in meta.get("output_ports", []) if isinstance(p, dict)]


def _input_ports(meta: dict[str, Any]) -> list[dict[str, Any]]:
    return [p for p in meta.get("input_ports", []) if isinstance(p, dict)]


def _port_names(ports: list[dict[str, Any]]) -> list[str]:
    return [str(p.get("name", "")) for p in ports if p.get("name")]


def _port_type_map(ports: list[dict[str, Any]]) -> dict[str, str]:
    return {str(p.get("name", "")): str(p.get("type", "Any")) for p in ports if p.get("name")}


def _find_compatible_port(
    src_port_name: str,
    src_port_type: str,
    dst_ports: list[dict[str, Any]],
    used_ports: set[str],
) -> str | None:
    """Find the best compatible destination port.

    Priority: exact name match > exact type match > Any type > first available.

    Args:
        src_port_name: Source port name.
        src_port_type: Source port type.
        dst_ports: Available destination ports.
        used_ports: Already-used destination port names.

    Returns:
        Port name or None if no compatible port exists.
    """
    available = [p for p in dst_ports if p.get("name") and p["name"] not in used_ports]
    if not available:
        return None

    # 1. Exact name match.
    for p in available:
        if p["name"] == src_port_name:
            return p["name"]

    # 2. Exact type match.
    for p in available:
        ptype = str(p.get("type", "Any"))
        if ptype == src_port_type:
            return p["name"]

    # 3. Any type on either side.
    if src_port_type == "Any":
        return available[0]["name"]
    for p in available:
        if str(p.get("type", "Any")) == "Any":
            return p["name"]

    # 4. Image ↔ Image(Array) coercion for batch transitions.
    image_types = {"Image", "Array(Image)"}
    if src_port_type in image_types:
        for p in available:
            if str(p.get("type", "")) in image_types:
                return p["name"]

    # 5. First available as last resort.
    return available[0]["name"]


def build_graph(
    selections: list[dict[str, Any]],
    plan: dict[str, Any],
    corpus_by_id: dict[str, dict[str, Any]],
    query: str,
) -> dict[str, Any]:
    """Build a complete SerializedGraph from the plan and selected filters.

    Handles three topologies:
    - linear: A → B → C → D
    - batch: load_folder → batch_ops → batch_save
    - branch: multiple inputs merging at blend/overlay

    Args:
        selections: Ordered list of {filter_id, parameters} from Stage 2.
        plan: Plan dict from Stage 1 (contains topology, steps).
        corpus_by_id: Filter metadata indexed by filter_id.
        query: Original user query.

    Returns:
        Complete SerializedGraph dict.
    """
    topology = plan.get("topology", "linear")

    if topology == "branch":
        return _build_branch_graph(selections, corpus_by_id, query)

    # Linear and batch use the same algorithm — just different port names.
    return _build_linear_graph(selections, corpus_by_id, query, topology)


def _build_linear_graph(
    selections: list[dict[str, Any]],
    corpus_by_id: dict[str, dict[str, Any]],
    query: str,
    topology: str,
) -> dict[str, Any]:
    """Build a linear chain graph."""
    nodes: list[dict[str, Any]] = []
    connections: list[dict[str, Any]] = []

    x_offset = 80
    x_step = 220
    y_pos = 120.0

    for idx, sel in enumerate(selections):
        fid = sel["filter_id"]
        params = sel.get("parameters", {})
        node_id = f"n{idx + 1}"

        nodes.append({
            "id": node_id,
            "filter_id": fid,
            "position": {"x": float(x_offset + idx * x_step), "y": y_pos},
            "parameters": params,
        })

    # Connect consecutive nodes.
    for i in range(len(nodes) - 1):
        src_node = nodes[i]
        dst_node = nodes[i + 1]
        src_meta = corpus_by_id.get(src_node["filter_id"], {})
        dst_meta = corpus_by_id.get(dst_node["filter_id"], {})

        src_out = _output_ports(src_meta)
        dst_in = _input_ports(dst_meta)

        if not src_out or not dst_in:
            # Best-effort fallback.
            from_port = "images" if topology == "batch" else "image"
            to_port = "images" if topology == "batch" else "image"
        else:
            src_type_map = _port_type_map(src_out)
            # Pick first output port.
            from_port = _port_names(src_out)[0]
            from_type = src_type_map.get(from_port, "Any")
            to_port_name = _find_compatible_port(from_port, from_type, dst_in, set())
            to_port = to_port_name or ("images" if topology == "batch" else "image")

        connections.append({
            "from_node": src_node["id"],
            "from_port": from_port,
            "to_node": dst_node["id"],
            "to_port": to_port,
        })

    return {
        "version": "1.0.0",
        "metadata": {
            "generatedBy": "ambara-agentic-pipeline",
            "query": query,
            "topology": topology,
        },
        "nodes": nodes,
        "connections": connections,
    }


def _build_branch_graph(
    selections: list[dict[str, Any]],
    corpus_by_id: dict[str, dict[str, Any]],
    query: str,
) -> dict[str, Any]:
    """Build a branching graph for blend/overlay operations.

    Identifies merge nodes (blend, overlay) and connects their multiple inputs
    from preceding source nodes.
    """
    nodes: list[dict[str, Any]] = []
    connections: list[dict[str, Any]] = []

    merge_filter_ids = {"blend", "overlay"}

    # Categorize selections.
    input_sels: list[dict[str, Any]] = []
    merge_sels: list[dict[str, Any]] = []
    processing_sels: list[dict[str, Any]] = []
    output_sels: list[dict[str, Any]] = []

    for sel in selections:
        fid = sel["filter_id"]
        meta = corpus_by_id.get(fid, {})
        cat = meta.get("category", "").lower()
        if cat == "input" or fid in ("load_image", "load_folder"):
            input_sels.append(sel)
        elif fid in merge_filter_ids:
            merge_sels.append(sel)
        elif cat == "output" or fid in ("save_image", "batch_save_images"):
            output_sels.append(sel)
        else:
            processing_sels.append(sel)

    # Ensure we have at least 2 inputs for branching.
    while len(input_sels) < 2:
        input_sels.append({"filter_id": "load_image", "parameters": {"path": ""}})

    # Ensure we have at least one merge node.
    if not merge_sels:
        merge_sels.append({"filter_id": "blend", "parameters": {"opacity": 0.5, "mode": "Normal"}})

    # Ensure we have an output.
    if not output_sels:
        output_sels.append({"filter_id": "save_image", "parameters": {"path": "output.png"}})

    # Layout: inputs on left, processing in middle, merge, output on right.
    x = 80.0
    node_idx = 1

    # Create input nodes.
    input_node_ids: list[str] = []
    for i, sel in enumerate(input_sels):
        nid = f"n{node_idx}"
        nodes.append({
            "id": nid,
            "filter_id": sel["filter_id"],
            "position": {"x": x, "y": 80.0 + i * 160.0},
            "parameters": sel.get("parameters", {}),
        })
        input_node_ids.append(nid)
        node_idx += 1

    x += 220.0

    # Create processing nodes (attached to first input branch).
    proc_node_ids: list[str] = []
    for sel in processing_sels:
        nid = f"n{node_idx}"
        nodes.append({
            "id": nid,
            "filter_id": sel["filter_id"],
            "position": {"x": x, "y": 80.0},
            "parameters": sel.get("parameters", {}),
        })
        proc_node_ids.append(nid)
        node_idx += 1
        x += 220.0

    # Create merge node(s).
    merge_node_ids: list[str] = []
    for sel in merge_sels:
        nid = f"n{node_idx}"
        nodes.append({
            "id": nid,
            "filter_id": sel["filter_id"],
            "position": {"x": x, "y": 120.0},
            "parameters": sel.get("parameters", {}),
        })
        merge_node_ids.append(nid)
        node_idx += 1
        x += 220.0

    # Create output node(s).
    output_node_ids: list[str] = []
    for sel in output_sels:
        nid = f"n{node_idx}"
        nodes.append({
            "id": nid,
            "filter_id": sel["filter_id"],
            "position": {"x": x, "y": 120.0},
            "parameters": sel.get("parameters", {}),
        })
        output_node_ids.append(nid)
        node_idx += 1
        x += 220.0

    # Wire connections.
    # Connect input processing chain.
    if proc_node_ids:
        # First input → first processing node.
        _add_connection(connections, input_node_ids[0], proc_node_ids[0], corpus_by_id, nodes)
        # Chain processing nodes.
        for i in range(len(proc_node_ids) - 1):
            _add_connection(connections, proc_node_ids[i], proc_node_ids[i + 1], corpus_by_id, nodes)
        # Last processing → merge (base input).
        if merge_node_ids:
            merge_meta = corpus_by_id.get(
                _node_filter_id(merge_node_ids[0], nodes), {}
            )
            merge_in = _input_ports(merge_meta)
            base_port = "base" if any(p.get("name") == "base" for p in merge_in) else (_port_names(merge_in)[0] if merge_in else "base")
            last_proc_meta = corpus_by_id.get(_node_filter_id(proc_node_ids[-1], nodes), {})
            src_out = _output_ports(last_proc_meta)
            from_port = _port_names(src_out)[0] if src_out else "image"
            connections.append({
                "from_node": proc_node_ids[-1],
                "from_port": from_port,
                "to_node": merge_node_ids[0],
                "to_port": base_port,
            })
    else:
        # No processing — connect first input directly to merge base.
        if merge_node_ids:
            merge_meta = corpus_by_id.get(
                _node_filter_id(merge_node_ids[0], nodes), {}
            )
            merge_in = _input_ports(merge_meta)
            base_port = "base" if any(p.get("name") == "base" for p in merge_in) else (_port_names(merge_in)[0] if merge_in else "base")
            connections.append({
                "from_node": input_node_ids[0],
                "from_port": "image",
                "to_node": merge_node_ids[0],
                "to_port": base_port,
            })

    # Connect second input to merge's second input (blend/overlay port).
    if len(input_node_ids) >= 2 and merge_node_ids:
        merge_meta = corpus_by_id.get(
            _node_filter_id(merge_node_ids[0], nodes), {}
        )
        merge_in = _input_ports(merge_meta)
        used = {c["to_port"] for c in connections if c["to_node"] == merge_node_ids[0]}
        blend_port = None
        for p in merge_in:
            if p.get("name") and p["name"] not in used:
                blend_port = p["name"]
                break
        if not blend_port:
            blend_port = "blend"
        connections.append({
            "from_node": input_node_ids[1],
            "from_port": "image",
            "to_node": merge_node_ids[0],
            "to_port": blend_port,
        })

    # Connect merge → output.
    if merge_node_ids and output_node_ids:
        _add_connection(connections, merge_node_ids[-1], output_node_ids[0], corpus_by_id, nodes)

    return {
        "version": "1.0.0",
        "metadata": {
            "generatedBy": "ambara-agentic-pipeline",
            "query": query,
            "topology": "branch",
        },
        "nodes": nodes,
        "connections": connections,
    }


def _node_filter_id(node_id: str, nodes: list[dict[str, Any]]) -> str:
    """Look up filter_id for a node_id."""
    for n in nodes:
        if n["id"] == node_id:
            return n.get("filter_id", "")
    return ""


def _add_connection(
    connections: list[dict[str, Any]],
    from_nid: str,
    to_nid: str,
    corpus_by_id: dict[str, dict[str, Any]],
    nodes: list[dict[str, Any]],
) -> None:
    """Add a connection between two nodes using port-type compatibility."""
    src_fid = _node_filter_id(from_nid, nodes)
    dst_fid = _node_filter_id(to_nid, nodes)
    src_meta = corpus_by_id.get(src_fid, {})
    dst_meta = corpus_by_id.get(dst_fid, {})

    src_out = _output_ports(src_meta)
    dst_in = _input_ports(dst_meta)

    from_port = _port_names(src_out)[0] if src_out else "image"
    src_type = _port_type_map(src_out).get(from_port, "Any")

    used = {c["to_port"] for c in connections if c["to_node"] == to_nid}
    to_port = _find_compatible_port(from_port, src_type, dst_in, used)
    if not to_port:
        to_port = "image"

    connections.append({
        "from_node": from_nid,
        "from_port": from_port,
        "to_node": to_nid,
        "to_port": to_port,
    })
