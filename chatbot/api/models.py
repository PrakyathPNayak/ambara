"""Pydantic request and response models for chatbot FastAPI endpoints."""

from __future__ import annotations

from typing import Any, Literal

from pydantic import BaseModel, ConfigDict, Field


class ChatMessage(BaseModel):
    """Single chat message in request/response history."""

    model_config = ConfigDict(extra="forbid")

    role: Literal["user", "assistant", "system"]
    content: str
    timestamp: str | None = None


class ChatRequest(BaseModel):
    """Request payload for /chat endpoint."""

    model_config = ConfigDict(extra="forbid")

    message: str
    session_id: str
    context: list[ChatMessage] = Field(default_factory=list)


class ChatResponse(BaseModel):
    """Response payload for /chat endpoint."""

    model_config = ConfigDict(extra="forbid")

    reply: str
    session_id: str
    graph_generated: bool
    graph: dict[str, Any] | None = None


class GraphGenerateRequest(BaseModel):
    """Request payload for /graph/generate endpoint."""

    model_config = ConfigDict(extra="forbid")

    query: str
    partial_graph: dict[str, Any] | None = None


class GraphValidationRequest(BaseModel):
    """Request payload for /graph/validate endpoint."""

    model_config = ConfigDict(extra="forbid")

    graph: dict[str, Any]


class GraphValidationResult(BaseModel):
    """Validation result payload."""

    model_config = ConfigDict(extra="forbid")

    valid: bool
    errors: list[str] = Field(default_factory=list)


class GraphExecuteRequest(BaseModel):
    """Execution request payload."""

    model_config = ConfigDict(extra="forbid")

    graph: dict[str, Any]


class GraphExecuteResult(BaseModel):
    """Execution result payload."""

    model_config = ConfigDict(extra="forbid")

    success: bool
    output_paths: list[str] = Field(default_factory=list)
    errors: list[str] = Field(default_factory=list)


class LLMConfigRequest(BaseModel):
    """Request payload for /llm/config endpoint."""

    model_config = ConfigDict(extra="forbid")

    provider: str | None = None
    model: str | None = None
    api_key: str | None = None
    api_url: str | None = None


class LLMConfigResponse(BaseModel):
    """Response payload for /llm/config endpoint."""

    model_config = ConfigDict(extra="forbid")

    provider: str
    model: str
    api_url: str


class HealthResponse(BaseModel):
    """Health endpoint response payload."""

    model_config = ConfigDict(extra="forbid")

    status: str
    filters_loaded: int
    chroma_ready: bool
    llm_backend: str
    llm_model: str
