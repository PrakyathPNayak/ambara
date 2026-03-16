"""Tests for filter and example retrieval modules."""

from __future__ import annotations

from chatbot.corpus.embedder import build_embeddings
from chatbot.retrieval.example_retriever import ExampleRetriever
from chatbot.retrieval.retriever import FilterRetriever


CHROMA_PATH = "build/chroma_db"
CORPUS_PATH = "build/filter_corpus.json"
EXAMPLES_PATH = "chatbot/corpus/examples.json"


def setup_module() -> None:
    build_embeddings()


def test_basic_retrieval() -> None:
    retriever = FilterRetriever(CHROMA_PATH, CORPUS_PATH)
    results = retriever.retrieve("gaussian blur")
    assert len(results) > 0


def test_empty_query_returns_defaults() -> None:
    retriever = FilterRetriever(CHROMA_PATH, CORPUS_PATH)
    results = retriever.retrieve("")
    assert len(results) > 0


def test_context_aware_retrieval() -> None:
    retriever = FilterRetriever(CHROMA_PATH, CORPUS_PATH)
    partial = {"nodes": [{"filter_id": "load_image"}], "connections": []}
    results = retriever.retrieve_with_graph_context("apply blur", partial_graph=partial)
    assert len(results) > 0


def test_example_retrieval() -> None:
    examples = ExampleRetriever(CHROMA_PATH, EXAMPLES_PATH)
    result = examples.retrieve_examples("make the image blurry then save it")
    assert len(result) > 0
