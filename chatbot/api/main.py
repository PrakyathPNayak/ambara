"""FastAPI app exposing Ambara chatbot and graph generation endpoints."""

from __future__ import annotations

import asyncio
import json
import logging
import subprocess
import tempfile
from pathlib import Path
from typing import Any

from fastapi import FastAPI, Query, WebSocket, WebSocketDisconnect

from chatbot.api.intent_classifier import IntentClassifier
from chatbot.api.models import (
    ChatRequest,
    ChatResponse,
    GraphExecuteRequest,
    GraphExecuteResult,
    GraphGenerateRequest,
    GraphValidationRequest,
    GraphValidationResult,
    HealthResponse,
)
from chatbot.api.session import SessionStore
from chatbot.corpus.embedder import build_embeddings
from chatbot.corpus.extractor import extract_filter_docs
from chatbot.corpus.schema_validator import validate_corpus
from chatbot.generation.graph_generator import GraphGenerator
from chatbot.generation.graph_validator import GraphValidator
from chatbot.generation.llm_client import LLMClient
from chatbot.models import GenerationResultModel
from chatbot.retrieval.retriever import FilterRetriever

ROOT = Path(__file__).resolve().parents[2]
LOGGER = logging.getLogger(__name__)
logging.basicConfig(level=logging.INFO)

CORPUS_PATH = ROOT / "build" / "filter_corpus.json"
CHROMA_PATH = ROOT / "build" / "chroma_db"
EXAMPLES_PATH = ROOT / "chatbot" / "corpus" / "examples.json"
SCHEMA_PATH = ROOT / "chatbot" / "corpus" / "graph_schema.json"
FILTER_IDS_PATH = ROOT / "build" / "filter_id_set.json"


app = FastAPI(title="Ambara Chatbot API", version="0.4.0")
sessions = SessionStore()
classifier = IntentClassifier()
chat_llm = LLMClient(force_mock=True)


def _ensure_corpus() -> None:
    """Ensure corpus exists for API startup.

    Args:
        None.

    Returns:
        None.

    Raises:
        OSError: If corpus generation fails to write artifacts.
    """
    if not CORPUS_PATH.exists():
        extract_filter_docs()

    corpus = json.loads(CORPUS_PATH.read_text())
    errors = validate_corpus(corpus)
    if errors:
        raise RuntimeError(f"Corpus validation failed: {errors}")

    if not FILTER_IDS_PATH.exists():
        FILTER_IDS_PATH.write_text(json.dumps([item["id"] for item in corpus], indent=2))

    if not CHROMA_PATH.exists():
        build_embeddings()


def _generator() -> GraphGenerator:
    """Create graph generator instance.

    Args:
        None.

    Returns:
        Graph generator.

    Raises:
        OSError: If supporting files are missing.
    """
    return GraphGenerator(
        chroma_path=str(CHROMA_PATH),
        corpus_path=str(CORPUS_PATH),
        examples_path=str(EXAMPLES_PATH),
        force_mock_llm=True,
    )


def _validator() -> GraphValidator:
    """Create graph validator instance.

    Args:
        None.

    Returns:
        Graph validator.

    Raises:
        OSError: If supporting files are missing.
    """
    return GraphValidator(str(SCHEMA_PATH), str(FILTER_IDS_PATH), str(CORPUS_PATH))


@app.on_event("startup")
def on_startup() -> None:
    """Startup hook ensuring required artifacts are present."""
    _ensure_corpus()


@app.post("/chat", response_model=ChatResponse)
def chat(req: ChatRequest) -> ChatResponse:
    """Handle conversational chat and optional graph generation.

    Args:
        req: Chat request payload.

    Returns:
        Chat response payload.

    Raises:
        RuntimeError: If generation fails unexpectedly.
    """
    sessions.append_message(req.session_id, {"role": "user", "content": req.message})
    intent = classifier.classify(req.message)

    if intent in {"GRAPH_REQUEST", "CLARIFICATION"}:
        result = _generator().generate(req.message)
        if result.valid:
            reply = result.explanation or "Generated a graph for your request."
            sessions.append_message(req.session_id, {"role": "assistant", "content": reply})
            return ChatResponse(
                reply=reply,
                session_id=req.session_id,
                graph_generated=True,
                graph=result.graph,
            )
        reply = "I could not generate a valid graph yet. Please refine your request."
        sessions.append_message(req.session_id, {"role": "assistant", "content": reply})
        return ChatResponse(reply=reply, session_id=req.session_id, graph_generated=False, graph=None)

    if intent == "QUESTION":
        reply = "Ambara supports graph-based image processing with built-in and plugin filters."
    else:
        reply = "I can help build Ambara processing graphs. Ask for operations like blur, resize, or blend."

    sessions.append_message(req.session_id, {"role": "assistant", "content": reply})
    return ChatResponse(reply=reply, session_id=req.session_id, graph_generated=False, graph=None)


@app.post("/graph/generate", response_model=GenerationResultModel)
def graph_generate(req: GraphGenerateRequest) -> GenerationResultModel:
    """Generate graph JSON from query.

    Args:
        req: Graph generate request.

    Returns:
        Generation result model.

    Raises:
        RuntimeError: If generation setup fails.
    """
    return _generator().generate(req.query, partial_graph=req.partial_graph)


@app.post("/graph/validate", response_model=GraphValidationResult)
def graph_validate(req: GraphValidationRequest) -> GraphValidationResult:
    """Validate graph JSON against schema and filter IDs.

    Args:
        req: Graph validation request.

    Returns:
        Validation result.

    Raises:
        RuntimeError: If validator setup fails.
    """
    validator = _validator()
    result = validator.validate_all(json.dumps(req.graph))
    return GraphValidationResult(valid=result.valid, errors=result.errors)


@app.post("/graph/execute", response_model=GraphExecuteResult)
def graph_execute(req: GraphExecuteRequest) -> GraphExecuteResult:
    """Execute graph through Ambara CLI.

    Args:
        req: Graph execute request.

    Returns:
        Execution result.

    Raises:
        OSError: If temporary graph file operations fail.
    """
    with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as fp:
        path = Path(fp.name)
        fp.write(json.dumps(req.graph))

    cmd = ["cargo", "run", "--", "load-graph", str(path), "--execute"]
    proc = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, check=False)

    if proc.returncode != 0:
        return GraphExecuteResult(success=False, output_paths=[], errors=[proc.stderr or proc.stdout or "Execution failed"])

    try:
        payload = json.loads(proc.stdout)
        success = bool(payload.get("success", False))
        errors = [str(e) for e in payload.get("errors", [])]
    except json.JSONDecodeError:
        success = True
        errors = []

    return GraphExecuteResult(success=success, output_paths=[], errors=errors)


@app.get("/filters")
def filters() -> list[dict[str, Any]]:
    """Return full filter corpus.

    Args:
        None.

    Returns:
        List of filter docs.

    Raises:
        OSError: If corpus file is unavailable.
    """
    _ensure_corpus()
    return json.loads(CORPUS_PATH.read_text())


@app.get("/filters/search")
def filters_search(q: str = Query(...), top_k: int = Query(5)) -> list[dict[str, Any]]:
    """Search filters by semantic retrieval.

    Args:
        q: Query string.
        top_k: Maximum number of results.

    Returns:
        Matching filter docs.

    Raises:
        RuntimeError: If retrieval setup fails.
    """
    retriever = FilterRetriever(str(CHROMA_PATH), str(CORPUS_PATH))
    return retriever.retrieve(q, top_k=top_k)


@app.get("/health", response_model=HealthResponse)
def health() -> HealthResponse:
    """Service health and readiness endpoint.

    Args:
        None.

    Returns:
        Health response.

    Raises:
        OSError: If corpus cannot be read.
    """
    _ensure_corpus()
    filters_count = len(json.loads(CORPUS_PATH.read_text())) if CORPUS_PATH.exists() else 0
    chroma_ready = CHROMA_PATH.exists()
    return HealthResponse(
        status="ok",
        filters_loaded=filters_count,
        chroma_ready=chroma_ready,
        llm_backend=chat_llm.backend,
    )


@app.websocket("/ws/{session_id}")
async def websocket_chat(ws: WebSocket, session_id: str) -> None:
    """Stream tokenized assistant replies over websocket.

    Args:
        ws: Active websocket connection.
        session_id: Session identifier.

    Returns:
        None.

    Raises:
        WebSocketDisconnect: When client disconnects.
    """
    await ws.accept()
    try:
        while True:
            msg = await ws.receive_text()
            response = chat(ChatRequest(message=msg, session_id=session_id, context=[]))
            for token in response.reply.split(" "):
                await ws.send_json({"type": "token", "content": f"{token} "})
                await asyncio.sleep(0.01)
            await ws.send_json({"type": "done", "graph": response.graph, "graph_generated": response.graph_generated})
    except WebSocketDisconnect:
        LOGGER.info("WebSocket disconnected: %s", session_id)
