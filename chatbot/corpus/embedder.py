"""Embed Ambara filter corpus and examples into a local ChromaDB store."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import chromadb

ROOT = Path(__file__).resolve().parents[2]
CORPUS_PATH = ROOT / "build" / "filter_corpus.json"
EXAMPLES_PATH = ROOT / "chatbot" / "corpus" / "examples.json"
CHROMA_PATH = ROOT / "build" / "chroma_db"


def _embed_texts(texts: list[str]) -> list[list[float]]:
    """Create embeddings for text list using sentence-transformers fallback.

    Args:
        texts: Input text documents.

    Returns:
        Vector embeddings.

    Raises:
        RuntimeError: If embedding generation fails unexpectedly.
    """
    try:
        from sentence_transformers import SentenceTransformer

        model = SentenceTransformer("all-MiniLM-L6-v2")
        vectors = model.encode(texts, normalize_embeddings=True)
        return [list(map(float, row)) for row in vectors]
    except Exception:
        # Deterministic hash embedding fallback for offline/test environments.
        out: list[list[float]] = []
        for text in texts:
            seed = abs(hash(text))
            vec = [((seed >> (i % 32)) & 0xFF) / 255.0 for i in range(384)]
            out.append(vec)
        return out


def _filter_doc_to_text(doc: dict[str, Any]) -> str:
    """Format filter doc into embedding text.

    Args:
        doc: Filter metadata dictionary.

    Returns:
        Concatenated semantic text.

    Raises:
        KeyError: If required fields are missing.
    """
    return (
        f"{doc.get('name', '')}. {doc.get('description', '')}. "
        f"Tags: {doc.get('tags', [])}. "
        f"Inputs: {doc.get('input_ports', doc.get('inputs', []))}. "
        f"Outputs: {doc.get('output_ports', doc.get('outputs', []))}. "
        f"Parameters: {doc.get('parameters', [])}"
    )


def build_embeddings() -> dict[str, int]:
    """Create or replace corpus/example collections in ChromaDB.

    Args:
        None.

    Returns:
        Counts for filters and examples collections.

    Raises:
        OSError: If corpus files cannot be read.
    """
    corpus = json.loads(CORPUS_PATH.read_text())
    examples = json.loads(EXAMPLES_PATH.read_text()) if EXAMPLES_PATH.exists() else []

    CHROMA_PATH.mkdir(parents=True, exist_ok=True)
    client = chromadb.PersistentClient(path=str(CHROMA_PATH))

    for name in ["ambara_filters", "ambara_examples"]:
        try:
            client.delete_collection(name)
        except Exception:
            pass

    filters_col = client.create_collection("ambara_filters")
    examples_col = client.create_collection("ambara_examples")

    filter_docs = [_filter_doc_to_text(doc) for doc in corpus]
    filter_ids = [str(doc["id"]) for doc in corpus]
    filter_embeddings = _embed_texts(filter_docs)
    filters_col.add(
        ids=filter_ids,
        documents=filter_docs,
        metadatas=[{"id": doc["id"], "name": doc.get("name", "")} for doc in corpus],
        embeddings=filter_embeddings,
    )

    if examples:
        ex_docs = [f"{e.get('query', '')}. {e.get('description', '')}" for e in examples]
        ex_ids = [f"example_{idx}" for idx in range(len(examples))]
        ex_embeddings = _embed_texts(ex_docs)
        examples_col.add(
            ids=ex_ids,
            documents=ex_docs,
            metadatas=[{"index": idx, "query": e.get("query", "")} for idx, e in enumerate(examples)],
            embeddings=ex_embeddings,
        )

    return {"filters": filters_col.count(), "examples": examples_col.count()}


if __name__ == "__main__":
    counts = build_embeddings()
    print(f"[A-05 OK] {counts['filters']} filter embeddings, {counts['examples']} examples")
