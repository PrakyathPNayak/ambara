"""Shared Pydantic models used by the Ambara chatbot backend."""

from __future__ import annotations

from typing import Any

from pydantic import BaseModel, ConfigDict, Field


class FilterDoc(BaseModel):
    """Documented filter metadata used by retrieval and generation.

    Args:
        id: Filter identifier.
        name: Display name.
        description: Filter description.
        category: Category label.
        input_ports: Input port definitions.
        output_ports: Output port definitions.
        parameters: Parameter definitions.
        tags: Search tags.
        examples: Optional usage examples.

    Returns:
        FilterDoc model instance.

    Raises:
        pydantic.ValidationError: If values fail schema checks.
    """

    model_config = ConfigDict(extra="forbid")

    id: str
    name: str
    description: str
    category: str = "Custom"
    input_ports: list[dict[str, Any]] = Field(default_factory=list)
    output_ports: list[dict[str, Any]] = Field(default_factory=list)
    parameters: list[dict[str, Any]] = Field(default_factory=list)
    tags: list[str] = Field(default_factory=list)
    examples: list[str] = Field(default_factory=list)


class ExampleDoc(BaseModel):
    """Few-shot example used by example retriever."""

    model_config = ConfigDict(extra="forbid")

    query: str
    graph: dict[str, Any]
    description: str


class ValidationResultModel(BaseModel):
    """Validation result payload."""

    model_config = ConfigDict(extra="forbid")

    valid: bool
    errors: list[str] = Field(default_factory=list)


class GenerationResultModel(BaseModel):
    """Graph generation output payload."""

    model_config = ConfigDict(extra="forbid")

    graph: dict[str, Any] | None = None
    valid: bool
    errors: list[str] = Field(default_factory=list)
    retries: int = 0
    retrieved_filters: list[str] = Field(default_factory=list)
    llm_response_raw: str = ""
    explanation: str = ""
