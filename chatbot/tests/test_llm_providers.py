"""Tests for `LLMClient` paid-provider response-shape handling.

Each `_generate_*` path: builds a request body, calls `_post_with_retry`,
checks the status code, and extracts the response payload's content
field. These tests pin the response-shape contract before refactor —
loop 18/19 covered the retry layer; this fills the gap above it.

Mocks `_post_with_retry` directly so we don't touch the network or
exercise retry behavior already covered elsewhere.
"""

from __future__ import annotations

from unittest.mock import MagicMock, patch

import pytest
import requests

from chatbot.generation.llm_client import LLMClient


def _resp(status: int, payload: dict | None = None, text: str = "") -> MagicMock:
    """Stand-in `requests.Response` for shape tests."""
    response = MagicMock(spec=requests.Response)
    response.status_code = status
    response.text = text
    response.json.return_value = payload or {}
    response.headers = {}
    return response


@pytest.fixture
def clean_env(monkeypatch):
    """Strip every backend env so we can install exactly one key per test."""
    for key in (
        "ANTHROPIC_API_KEY",
        "OPENAI_API_KEY",
        "GROQ_API_KEY",
        "AMBARA_FORCE_MOCK_LLM",
        "OLLAMA_URL",
    ):
        monkeypatch.delenv(key, raising=False)


# --- Backend selection ----------------------------------------------------------


def test_force_mock_overrides_keys(clean_env, monkeypatch) -> None:
    monkeypatch.setenv("ANTHROPIC_API_KEY", "sk-test")
    client = LLMClient(force_mock=True)
    assert client.backend == "mock"


def test_anthropic_selected_first(clean_env, monkeypatch) -> None:
    monkeypatch.setenv("ANTHROPIC_API_KEY", "sk-test")
    monkeypatch.setenv("OPENAI_API_KEY", "sk-other")
    client = LLMClient()
    assert client.backend == "anthropic"


def test_groq_beats_openai(clean_env, monkeypatch) -> None:
    monkeypatch.setenv("GROQ_API_KEY", "gsk-test")
    monkeypatch.setenv("OPENAI_API_KEY", "sk-other")
    client = LLMClient()
    assert client.backend == "groq"


def test_ollama_default_when_no_keys(clean_env) -> None:
    client = LLMClient()
    assert client.backend == "ollama"


# --- Anthropic ------------------------------------------------------------------


def test_anthropic_extracts_content_text(clean_env, monkeypatch) -> None:
    monkeypatch.setenv("ANTHROPIC_API_KEY", "sk-test")
    client = LLMClient()
    response = _resp(
        200,
        payload={"content": [{"type": "text", "text": "hello world"}]},
    )
    with patch.object(client, "_post_with_retry", return_value=response) as call:
        out = client.generate({"messages": [{"role": "user", "content": "hi"}]})
    assert out == "hello world"
    call.assert_called_once()


def test_anthropic_separates_system_from_user_messages(clean_env, monkeypatch) -> None:
    monkeypatch.setenv("ANTHROPIC_API_KEY", "sk-test")
    client = LLMClient()
    response = _resp(200, payload={"content": [{"type": "text", "text": "ok"}]})
    with patch.object(client, "_post_with_retry", return_value=response) as call:
        client.generate({
            "messages": [
                {"role": "system", "content": "be brief"},
                {"role": "user", "content": "hi"},
            ],
        })
    body = call.call_args.args[2]
    assert body["system"] == "be brief"
    assert body["messages"] == [{"role": "user", "content": "hi"}]


def test_anthropic_max_tokens_default(clean_env, monkeypatch) -> None:
    monkeypatch.setenv("ANTHROPIC_API_KEY", "sk-test")
    monkeypatch.delenv("ANTHROPIC_MAX_TOKENS", raising=False)
    client = LLMClient()
    response = _resp(200, payload={"content": [{"type": "text", "text": "ok"}]})
    with patch.object(client, "_post_with_retry", return_value=response) as call:
        client.generate({"messages": [{"role": "user", "content": "hi"}]})
    body = call.call_args.args[2]
    assert body["max_tokens"] == 4096


def test_anthropic_max_tokens_env_override(clean_env, monkeypatch) -> None:
    monkeypatch.setenv("ANTHROPIC_API_KEY", "sk-test")
    monkeypatch.setenv("ANTHROPIC_MAX_TOKENS", "8192")
    client = LLMClient()
    response = _resp(200, payload={"content": [{"type": "text", "text": "ok"}]})
    with patch.object(client, "_post_with_retry", return_value=response) as call:
        client.generate({"messages": [{"role": "user", "content": "hi"}]})
    body = call.call_args.args[2]
    assert body["max_tokens"] == 8192


def test_anthropic_raises_on_4xx_status(clean_env, monkeypatch) -> None:
    monkeypatch.setenv("ANTHROPIC_API_KEY", "sk-test")
    client = LLMClient()
    response = _resp(429, payload={}, text="rate limited")
    with patch.object(client, "_post_with_retry", return_value=response):
        with pytest.raises(RuntimeError, match="Anthropic request failed: 429"):
            client.generate({"messages": [{"role": "user", "content": "hi"}]})


def test_anthropic_raises_on_empty_content(clean_env, monkeypatch) -> None:
    monkeypatch.setenv("ANTHROPIC_API_KEY", "sk-test")
    client = LLMClient()
    response = _resp(200, payload={"content": []})
    with patch.object(client, "_post_with_retry", return_value=response):
        with pytest.raises(RuntimeError, match="missing content"):
            client.generate({"messages": [{"role": "user", "content": "hi"}]})


def test_anthropic_raises_when_key_missing(clean_env) -> None:
    client = LLMClient()
    # Force-route through anthropic even though auto-selection chose ollama.
    client.backend = "anthropic"
    client.anthropic_key = None
    with pytest.raises(RuntimeError, match="ANTHROPIC_API_KEY missing"):
        client.generate({"messages": [{"role": "user", "content": "hi"}]})


# --- OpenAI ---------------------------------------------------------------------


def test_openai_extracts_choices_message_content(clean_env, monkeypatch) -> None:
    monkeypatch.setenv("OPENAI_API_KEY", "sk-test")
    client = LLMClient()
    response = _resp(
        200,
        payload={"choices": [{"message": {"content": "openai-ok"}}]},
    )
    with patch.object(client, "_post_with_retry", return_value=response):
        out = client.generate({"messages": [{"role": "user", "content": "hi"}]})
    assert out == "openai-ok"


def test_openai_raises_on_5xx_status(clean_env, monkeypatch) -> None:
    monkeypatch.setenv("OPENAI_API_KEY", "sk-test")
    client = LLMClient()
    response = _resp(503, text="service unavailable")
    with patch.object(client, "_post_with_retry", return_value=response):
        with pytest.raises(RuntimeError, match="OpenAI request failed: 503"):
            client.generate({"messages": [{"role": "user", "content": "hi"}]})


def test_openai_raises_when_key_missing(clean_env) -> None:
    client = LLMClient()
    client.backend = "openai"
    client.openai_key = None
    with pytest.raises(RuntimeError, match="OPENAI_API_KEY missing"):
        client.generate({"messages": [{"role": "user", "content": "hi"}]})


# --- Groq -----------------------------------------------------------------------


def test_groq_extracts_choices_message_content(clean_env, monkeypatch) -> None:
    monkeypatch.setenv("GROQ_API_KEY", "gsk-test")
    client = LLMClient()
    response = _resp(
        200,
        payload={"choices": [{"message": {"content": "groq-ok"}}]},
    )
    with patch.object(client, "_post_with_retry", return_value=response):
        out = client.generate({"messages": [{"role": "user", "content": "hi"}]})
    assert out == "groq-ok"


def test_groq_raises_on_4xx_status(clean_env, monkeypatch) -> None:
    monkeypatch.setenv("GROQ_API_KEY", "gsk-test")
    client = LLMClient()
    response = _resp(401, text="unauthorized")
    with patch.object(client, "_post_with_retry", return_value=response):
        with pytest.raises(RuntimeError, match="Groq request failed: 401"):
            client.generate({"messages": [{"role": "user", "content": "hi"}]})


def test_groq_raises_when_key_missing(clean_env) -> None:
    client = LLMClient()
    client.backend = "groq"
    client.groq_key = None
    with pytest.raises(RuntimeError, match="GROQ_API_KEY missing"):
        client.generate({"messages": [{"role": "user", "content": "hi"}]})


# --- Ollama ---------------------------------------------------------------------


def test_ollama_extracts_message_content(clean_env) -> None:
    client = LLMClient()
    response = _resp(200, payload={"message": {"content": "ollama-ok"}})
    with patch.object(client, "_post_with_retry", return_value=response):
        out = client.generate({"messages": [{"role": "user", "content": "hi"}]})
    assert out == "ollama-ok"


def test_ollama_returns_empty_content_with_warning(clean_env) -> None:
    client = LLMClient()
    response = _resp(200, payload={"message": {"content": ""}})
    with patch.object(client, "_post_with_retry", return_value=response), \
         patch("chatbot.generation.llm_client.LOGGER.warning") as warn:
        out = client.generate({"messages": [{"role": "user", "content": "hi"}]})
    assert out == ""
    warn.assert_called_once()
    assert "empty content" in warn.call_args.args[0]


def test_ollama_unreachable_wraps_runtimeerror(clean_env) -> None:
    client = LLMClient()
    with patch.object(
        client, "_post_with_retry",
        side_effect=RuntimeError("Ollama request failed: connection refused"),
    ):
        with pytest.raises(RuntimeError, match="Ollama backend is unavailable"):
            client.generate({"messages": [{"role": "user", "content": "hi"}]})
