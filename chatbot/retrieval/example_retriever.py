"""Semantic retriever for few-shot (query -> graph) examples."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import chromadb

from chatbot.retrieval.retriever import _embed_query


class ExampleRetriever:
    """Retrieve semantically similar few-shot examples from ChromaDB."""

    def __init__(self, chroma_path: str, examples_path: str) -> None:
        """Create example retriever.

        Args:
            chroma_path: Chroma database path.
            examples_path: Path to examples JSON.

        Returns:
            None.

        Raises:
            OSError: If examples file cannot be read.
        """
        self.client = chromadb.PersistentClient(path=chroma_path)
        self.collection = self.client.get_collection("ambara_examples")
        self.examples_path = Path(examples_path)
        self.examples: list[dict[str, Any]] = json.loads(self.examples_path.read_text())

    def retrieve_examples(self, query: str, top_k: int = 3) -> list[dict[str, Any]]:
        """Retrieve similar examples for few-shot prompting.

        Args:
            query: User query.
            top_k: Result count.

        Returns:
            List of examples.

        Raises:
            ValueError: If top_k is invalid.
        """
        if top_k <= 0:
            raise ValueError("top_k must be > 0")

        embedding = _embed_query(query)
        result = self.collection.query(query_embeddings=[embedding], n_results=top_k)
        ids = result.get("ids", [[]])[0]

        out: list[dict[str, Any]] = []
        for ex_id in ids:
            if ex_id.startswith("example_"):
                idx = int(ex_id.split("_")[1])
                if 0 <= idx < len(self.examples):
                    out.append(self.examples[idx])
        return out
