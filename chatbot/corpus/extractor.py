"""Build filter corpus from Ambara CLI JSON output with source fallback."""

from __future__ import annotations

import json
import re
import subprocess
from pathlib import Path

from chatbot.models import FilterDoc

ROOT = Path(__file__).resolve().parents[2]
BUILD_DIR = ROOT / "build"
OUT_PATH = BUILD_DIR / "filter_corpus.json"


def _run_cli_list_json() -> list[dict]:
    """Run `cargo run -- list --json` and parse result.

    Args:
        None.

    Returns:
        List of filter dictionaries.

    Raises:
        subprocess.CalledProcessError: If command execution fails.
        json.JSONDecodeError: If output is not valid JSON.
    """
    cmd = ["cargo", "run", "--", "list", "--json"]
    proc = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, check=False)
    if proc.returncode != 0:
        raise subprocess.CalledProcessError(proc.returncode, cmd, proc.stdout, proc.stderr)

    text = proc.stdout.strip()
    if not text:
        return []
    return json.loads(text)


def _fallback_extract_from_source() -> list[dict]:
    """Extract metadata from builtin filter source files.

    Args:
        None.

    Returns:
        Best-effort filter corpus entries.

    Raises:
        OSError: If source files cannot be read.
    """
    builtin_dir = ROOT / "src" / "filters" / "builtin"
    pattern = re.compile(r'NodeMetadata::builder\("([a-zA-Z0-9_]+)",\s*"([^"]+)"\)')
    desc_pattern = re.compile(r"\\.description\(\"([^\"]+)\"\)")

    docs: list[dict] = []
    for src in sorted(builtin_dir.glob("*.rs")):
        text = src.read_text(errors="replace")
        for match in pattern.finditer(text):
            filter_id = match.group(1)
            name = match.group(2)
            tail = text[match.end() : match.end() + 2000]
            desc_match = desc_pattern.search(tail)
            description = desc_match.group(1) if desc_match else f"{name} filter"
            docs.append(
                FilterDoc(
                    id=filter_id,
                    name=name,
                    description=description,
                    category=src.stem.capitalize(),
                    input_ports=[],
                    output_ports=[],
                    parameters=[],
                    tags=[src.stem],
                    examples=[],
                ).model_dump()
            )
    return docs


def extract_filter_docs() -> list[dict]:
    """Extract filter docs from CLI or source fallback.

    Args:
        None.

    Returns:
        Normalized filter documentation list.

    Raises:
        OSError: If output file cannot be written.
    """
    try:
        data = _run_cli_list_json()
    except (subprocess.CalledProcessError, json.JSONDecodeError):
        data = _fallback_extract_from_source()

    normalized: list[dict] = []
    for item in data:
        normalized.append(
            FilterDoc(
                id=item.get("id", ""),
                name=item.get("name", item.get("id", "Unknown")),
                description=item.get("description", ""),
                category=item.get("category", "Custom"),
                input_ports=item.get("input_ports", item.get("inputs", [])),
                output_ports=item.get("output_ports", item.get("outputs", [])),
                parameters=item.get("parameters", []),
                tags=item.get("tags", []),
                examples=item.get("examples", []),
            ).model_dump()
        )

    BUILD_DIR.mkdir(parents=True, exist_ok=True)
    OUT_PATH.write_text(json.dumps(normalized, indent=2))
    return normalized


if __name__ == "__main__":
    corpus = extract_filter_docs()
    print(f"[A-02 OK] {len(corpus)} filters extracted")
