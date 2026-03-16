"""Tests for chatbot corpus extraction, validation, and embeddings."""

from __future__ import annotations

import json
from pathlib import Path

from chatbot.corpus.embedder import build_embeddings
from chatbot.corpus.extractor import extract_filter_docs
from chatbot.corpus.schema_validator import validate_corpus

ROOT = Path(__file__).resolve().parents[2]


def test_extraction_and_corpus_file() -> None:
    docs = extract_filter_docs()
    assert len(docs) > 5
    assert (ROOT / "build" / "filter_corpus.json").exists()


def test_schema_validation_and_id_set() -> None:
    corpus_path = ROOT / "build" / "filter_corpus.json"
    corpus = json.loads(corpus_path.read_text())
    errors = validate_corpus(corpus)
    assert errors == []


def test_embeddings_creation() -> None:
    counts = build_embeddings()
    assert counts["filters"] > 5


def test_graph_schema_loads() -> None:
    schema_path = ROOT / "chatbot" / "corpus" / "graph_schema.json"
    schema = json.loads(schema_path.read_text())
    assert schema.get("type") == "object"
