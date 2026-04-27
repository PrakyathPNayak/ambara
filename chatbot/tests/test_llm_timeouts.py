"""Tests for `_resolve_ollama_timeout` env-var configuration.

Ollama runs locally and is the default backend when no API keys are
configured. CPU-only models can take well over 60s for the first
token, so the helper allows `OLLAMA_TIMEOUT_S` to override the default.
"""

from __future__ import annotations

from unittest.mock import patch

from chatbot.generation import llm_client
from chatbot.generation.llm_client import (
    _OLLAMA_DEFAULT_TIMEOUT_S,
    _resolve_ollama_timeout,
)


def test_resolve_ollama_timeout_default_when_unset(monkeypatch) -> None:
    monkeypatch.delenv("OLLAMA_TIMEOUT_S", raising=False)
    assert _resolve_ollama_timeout() == _OLLAMA_DEFAULT_TIMEOUT_S


def test_resolve_ollama_timeout_default_when_blank(monkeypatch) -> None:
    monkeypatch.setenv("OLLAMA_TIMEOUT_S", "   ")
    assert _resolve_ollama_timeout() == _OLLAMA_DEFAULT_TIMEOUT_S


def test_resolve_ollama_timeout_honors_positive_int(monkeypatch) -> None:
    monkeypatch.setenv("OLLAMA_TIMEOUT_S", "300")
    assert _resolve_ollama_timeout() == 300


def test_resolve_ollama_timeout_rejects_non_integer(monkeypatch) -> None:
    monkeypatch.setenv("OLLAMA_TIMEOUT_S", "not-a-number")
    with patch.object(llm_client.LOGGER, "warning") as warn:
        result = _resolve_ollama_timeout()
    assert result == _OLLAMA_DEFAULT_TIMEOUT_S
    warn.assert_called_once()
    assert "non-integer" in warn.call_args.args[0]


def test_resolve_ollama_timeout_rejects_zero(monkeypatch) -> None:
    monkeypatch.setenv("OLLAMA_TIMEOUT_S", "0")
    with patch.object(llm_client.LOGGER, "warning") as warn:
        result = _resolve_ollama_timeout()
    assert result == _OLLAMA_DEFAULT_TIMEOUT_S
    warn.assert_called_once()
    assert "non-positive" in warn.call_args.args[0]


def test_resolve_ollama_timeout_rejects_negative(monkeypatch) -> None:
    monkeypatch.setenv("OLLAMA_TIMEOUT_S", "-30")
    with patch.object(llm_client.LOGGER, "warning") as warn:
        result = _resolve_ollama_timeout()
    assert result == _OLLAMA_DEFAULT_TIMEOUT_S
    warn.assert_called_once()


def test_default_is_higher_than_paid_provider_timeout() -> None:
    # Local CPU inference is allowed more time than paid network APIs
    # because cold-start latency on a 7-8B model can exceed 60s.
    from chatbot.generation.llm_client import _PAID_PROVIDER_TIMEOUT_S
    assert _OLLAMA_DEFAULT_TIMEOUT_S > _PAID_PROVIDER_TIMEOUT_S
