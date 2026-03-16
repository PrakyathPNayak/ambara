"""ChromaDB-based filter retriever for Ambara chatbot queries."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import chromadb

ROOT = Path(__file__).resolve().parents[2]
_ST_MODEL = None


def _hash_embed(text: str) -> list[float]:
    """Create deterministic fallback embedding from hash.

    Args:
        text: Query text.

    Returns:
        384-d pseudo embedding.

    Raises:
        RuntimeError: Never raised in normal operation.
    """
    seed = abs(hash(text))
    return [((seed >> (i % 32)) & 0xFF) / 255.0 for i in range(384)]


def _get_model():
    """Load sentence-transformer model once per process.

    Args:
        None.

    Returns:
        SentenceTransformer instance or None if unavailable.

    Raises:
        RuntimeError: Never raised in normal operation.
    """
    global _ST_MODEL  # pylint: disable=global-statement
    if _ST_MODEL is not None:
        return _ST_MODEL
    try:
        from sentence_transformers import SentenceTransformer

        _ST_MODEL = SentenceTransformer("all-MiniLM-L6-v2")
        return _ST_MODEL
    except Exception:
        _ST_MODEL = False
        return None


def _embed_query(text: str) -> list[float]:
    """Embed a query using sentence-transformers or hash fallback.

    Args:
        text: Query text.

    Returns:
        Embedding vector.

    Raises:
        RuntimeError: If embedding fails unexpectedly.
    """
    model = _get_model()
    if model is None:
        return _hash_embed(text)

    try:
        vector = model.encode([text], normalize_embeddings=True)[0]
        return [float(v) for v in vector]
    except Exception:
        return _hash_embed(text)


class FilterRetriever:
    """Retrieve filter docs by semantic similarity with context-aware ranking."""

    def __init__(self, chroma_path: str, corpus_path: str | None = None) -> None:
        """Initialize retriever with ChromaDB path and corpus.

        Args:
            chroma_path: Path to persistent chroma database.
            corpus_path: Optional corpus JSON path.

        Returns:
            None.

        Raises:
            OSError: If corpus file cannot be read.
        """
        self.client = chromadb.PersistentClient(path=chroma_path)
        self.collection = self.client.get_collection("ambara_filters")
        corpus_file = Path(corpus_path) if corpus_path else ROOT / "build" / "filter_corpus.json"
        self.corpus: list[dict[str, Any]] = json.loads(corpus_file.read_text()) if corpus_file.exists() else []
        self.by_id = {doc["id"]: doc for doc in self.corpus}

    def retrieve(self, query: str, top_k: int = 5) -> list[dict[str, Any]]:
        """Retrieve top-k relevant filters.

        Args:
            query: User query.
            top_k: Maximum result count.

        Returns:
            List of filter docs.

        Raises:
            ValueError: If top_k is invalid.
        """
        if top_k <= 0:
            raise ValueError("top_k must be > 0")

        if not query.strip():
            return self.corpus[:top_k]

        query_embedding = _embed_query(query)
        result = self.collection.query(query_embeddings=[query_embedding], n_results=top_k)
        ids = result.get("ids", [[]])[0]
        docs = [self.by_id[i] for i in ids if i in self.by_id]

        if docs:
            return docs

        lowered = query.lower()
        fallback = [
            doc for doc in self.corpus if lowered in doc.get("id", "").lower() or lowered in doc.get("name", "").lower()
        ]
        return fallback[:top_k]

    def retrieve_with_graph_context(self, query: str, partial_graph: dict[str, Any], top_k: int = 5) -> list[dict[str, Any]]:
        """Retrieve filters with compatibility preference for next chain step.

        Args:
            query: User query.
            partial_graph: Current partially built graph.
            top_k: Maximum result count.

        Returns:
            Ranked list of filters.

        Raises:
            TypeError: If partial graph has invalid shape.
        """
        base = self.retrieve(query, max(top_k * 3, 10))
        nodes = partial_graph.get("nodes", []) if isinstance(partial_graph, dict) else []

        if not nodes:
            return base[:top_k]

        last_filter_id = nodes[-1].get("filter_id") if isinstance(nodes[-1], dict) else None
        last_doc = self.by_id.get(last_filter_id, {})
        last_outputs = last_doc.get("output_ports", last_doc.get("outputs", []))
        output_types = {
            str(port.get("type") or port.get("port_type") or "Any")
            for port in last_outputs
            if isinstance(port, dict)
        }

        def score(doc: dict[str, Any]) -> int:
            inputs = doc.get("input_ports", doc.get("inputs", []))
            input_types = {
                str(port.get("type") or port.get("port_type") or "Any")
                for port in inputs
                if isinstance(port, dict)
            }
            if "Any" in input_types or "Any" in output_types:
                return 1
            return 2 if output_types.intersection(input_types) else 0

        ranked = sorted(base, key=score, reverse=True)
        return ranked[:top_k]
