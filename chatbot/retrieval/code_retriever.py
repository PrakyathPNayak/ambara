"""Code-as-RAG retriever that reads Ambara Rust source files directly.

Instead of relying on a pre-built ChromaDB index, this retriever parses the actual
filter implementation source code to provide accurate, up-to-date metadata.  It
extracts struct definitions, metadata builder calls, port definitions, parameter
constraints, and even implementation snippets.

The retriever maintains an in-memory index that is built once on first access and
can be refreshed at any time by calling ``refresh()``.
"""

from __future__ import annotations

import json
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
BUILTIN_DIR = ROOT / "src" / "filters" / "builtin"
CORPUS_CACHE = ROOT / "build" / "filter_corpus.json"


# ---------------------------------------------------------------------------
# Data structures
# ---------------------------------------------------------------------------

@dataclass
class PortInfo:
    """Extracted port definition."""
    name: str
    port_type: str
    description: str = ""


@dataclass
class ParamInfo:
    """Extracted parameter definition."""
    name: str
    port_type: str
    default: str = ""
    description: str = ""
    constraint: str = ""
    ui_hint: str = ""


@dataclass
class FilterInfo:
    """Complete filter metadata extracted from source code."""
    id: str
    name: str
    description: str = ""
    category: str = "Custom"
    author: str = ""
    version: str = ""
    source_file: str = ""
    inputs: list[PortInfo] = field(default_factory=list)
    outputs: list[PortInfo] = field(default_factory=list)
    parameters: list[ParamInfo] = field(default_factory=list)
    source_snippet: str = ""  # The metadata() block for LLM context
    implementation_snippet: str = ""  # The execute() block for deep context
    struct_name: str = ""

    def to_dict(self) -> dict[str, Any]:
        """Serialize to dict matching FilterDoc shape."""
        return {
            "id": self.id,
            "name": self.name,
            "description": self.description,
            "category": self.category,
            "input_ports": [{"name": p.name, "type": p.port_type, "description": p.description} for p in self.inputs],
            "output_ports": [{"name": p.name, "type": p.port_type, "description": p.description} for p in self.outputs],
            "parameters": [
                {
                    "name": p.name,
                    "type": p.port_type,
                    "default": p.default,
                    "description": p.description,
                    "constraint": p.constraint,
                    "ui_hint": p.ui_hint,
                }
                for p in self.parameters
            ],
            "tags": [self.source_file, self.category.lower()],
            "source_file": self.source_file,
            "struct_name": self.struct_name,
        }

    def to_compact_card(self) -> str:
        """One-line compact card for LLM context (id, category, ports, params)."""
        ins = ", ".join(f"{p.name}:{p.port_type}" for p in self.inputs)
        outs = ", ".join(f"{p.name}:{p.port_type}" for p in self.outputs)
        params = ", ".join(
            f"{p.name}:{p.port_type}={p.default}" for p in self.parameters
        )
        return f"[{self.id}] {self.name} ({self.category}) | in({ins}) → out({outs}) | params({params})"

    def to_rich_context(self) -> str:
        """Detailed context block including source snippets for deep LLM reasoning."""
        lines = [
            f"## Filter: {self.name} (`{self.id}`)",
            f"Category: {self.category}",
            f"Description: {self.description}",
            f"Source: {self.source_file}",
            "",
        ]
        if self.inputs:
            lines.append("### Inputs")
            for p in self.inputs:
                lines.append(f"  - `{p.name}` ({p.port_type}): {p.description}")
        if self.outputs:
            lines.append("### Outputs")
            for p in self.outputs:
                lines.append(f"  - `{p.name}` ({p.port_type}): {p.description}")
        if self.parameters:
            lines.append("### Parameters")
            for p in self.parameters:
                constraint = f" [{p.constraint}]" if p.constraint else ""
                lines.append(f"  - `{p.name}` ({p.port_type}, default={p.default}){constraint}: {p.description}")
        if self.source_snippet:
            lines.append("\n### Metadata Source")
            lines.append(f"```rust\n{self.source_snippet}\n```")
        return "\n".join(lines)


# ---------------------------------------------------------------------------
# Parser — extracts FilterInfo from Rust source files
# ---------------------------------------------------------------------------

# Regex patterns for parsing Rust filter source code
_RE_BUILDER = re.compile(
    r'NodeMetadata::builder\(\s*"([^"]+)"\s*,\s*"([^"]+)"\s*\)',
)
_RE_DESCRIPTION = re.compile(r'\.description\(\s*"([^"]+)"\s*\)')
_RE_CATEGORY = re.compile(r'\.category\(\s*Category::(\w+)\s*\)')
_RE_AUTHOR = re.compile(r'\.author\(\s*"([^"]+)"\s*\)')
_RE_VERSION = re.compile(r'\.version\(\s*"([^"]+)"\s*\)')
_RE_INPUT = re.compile(
    r'\.input\(\s*PortDefinition::input\(\s*"([^"]+)"\s*,\s*PortType::(\w+)\s*\)'
)
_RE_OUTPUT = re.compile(
    r'\.output\(\s*PortDefinition::output\(\s*"([^"]+)"\s*,\s*PortType::(\w+)\s*\)'
)
_RE_PARAM = re.compile(
    r'ParameterDefinition::new\(\s*"([^"]+)"\s*,\s*PortType::(\w+)\s*,\s*Value::(\w+)\(([^)]*)\)\s*\)'
)
_RE_PARAM_DESC = re.compile(r'\.with_description\(\s*"([^"]+)"\s*\)')
_RE_CONSTRAINT_RANGE = re.compile(
    r'\.with_constraint\(\s*Constraint::Range\s*\{\s*min:\s*([\d.\-]+)\s*,\s*max:\s*([\d.\-]+)\s*\}\s*\)'
)
_RE_CONSTRAINT_ONEOF = re.compile(r'\.with_constraint\(\s*Constraint::OneOf\(([^)]+)\)\s*\)')
_RE_UI_HINT = re.compile(r'\.with_ui_hint\(\s*UiHint::(\w+)')
_RE_PORT_DESC = re.compile(r'\.with_description\(\s*"([^"]+)"\s*\)')
_RE_STRUCT = re.compile(r'pub\s+struct\s+(\w+)\s*;')

# Pattern to find `impl FilterNode for X` blocks
_RE_IMPL_BLOCK = re.compile(r'impl\s+FilterNode\s+for\s+(\w+)\s*\{')


def _extract_between_braces(text: str, start: int) -> str:
    """Extract text between matched braces starting at position ``start``."""
    depth = 0
    i = start
    while i < len(text):
        if text[i] == '{':
            depth += 1
        elif text[i] == '}':
            depth -= 1
            if depth == 0:
                return text[start:i + 1]
        i += 1
    return text[start:]


def _extract_method(impl_block: str, method_name: str) -> str:
    """Extract a specific method body from an impl block."""
    pattern = re.compile(rf'fn\s+{method_name}\s*\(')
    match = pattern.search(impl_block)
    if not match:
        return ""
    # Find the opening brace of the method body
    brace_pos = impl_block.find('{', match.start())
    if brace_pos < 0:
        return ""
    body = _extract_between_braces(impl_block, brace_pos)
    return impl_block[match.start():match.start() + len(f"fn {method_name}(")] + "..." + body


def _parse_param_block(text: str) -> ParamInfo:
    """Parse a .parameter(...) block into ParamInfo."""
    m_new = _RE_PARAM.search(text)
    if not m_new:
        return ParamInfo(name="?", port_type="?")

    name = m_new.group(1)
    port_type = m_new.group(2)
    value_type = m_new.group(3)
    raw_default = m_new.group(4).strip().strip('"')

    # Simplify default representation
    default_val = raw_default if raw_default else ""
    if value_type == "String" and default_val:
        default_val = f'"{default_val}"'

    desc_m = _RE_PARAM_DESC.search(text)
    description = desc_m.group(1) if desc_m else ""

    constraint = ""
    range_m = _RE_CONSTRAINT_RANGE.search(text)
    if range_m:
        constraint = f"range({range_m.group(1)}..{range_m.group(2)})"
    else:
        oneof_m = _RE_CONSTRAINT_ONEOF.search(text)
        if oneof_m:
            constraint = f"one_of({oneof_m.group(1).strip()})"

    hint_m = _RE_UI_HINT.search(text)
    ui_hint = hint_m.group(1) if hint_m else ""

    return ParamInfo(
        name=name,
        port_type=port_type,
        default=default_val,
        description=description,
        constraint=constraint,
        ui_hint=ui_hint,
    )


def _parse_source_file(path: Path) -> list[FilterInfo]:
    """Parse a single Rust source file and extract all filters."""
    text = path.read_text(errors="replace")
    filename = path.name
    filters: list[FilterInfo] = []

    # Find all impl FilterNode for <Struct> blocks
    for impl_match in _RE_IMPL_BLOCK.finditer(text):
        struct_name = impl_match.group(1)
        brace_start = text.find('{', impl_match.start())
        if brace_start < 0:
            continue
        impl_block = _extract_between_braces(text, brace_start)

        # Extract metadata() method
        metadata_src = _extract_method(impl_block, "metadata")
        if not metadata_src:
            continue

        # Parse builder call
        builder_m = _RE_BUILDER.search(metadata_src)
        if not builder_m:
            continue

        fid = builder_m.group(1)
        fname = builder_m.group(2)

        desc_m = _RE_DESCRIPTION.search(metadata_src)
        cat_m = _RE_CATEGORY.search(metadata_src)
        auth_m = _RE_AUTHOR.search(metadata_src)
        ver_m = _RE_VERSION.search(metadata_src)

        # Parse inputs
        inputs: list[PortInfo] = []
        for inp_m in _RE_INPUT.finditer(metadata_src):
            port_name, port_type = inp_m.group(1), inp_m.group(2)
            # Look for .with_description after this port
            after = metadata_src[inp_m.end():inp_m.end() + 200]
            pdesc_m = _RE_PORT_DESC.search(after)
            inputs.append(PortInfo(port_name, port_type, pdesc_m.group(1) if pdesc_m else ""))

        # Parse outputs
        outputs: list[PortInfo] = []
        for out_m in _RE_OUTPUT.finditer(metadata_src):
            port_name, port_type = out_m.group(1), out_m.group(2)
            after = metadata_src[out_m.end():out_m.end() + 200]
            pdesc_m = _RE_PORT_DESC.search(after)
            outputs.append(PortInfo(port_name, port_type, pdesc_m.group(1) if pdesc_m else ""))

        # Parse parameters — split by .parameter( blocks
        param_blocks = re.split(r'\.parameter\(', metadata_src)
        parameters: list[ParamInfo] = []
        for pb in param_blocks[1:]:  # skip content before first .parameter(
            parameters.append(_parse_param_block(pb))

        # Extract execute() snippet for deep context
        execute_src = _extract_method(impl_block, "execute")

        info = FilterInfo(
            id=fid,
            name=fname,
            description=desc_m.group(1) if desc_m else f"{fname} filter",
            category=cat_m.group(1) if cat_m else path.stem.capitalize(),
            author=auth_m.group(1) if auth_m else "",
            version=ver_m.group(1) if ver_m else "",
            source_file=filename,
            inputs=inputs,
            outputs=outputs,
            parameters=parameters,
            source_snippet=metadata_src[:2000],
            implementation_snippet=execute_src[:3000] if execute_src else "",
            struct_name=struct_name,
        )
        filters.append(info)

    return filters


# ---------------------------------------------------------------------------
# CodeRetriever — the main public interface
# ---------------------------------------------------------------------------

class CodeRetriever:
    """Retrieves filter information directly from Rust source code.

    This replaces ChromaDB-based retrieval with direct source parsing,
    providing accurate and always-up-to-date filter metadata.
    """

    def __init__(self, source_dir: str | Path | None = None) -> None:
        self.source_dir = Path(source_dir) if source_dir else BUILTIN_DIR
        self._index: dict[str, FilterInfo] = {}
        self._by_category: dict[str, list[FilterInfo]] = {}
        self._loaded = False

    def _ensure_loaded(self) -> None:
        if not self._loaded:
            self.refresh()

    def refresh(self) -> None:
        """Re-parse all source files and rebuild the in-memory index."""
        self._index.clear()
        self._by_category.clear()

        for path in sorted(self.source_dir.glob("*.rs")):
            if path.name == "mod.rs":
                continue
            for info in _parse_source_file(path):
                self._index[info.id] = info
                self._by_category.setdefault(info.category, []).append(info)

        self._loaded = True

    # --- Query methods ---

    @property
    def all_filter_ids(self) -> list[str]:
        self._ensure_loaded()
        return sorted(self._index.keys())

    @property
    def all_filters(self) -> list[FilterInfo]:
        self._ensure_loaded()
        return list(self._index.values())

    @property
    def categories(self) -> dict[str, list[str]]:
        self._ensure_loaded()
        return {cat: [f.id for f in fs] for cat, fs in sorted(self._by_category.items())}

    def get(self, filter_id: str) -> FilterInfo | None:
        """Get full FilterInfo for a specific filter ID."""
        self._ensure_loaded()
        return self._index.get(filter_id)

    def get_details(self, filter_id: str) -> str:
        """Get rich context string for a filter, suitable for LLM prompts."""
        info = self.get(filter_id)
        if not info:
            return f"Filter '{filter_id}' not found."
        return info.to_rich_context()

    def get_source(self, filter_id: str) -> str:
        """Get complete source file content for a filter."""
        info = self.get(filter_id)
        if not info:
            return f"Filter '{filter_id}' not found."
        path = self.source_dir / info.source_file
        if path.exists():
            return path.read_text(errors="replace")
        return f"Source file {info.source_file} not found."

    def search(self, query: str, top_k: int = 5) -> list[FilterInfo]:
        """Search filters by keyword matching against id, name, description, category."""
        self._ensure_loaded()
        q = query.lower()
        scored: list[tuple[float, FilterInfo]] = []

        for info in self._index.values():
            score = 0.0
            # Exact ID match
            if q == info.id:
                score += 100
            # ID contains query
            elif q in info.id:
                score += 50
            # Name match
            if q in info.name.lower():
                score += 40
            # Category match
            if q in info.category.lower():
                score += 30
            # Description match
            words = q.split()
            for word in words:
                if len(word) >= 3:
                    if word in info.description.lower():
                        score += 15
                    if word in info.id:
                        score += 20
                    # Check parameter names/descriptions
                    for p in info.parameters:
                        if word in p.name or word in p.description.lower():
                            score += 5
            if score > 0:
                scored.append((score, info))

        scored.sort(key=lambda x: -x[0])
        return [info for _, info in scored[:top_k]]

    def search_by_category(self, category: str) -> list[FilterInfo]:
        """Get all filters in a category."""
        self._ensure_loaded()
        return self._by_category.get(category, [])

    def search_by_port_type(self, port_type: str, direction: str = "input") -> list[FilterInfo]:
        """Find filters that accept or produce a specific port type."""
        self._ensure_loaded()
        results = []
        for info in self._index.values():
            ports = info.inputs if direction == "input" else info.outputs
            if any(p.port_type == port_type for p in ports):
                results.append(info)
        return results

    def get_compatible_next(self, filter_id: str) -> list[FilterInfo]:
        """Find filters that can connect after a given filter (output→input type match)."""
        info = self.get(filter_id)
        if not info or not info.outputs:
            return []
        out_types = {p.port_type for p in info.outputs}
        results = []
        for candidate in self._index.values():
            if candidate.id == filter_id:
                continue
            if any(p.port_type in out_types or p.port_type == "Any" for p in candidate.inputs):
                results.append(candidate)
        return results

    def build_catalog(self) -> str:
        """Build a compact filter catalog string for LLM planning prompts."""
        self._ensure_loaded()
        lines = []
        for cat, filters in sorted(self._by_category.items()):
            entries = ", ".join(
                f"{f.id} ({f.description[:50]})" for f in filters
            )
            lines.append(f"{cat}: {entries}")
        return "\n".join(lines)

    def build_corpus_json(self) -> list[dict[str, Any]]:
        """Build filter corpus in the same format as filter_corpus.json."""
        self._ensure_loaded()
        return [info.to_dict() for info in self._index.values()]

    def export_corpus(self, path: Path | None = None) -> Path:
        """Export corpus JSON to file (useful for rebuilding build artifacts)."""
        out = path or CORPUS_CACHE
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(json.dumps(self.build_corpus_json(), indent=2))
        return out

    def get_filter_card(self, filter_id: str) -> str:
        """Get a compact one-line card for a filter."""
        info = self.get(filter_id)
        if not info:
            return f"[{filter_id}] NOT FOUND"
        return info.to_compact_card()

    def get_category_summary(self) -> str:
        """Get a human-readable summary of all categories and filter counts."""
        self._ensure_loaded()
        lines = []
        total = 0
        for cat, filters in sorted(self._by_category.items()):
            count = len(filters)
            total += count
            ids = ", ".join(f.id for f in filters)
            lines.append(f"  {cat} ({count}): {ids}")
        lines.insert(0, f"Ambara has {total} filters across {len(self._by_category)} categories:")
        return "\n".join(lines)
