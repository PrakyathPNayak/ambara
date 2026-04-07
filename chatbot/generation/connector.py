"""Stage 3 – Connect: deterministically wire selected filters into a valid graph.

This stage is 100 % code — no LLM calls.  It takes the ordered list of
selected filters from Stage 2 and produces valid SerializedGraph JSON by
applying port-type compatibility rules.

Supported topologies:
  * **linear / batch** – sequential chain where each node's output feeds
    the next node's compatible input.  Array<Image> ↔ Image transitions
    are handled transparently by the type matcher.
  * **branch** – multiple source nodes (inputs / processing branches)
    converge on one or more merge nodes (blend, overlay) which then
    feed output nodes.  Allows fan-out from a single source.
  * **dag** (implicit) – any plan whose steps carry explicit ``inputs``
    lists describing which preceding steps feed each node.  Falls back
    to linear chaining when ``inputs`` are absent.
"""

from __future__ import annotations

import logging
from typing import Any

LOGGER = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Port helpers
# ---------------------------------------------------------------------------

_IMAGE_TYPES = frozenset({"Image", "Array<Image>"})

# Filters that merge multiple images (take ≥ 2 image inputs).
_MERGE_FILTER_IDS = frozenset({"blend", "overlay"})


def _output_ports(meta: dict[str, Any]) -> list[dict[str, Any]]:
    return [p for p in meta.get("output_ports", []) if isinstance(p, dict)]


def _input_ports(meta: dict[str, Any]) -> list[dict[str, Any]]:
    return [p for p in meta.get("input_ports", []) if isinstance(p, dict)]


def _port_names(ports: list[dict[str, Any]]) -> list[str]:
    return [str(p.get("name", "")) for p in ports if p.get("name")]


def _port_type_map(ports: list[dict[str, Any]]) -> dict[str, str]:
    return {
        str(p.get("name", "")): str(p.get("type", "Any"))
        for p in ports
        if p.get("name")
    }


def _types_compatible(src_type: str, dst_type: str) -> bool:
    """Check if two port types are compatible for connection."""
    if src_type == dst_type:
        return True
    if "Any" in (src_type, dst_type):
        return True
    # Image ↔ Array<Image> coercion (batch transitions).
    if src_type in _IMAGE_TYPES and dst_type in _IMAGE_TYPES:
        return True
    return False


def _find_best_output(
    src_meta: dict[str, Any],
    dst_meta: dict[str, Any],
) -> tuple[str, str] | None:
    """Pick the best (from_port, to_port) pair between two nodes.

    Priority: exact name+type match → type match → Image coercion → first pair.
    Returns None only when one side has no ports at all.
    """
    src_out = _output_ports(src_meta)
    dst_in = _input_ports(dst_meta)

    if not src_out or not dst_in:
        return None

    src_tmap = _port_type_map(src_out)
    dst_tmap = _port_type_map(dst_in)

    # 1. Exact name AND type match.
    for sname, stype in src_tmap.items():
        if sname in dst_tmap and _types_compatible(stype, dst_tmap[sname]):
            return sname, sname

    # 2. First type-compatible pair.
    for sname, stype in src_tmap.items():
        for dname, dtype in dst_tmap.items():
            if _types_compatible(stype, dtype):
                return sname, dname

    # 3. Absolute fallback — first ports.
    return _port_names(src_out)[0], _port_names(dst_in)[0]


def _find_compatible_input(
    src_port_name: str,
    src_port_type: str,
    dst_ports: list[dict[str, Any]],
    used_ports: set[str],
) -> str | None:
    """Find the best unused compatible input port on a destination node.

    Used when wiring multiple sources into a merge node (blend / overlay).
    """
    available = [
        p for p in dst_ports
        if p.get("name") and p["name"] not in used_ports
    ]
    if not available:
        return None

    # 1. Exact name match.
    for p in available:
        if p["name"] == src_port_name:
            return p["name"]

    # 2. Type-compatible.
    for p in available:
        if _types_compatible(src_port_type, str(p.get("type", "Any"))):
            return p["name"]

    # 3. First available.
    return available[0]["name"]


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------

def build_graph(
    selections: list[dict[str, Any]],
    plan: dict[str, Any],
    corpus_by_id: dict[str, dict[str, Any]],
    query: str,
) -> dict[str, Any]:
    """Build a complete SerializedGraph from the plan and selected filters.

    Handles three topologies:
      * linear / batch – sequential A → B → C chain
      * branch – multi-source merge via blend / overlay
      * dag (planned) – arbitrary wiring via step ``inputs`` lists

    Args:
        selections: Ordered list of ``{filter_id, parameters}`` from Stage 2.
        plan: Plan dict from Stage 1 (contains topology, steps).
        corpus_by_id: Filter metadata indexed by filter_id.
        query: Original user query.

    Returns:
        Complete SerializedGraph dict.
    """
    topology = plan.get("topology", "linear")

    if topology == "branch":
        return _build_branch_graph(selections, corpus_by_id, query)

    return _build_linear_graph(selections, corpus_by_id, query, topology)


# ---------------------------------------------------------------------------
# Linear / batch builder
# ---------------------------------------------------------------------------

def _build_linear_graph(
    selections: list[dict[str, Any]],
    corpus_by_id: dict[str, dict[str, Any]],
    query: str,
    topology: str,
) -> dict[str, Any]:
    """Build a sequential chain graph.

    Connects each node pair using port-type matching.  Array<Image> ↔ Image
    transitions are handled transparently so there is no difference between
    "linear" and "batch" wiring — the types decide which ports match.
    """
    nodes: list[dict[str, Any]] = []
    connections: list[dict[str, Any]] = []

    x_offset = 80
    x_step = 220
    y_pos = 120.0

    for idx, sel in enumerate(selections):
        nodes.append({
            "id": f"n{idx + 1}",
            "filter_id": sel["filter_id"],
            "position": {"x": float(x_offset + idx * x_step), "y": y_pos},
            "parameters": sel.get("parameters", {}),
        })

    # Wire consecutive pairs using type-aware matching.
    for i in range(len(nodes) - 1):
        src_meta = corpus_by_id.get(nodes[i]["filter_id"], {})
        dst_meta = corpus_by_id.get(nodes[i + 1]["filter_id"], {})

        pair = _find_best_output(src_meta, dst_meta)
        if pair:
            from_port, to_port = pair
        else:
            from_port, to_port = "image", "image"

        connections.append({
            "from_node": nodes[i]["id"],
            "from_port": from_port,
            "to_node": nodes[i + 1]["id"],
            "to_port": to_port,
        })

    return _wrap_graph(nodes, connections, query, topology)


# ---------------------------------------------------------------------------
# Branch (merge) builder
# ---------------------------------------------------------------------------

def _build_branch_graph(
    selections: list[dict[str, Any]],
    corpus_by_id: dict[str, dict[str, Any]],
    query: str,
) -> dict[str, Any]:
    """Build a branching graph for blend / overlay / multi-input operations.

    Categorises selections into source, processing, merge, and output groups,
    then wires them following DAG rules:
      * Each source branch gets its own processing chain.
      * All branches converge on the merge node(s).
      * Merge output feeds the output node(s).
    Fan-out from a single source to multiple merge inputs is supported.
    """
    nodes: list[dict[str, Any]] = []
    connections: list[dict[str, Any]] = []

    # --- Categorise selections ---
    input_sels:      list[dict[str, Any]] = []
    processing_sels: list[dict[str, Any]] = []
    merge_sels:      list[dict[str, Any]] = []
    output_sels:     list[dict[str, Any]] = []

    for sel in selections:
        fid = sel["filter_id"]
        meta = corpus_by_id.get(fid, {})
        cat = meta.get("category", "").lower()
        if cat == "input" or fid in ("load_image", "load_folder"):
            input_sels.append(sel)
        elif fid in _MERGE_FILTER_IDS:
            merge_sels.append(sel)
        elif cat == "output" or fid in ("save_image", "batch_save_images"):
            output_sels.append(sel)
        else:
            processing_sels.append(sel)

    # Ensure structural minimums.
    if len(input_sels) < 2:
        while len(input_sels) < 2:
            input_sels.append({"filter_id": "load_image", "parameters": {}})
    if not merge_sels:
        merge_sels.append({"filter_id": "blend", "parameters": {"opacity": 0.5, "mode": "Normal"}})
    if not output_sels:
        output_sels.append({"filter_id": "save_image", "parameters": {}})

    # --- Create nodes with spatial layout ---
    x = 80.0
    node_idx = 1

    # Input nodes (stacked vertically).
    input_nids: list[str] = []
    for i, sel in enumerate(input_sels):
        nid = f"n{node_idx}"
        nodes.append({
            "id": nid,
            "filter_id": sel["filter_id"],
            "position": {"x": x, "y": 80.0 + i * 160.0},
            "parameters": sel.get("parameters", {}),
        })
        input_nids.append(nid)
        node_idx += 1
    x += 220.0

    # Processing nodes (single chain off the first input branch).
    proc_nids: list[str] = []
    for sel in processing_sels:
        nid = f"n{node_idx}"
        nodes.append({
            "id": nid,
            "filter_id": sel["filter_id"],
            "position": {"x": x, "y": 80.0},
            "parameters": sel.get("parameters", {}),
        })
        proc_nids.append(nid)
        node_idx += 1
        x += 220.0

    # Merge node(s).
    merge_nids: list[str] = []
    for sel in merge_sels:
        nid = f"n{node_idx}"
        nodes.append({
            "id": nid,
            "filter_id": sel["filter_id"],
            "position": {"x": x, "y": 120.0},
            "parameters": sel.get("parameters", {}),
        })
        merge_nids.append(nid)
        node_idx += 1
        x += 220.0

    # Output node(s).
    output_nids: list[str] = []
    for sel in output_sels:
        nid = f"n{node_idx}"
        nodes.append({
            "id": nid,
            "filter_id": sel["filter_id"],
            "position": {"x": x, "y": 120.0},
            "parameters": sel.get("parameters", {}),
        })
        output_nids.append(nid)
        node_idx += 1
        x += 220.0

    nmap = {n["id"]: n for n in nodes}

    # --- Wire connections ---

    # Determine the node that feeds the merge's first (base) input.
    first_branch_tip = input_nids[0]

    # If there are processing nodes, chain: input[0] → proc[0] → … → proc[-1]
    if proc_nids:
        _add_typed_conn(connections, input_nids[0], proc_nids[0], corpus_by_id, nmap)
        for i in range(len(proc_nids) - 1):
            _add_typed_conn(connections, proc_nids[i], proc_nids[i + 1], corpus_by_id, nmap)
        first_branch_tip = proc_nids[-1]

    # Connect branches into the first merge node.
    if merge_nids:
        merge_fid = nmap[merge_nids[0]]["filter_id"]
        merge_meta = corpus_by_id.get(merge_fid, {})
        merge_in = _input_ports(merge_meta)
        used: set[str] = set()

        # First branch → merge (base / first input).
        _connect_to_merge(
            connections, first_branch_tip, merge_nids[0],
            corpus_by_id, nmap, merge_in, used,
        )

        # Remaining inputs → merge (next available input port each).
        for src_nid in input_nids[1:]:
            _connect_to_merge(
                connections, src_nid, merge_nids[0],
                corpus_by_id, nmap, merge_in, used,
            )

        # Chain merge nodes if there are multiple.
        for i in range(len(merge_nids) - 1):
            _add_typed_conn(connections, merge_nids[i], merge_nids[i + 1], corpus_by_id, nmap)

    # Merge (or last merge) → output.
    last_merge = merge_nids[-1] if merge_nids else first_branch_tip
    if output_nids:
        _add_typed_conn(connections, last_merge, output_nids[0], corpus_by_id, nmap)
        for i in range(len(output_nids) - 1):
            _add_typed_conn(connections, output_nids[i], output_nids[i + 1], corpus_by_id, nmap)

    return _wrap_graph(nodes, connections, query, "branch")


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _wrap_graph(
    nodes: list[dict[str, Any]],
    connections: list[dict[str, Any]],
    query: str,
    topology: str,
) -> dict[str, Any]:
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


def _add_typed_conn(
    connections: list[dict[str, Any]],
    from_nid: str,
    to_nid: str,
    corpus_by_id: dict[str, dict[str, Any]],
    nmap: dict[str, dict[str, Any]],
) -> None:
    """Add a connection between two nodes using type-aware port matching."""
    src_meta = corpus_by_id.get(nmap[from_nid]["filter_id"], {})
    dst_meta = corpus_by_id.get(nmap[to_nid]["filter_id"], {})

    pair = _find_best_output(src_meta, dst_meta)
    if pair:
        from_port, to_port = pair
    else:
        from_port, to_port = "image", "image"

    connections.append({
        "from_node": from_nid,
        "from_port": from_port,
        "to_node": to_nid,
        "to_port": to_port,
    })


def _connect_to_merge(
    connections: list[dict[str, Any]],
    src_nid: str,
    merge_nid: str,
    corpus_by_id: dict[str, dict[str, Any]],
    nmap: dict[str, dict[str, Any]],
    merge_in: list[dict[str, Any]],
    used: set[str],
) -> None:
    """Wire a source node into the next available input port on a merge node."""
    src_meta = corpus_by_id.get(nmap[src_nid]["filter_id"], {})
    src_out = _output_ports(src_meta)

    if src_out:
        from_port = _port_names(src_out)[0]
        from_type = _port_type_map(src_out).get(from_port, "Any")
    else:
        from_port, from_type = "image", "Image"

    to_port = _find_compatible_input(from_port, from_type, merge_in, used)
    if not to_port:
        to_port = "image"

    used.add(to_port)
    connections.append({
        "from_node": src_nid,
        "from_port": from_port,
        "to_node": merge_nid,
        "to_port": to_port,
    })
