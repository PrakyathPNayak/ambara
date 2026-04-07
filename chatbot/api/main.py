"""FastAPI app exposing Ambara chatbot and graph generation endpoints."""

from __future__ import annotations

import asyncio
import json
import logging
import os
import subprocess
import tempfile
from collections.abc import AsyncIterator
from contextlib import asynccontextmanager
from pathlib import Path
from typing import Any

from fastapi import FastAPI, Query, WebSocket, WebSocketDisconnect
from fastapi.middleware.cors import CORSMiddleware

from chatbot.api.models import (
    ChatRequest,
    ChatResponse,
    FilterItemModel,
    GraphExecuteRequest,
    GraphExecuteResult,
    GraphGenerateRequest,
    GraphValidationRequest,
    GraphValidationResult,
    HealthResponse,
    LLMConfigRequest,
    LLMConfigResponse,
)
from chatbot.api.session import SessionStore
from chatbot.generation.agent import Agent
from chatbot.generation.graph_generator import GraphGenerator
from chatbot.generation.graph_validator import GraphValidator
from chatbot.generation.llm_client import LLMClient
from chatbot.models import GenerationResultModel
from chatbot.retrieval.code_retriever import CodeRetriever

ROOT = Path(__file__).resolve().parents[2]
LOGGER = logging.getLogger(__name__)
logging.basicConfig(level=logging.INFO)

CORPUS_PATH = ROOT / "build" / "filter_corpus.json"
EXAMPLES_PATH = ROOT / "chatbot" / "corpus" / "examples.json"
SCHEMA_PATH = ROOT / "chatbot" / "corpus" / "graph_schema.json"
FILTER_IDS_PATH = ROOT / "build" / "filter_id_set.json"


sessions = SessionStore()
chat_llm = LLMClient(force_mock=False)
code_retriever = CodeRetriever()

# Startup health tracking — set during lifespan, read by /health
_startup_status: str = "starting"
_startup_error: str = ""


def _ensure_corpus() -> None:
    """Ensure filter corpus and supporting artifacts are built from source code.

    Uses CodeRetriever to parse Rust source files directly (code-as-RAG),
    then exports corpus JSON for components that still need it.
    """
    code_retriever.refresh()

    # Export corpus JSON for graph validator and other components
    if not CORPUS_PATH.exists() or _corpus_stale():
        code_retriever.export_corpus(CORPUS_PATH)

    corpus = json.loads(CORPUS_PATH.read_text())
    if not FILTER_IDS_PATH.exists():
        FILTER_IDS_PATH.write_text(json.dumps([item["id"] for item in corpus], indent=2))


@asynccontextmanager
async def _lifespan(application: FastAPI) -> AsyncIterator[None]:
    """Application lifespan: initialise corpus on startup, cleanup on shutdown."""
    global _startup_status, _startup_error
    try:
        _ensure_corpus()
        filter_count = len(code_retriever.all_filter_ids)
        _startup_status = "ok"
        LOGGER.info(
            "Startup complete: %d filters loaded, LLM backend=%s/%s",
            filter_count,
            chat_llm.backend,
            chat_llm.model_name,
        )
    except Exception as exc:
        _startup_status = "degraded"
        _startup_error = f"{type(exc).__name__}: {exc}"
        LOGGER.error("Corpus initialisation failed — API starting in degraded mode", exc_info=True)
    yield


app = FastAPI(title="Ambara Chatbot API", version="0.9.0", lifespan=_lifespan)

_cors_origins = os.environ.get("AMBARA_CORS_ORIGINS", "*").split(",")
app.add_middleware(
    CORSMiddleware,
    allow_origins=_cors_origins,
    allow_methods=["*"],
    allow_headers=["*"],
)


def _corpus_stale() -> bool:
    """Check if corpus JSON is older than any Rust source file."""
    if not CORPUS_PATH.exists():
        return True
    corpus_mtime = CORPUS_PATH.stat().st_mtime
    src_dir = ROOT / "src" / "filters" / "builtin"
    for rs_file in src_dir.glob("*.rs"):
        if rs_file.name == "mod.rs":
            continue
        if rs_file.stat().st_mtime > corpus_mtime:
            return True
    return False


def _generator() -> GraphGenerator:
    """Create graph generator instance using CodeRetriever."""
    return GraphGenerator(
        code_retriever=code_retriever,
        corpus_path=str(CORPUS_PATH),
        examples_path=str(EXAMPLES_PATH),
        llm_client=chat_llm,
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


@app.post("/chat", response_model=ChatResponse)
def chat(req: ChatRequest) -> ChatResponse:
    """Handle conversational chat via the agentic router.

    The agent decides whether to explain, search, or generate a graph
    based on the user's message and conversation history.
    """
    sessions.append_message(req.session_id, {"role": "user", "content": req.message})
    history = sessions.get_history(req.session_id)

    agent = Agent(
        llm_client=chat_llm,
        retriever=code_retriever,
        generator=_generator(),
    )

    # Pre-set input image path if provided via attachment
    if req.image_paths:
        for img_path in req.image_paths:
            agent.tool_executor.execute("set_input_image", {"path": img_path})

    result = agent.run(req.message, session_history=history)

    sessions.append_message(req.session_id, {"role": "assistant", "content": result.reply})
    return ChatResponse(
        reply=result.reply,
        session_id=req.session_id,
        graph_generated=result.graph_generated,
        graph=result.graph,
    )


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

    try:
        cmd = ["cargo", "run", "--", "load-graph", str(path), "--execute"]
        proc = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, check=False, timeout=120)
    except subprocess.TimeoutExpired:
        return GraphExecuteResult(success=False, output_paths=[], errors=["Graph execution timed out after 120s"])
    finally:
        path.unlink(missing_ok=True)

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


@app.get("/filters", response_model=list[FilterItemModel])
def filters() -> list[dict[str, Any]]:
    """Return full filter corpus from code-as-RAG retriever."""
    _ensure_corpus()
    return code_retriever.build_corpus_json()


@app.get("/filters/search", response_model=list[FilterItemModel])
def filters_search(q: str = Query(..., max_length=500), top_k: int = Query(5, ge=1, le=50)) -> list[dict[str, Any]]:
    """Search filters by keyword using code-as-RAG retriever."""
    _ensure_corpus()
    results = code_retriever.search(q, top_k=top_k)
    return [r.to_dict() for r in results]


@app.get("/health", response_model=HealthResponse)
def health() -> HealthResponse:
    """Service health and readiness endpoint."""
    filters_count = len(code_retriever.all_filter_ids) if _startup_status == "ok" else 0
    return HealthResponse(
        status=_startup_status,
        filters_loaded=filters_count,
        chroma_ready=True,  # No longer needed but kept for API compat
        llm_backend=chat_llm.backend,
        llm_model=chat_llm.model_name,
        error=_startup_error or None,
    )


@app.get("/llm/config", response_model=LLMConfigResponse)
def get_llm_config() -> LLMConfigResponse:
    """Return current LLM configuration.

    Args:
        None.

    Returns:
        Current LLM config.
    """
    return LLMConfigResponse(
        provider=chat_llm.backend,
        model=chat_llm.model_name,
        api_url=chat_llm.ollama_url if chat_llm.backend == "ollama" else "",
    )


@app.put("/llm/config", response_model=LLMConfigResponse)
def update_llm_config(req: LLMConfigRequest) -> LLMConfigResponse:
    """Update LLM configuration at runtime.

    Args:
        req: New LLM config values.

    Returns:
        Updated LLM config.
    """
    global chat_llm
    if req.provider:
        chat_llm.backend = req.provider
    if req.model:
        chat_llm.model_name = req.model
    if req.api_url and chat_llm.backend == "ollama":
        chat_llm.ollama_url = req.api_url
    if req.api_key:
        if chat_llm.backend == "openai":
            chat_llm.openai_key = req.api_key
        elif chat_llm.backend == "anthropic":
            chat_llm.anthropic_key = req.api_key
        elif chat_llm.backend == "groq":
            chat_llm.groq_key = req.api_key
        LOGGER.info("API key updated for backend: %s", chat_llm.backend)
    return LLMConfigResponse(
        provider=chat_llm.backend,
        model=chat_llm.model_name,
        api_url=chat_llm.ollama_url if chat_llm.backend == "ollama" else "",
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
            if not msg.strip():
                await ws.send_json({"type": "done", "graph": None, "graph_generated": False})
                continue

            # Parse message — may be plain text or JSON with image_paths
            image_paths: list[str] = []
            text_msg = msg
            try:
                parsed_msg = json.loads(msg)
                if isinstance(parsed_msg, dict):
                    text_msg = parsed_msg.get("message", msg)
                    image_paths = parsed_msg.get("image_paths", [])
            except (json.JSONDecodeError, ValueError):
                pass

            try:
                response = await asyncio.wait_for(
                    asyncio.to_thread(
                        chat, ChatRequest(message=text_msg, session_id=session_id, context=[], image_paths=image_paths)
                    ),
                    timeout=120.0,
                )
            except asyncio.TimeoutError:
                LOGGER.error("WebSocket chat timeout for session %s", session_id)
                await ws.send_json({"type": "token", "content": "Sorry, the request timed out. Please try a simpler query. "})
                await ws.send_json({"type": "done", "graph": None, "graph_generated": False})
                continue
            except Exception as exc:
                LOGGER.error("WebSocket chat error for session %s: %s", session_id, exc)
                await ws.send_json({"type": "token", "content": f"Error: {exc} "})
                await ws.send_json({"type": "done", "graph": None, "graph_generated": False})
                continue
            for token in response.reply.split(" "):
                await ws.send_json({"type": "token", "content": f"{token} "})
                await asyncio.sleep(0.01)
            await ws.send_json({"type": "done", "graph": response.graph, "graph_generated": response.graph_generated})
    except WebSocketDisconnect:
        LOGGER.info("WebSocket disconnected: %s", session_id)
