"""Corpus schema validator for extracted Ambara filter documentation."""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CORPUS_PATH = ROOT / "build" / "filter_corpus.json"
ID_SET_PATH = ROOT / "build" / "filter_id_set.json"


def validate_corpus(corpus: list[dict]) -> list[str]:
    """Validate corpus-level constraints.

    Args:
        corpus: Filter documents loaded from JSON.

    Returns:
        List of validation errors.

    Raises:
        TypeError: If corpus entries are malformed.
    """
    errors: list[str] = []
    ids: set[str] = set()

    for idx, item in enumerate(corpus):
        for field in ["id", "name", "description"]:
            if not item.get(field):
                errors.append(f"item[{idx}] missing required field '{field}'")

        item_id = str(item.get("id", ""))
        if item_id in ids:
            errors.append(f"duplicate filter id: {item_id}")
        ids.add(item_id)

        for key in ["input_ports", "output_ports"]:
            for port_idx, port in enumerate(item.get(key, [])):
                if not port.get("name"):
                    errors.append(f"{item_id}:{key}[{port_idx}] missing name")
                if not port.get("type") and not port.get("port_type"):
                    errors.append(f"{item_id}:{key}[{port_idx}] missing type")

    return errors


def main() -> int:
    """CLI entrypoint for corpus schema validation.

    Args:
        None.

    Returns:
        Exit code.

    Raises:
        OSError: If files cannot be read or written.
    """
    if not CORPUS_PATH.exists():
        print("[A-04 FAIL] build/filter_corpus.json not found")
        return 1

    corpus = json.loads(CORPUS_PATH.read_text())
    errors = validate_corpus(corpus)

    if errors:
        print("[A-04 FAIL]")
        for err in errors:
            print(f"  - {err}")
        return 1

    ids = [item["id"] for item in corpus]
    ID_SET_PATH.write_text(json.dumps(ids, indent=2))
    print(f"[A-04 VALID] {len(ids)} filters")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
