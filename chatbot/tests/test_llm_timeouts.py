"""Tests for env-var-driven LLM client knobs.

Covers:
- The generic ``_resolve_positive_int_env`` helper.
- The Ollama HTTP timeout resolver (CPU-friendly default).
- The Anthropic ``max_tokens`` resolver (avoids silent output
  truncation on long responses).
"""

from __future__ import annotations

from unittest.mock import patch

from chatbot.generation import llm_client
from chatbot.generation.llm_client import (
    _ANTHROPIC_DEFAULT_MAX_TOKENS,
    _ANTHROPIC_DEFAULT_VERSION,
    _OLLAMA_DEFAULT_TIMEOUT_S,
    _PAID_PROVIDER_TIMEOUT_S,
    _resolve_anthropic_max_tokens,
    _resolve_anthropic_version,
    _resolve_ollama_timeout,
    _resolve_positive_int_env,
    _resolve_str_env,
)


# ── _resolve_positive_int_env ────────────────────────────────────────────────


def test_resolver_default_when_unset(monkeypatch) -> None:
    monkeypatch.delenv("AMBARA_TEST_KNOB", raising=False)
    assert _resolve_positive_int_env("AMBARA_TEST_KNOB", 42) == 42


def test_resolver_default_when_blank(monkeypatch) -> None:
    monkeypatch.setenv("AMBARA_TEST_KNOB", "   ")
    assert _resolve_positive_int_env("AMBARA_TEST_KNOB", 42) == 42


def test_resolver_honors_positive_int(monkeypatch) -> None:
    monkeypatch.setenv("AMBARA_TEST_KNOB", "7")
    assert _resolve_positive_int_env("AMBARA_TEST_KNOB", 42) == 7


def test_resolver_rejects_non_integer(monkeypatch) -> None:
    monkeypatch.setenv("AMBARA_TEST_KNOB", "abc")
    with patch.object(llm_client.LOGGER, "warning") as warn:
        result = _resolve_positive_int_env("AMBARA_TEST_KNOB", 42)
    assert result == 42
    warn.assert_called_once()
    assert "non-integer" in warn.call_args.args[0]
    # Var name and offending value both reach the log for debuggability.
    assert "AMBARA_TEST_KNOB" in warn.call_args.args
    assert "abc" in warn.call_args.args


def test_resolver_rejects_zero(monkeypatch) -> None:
    monkeypatch.setenv("AMBARA_TEST_KNOB", "0")
    with patch.object(llm_client.LOGGER, "warning") as warn:
        result = _resolve_positive_int_env("AMBARA_TEST_KNOB", 42)
    assert result == 42
    warn.assert_called_once()
    assert "below minimum" in warn.call_args.args[0]


def test_resolver_rejects_negative(monkeypatch) -> None:
    monkeypatch.setenv("AMBARA_TEST_KNOB", "-5")
    with patch.object(llm_client.LOGGER, "warning") as warn:
        result = _resolve_positive_int_env("AMBARA_TEST_KNOB", 42)
    assert result == 42
    warn.assert_called_once()


def test_resolver_unit_appears_in_warning(monkeypatch) -> None:
    monkeypatch.setenv("AMBARA_TEST_KNOB", "0")
    with patch.object(llm_client.LOGGER, "warning") as warn:
        _resolve_positive_int_env("AMBARA_TEST_KNOB", 60, unit="s")
    fmt, *args = warn.call_args.args
    rendered = fmt % tuple(args)
    assert "60s" in rendered


def test_resolver_min_value_zero_accepts_zero(monkeypatch) -> None:
    # Counter-style knobs (retry counts, attempt budgets) should
    # be allowed to pass 0 explicitly.
    from chatbot.generation.llm_client import _resolve_int_env
    monkeypatch.setenv("AMBARA_TEST_KNOB", "0")
    assert _resolve_int_env("AMBARA_TEST_KNOB", 5, min_value=0) == 0


def test_resolver_min_value_zero_rejects_negative(monkeypatch) -> None:
    from chatbot.generation.llm_client import _resolve_int_env
    monkeypatch.setenv("AMBARA_TEST_KNOB", "-1")
    with patch.object(llm_client.LOGGER, "warning") as warn:
        result = _resolve_int_env("AMBARA_TEST_KNOB", 5, min_value=0)
    assert result == 5
    warn.assert_called_once()
    assert "below minimum" in warn.call_args.args[0]


# ── _resolve_ollama_timeout (regression-style; covers wiring) ────────────────


def test_resolve_ollama_timeout_default_when_unset(monkeypatch) -> None:
    monkeypatch.delenv("OLLAMA_TIMEOUT_S", raising=False)
    assert _resolve_ollama_timeout() == _OLLAMA_DEFAULT_TIMEOUT_S


def test_resolve_ollama_timeout_honors_override(monkeypatch) -> None:
    monkeypatch.setenv("OLLAMA_TIMEOUT_S", "300")
    assert _resolve_ollama_timeout() == 300


def test_resolve_ollama_timeout_rejects_garbage(monkeypatch) -> None:
    monkeypatch.setenv("OLLAMA_TIMEOUT_S", "soon")
    with patch.object(llm_client.LOGGER, "warning"):
        assert _resolve_ollama_timeout() == _OLLAMA_DEFAULT_TIMEOUT_S


def test_default_is_higher_than_paid_provider_timeout() -> None:
    # Local CPU inference is allowed more time than paid network APIs
    # because cold-start latency on a 7-8B model can exceed 60s.
    assert _OLLAMA_DEFAULT_TIMEOUT_S > _PAID_PROVIDER_TIMEOUT_S


# ── _resolve_anthropic_max_tokens ────────────────────────────────────────────


def test_anthropic_max_tokens_default_when_unset(monkeypatch) -> None:
    monkeypatch.delenv("ANTHROPIC_MAX_TOKENS", raising=False)
    assert _resolve_anthropic_max_tokens() == _ANTHROPIC_DEFAULT_MAX_TOKENS
    assert _ANTHROPIC_DEFAULT_MAX_TOKENS == 4096  # pin historical default


def test_anthropic_max_tokens_honors_override(monkeypatch) -> None:
    monkeypatch.setenv("ANTHROPIC_MAX_TOKENS", "8192")
    assert _resolve_anthropic_max_tokens() == 8192


def test_anthropic_max_tokens_rejects_zero(monkeypatch) -> None:
    monkeypatch.setenv("ANTHROPIC_MAX_TOKENS", "0")
    with patch.object(llm_client.LOGGER, "warning"):
        assert _resolve_anthropic_max_tokens() == _ANTHROPIC_DEFAULT_MAX_TOKENS


def test_anthropic_max_tokens_rejects_garbage(monkeypatch) -> None:
    monkeypatch.setenv("ANTHROPIC_MAX_TOKENS", "many")
    with patch.object(llm_client.LOGGER, "warning"):
        assert _resolve_anthropic_max_tokens() == _ANTHROPIC_DEFAULT_MAX_TOKENS



# ── _resolve_str_env / _resolve_anthropic_version ────────────────────────────


def test_str_resolver_default_when_unset(monkeypatch) -> None:
    monkeypatch.delenv("AMBARA_TEST_STR_KNOB", raising=False)
    assert _resolve_str_env("AMBARA_TEST_STR_KNOB", "fallback") == "fallback"


def test_str_resolver_default_when_blank(monkeypatch) -> None:
    monkeypatch.setenv("AMBARA_TEST_STR_KNOB", "   ")
    assert _resolve_str_env("AMBARA_TEST_STR_KNOB", "fallback") == "fallback"


def test_str_resolver_trims_and_returns_value(monkeypatch) -> None:
    monkeypatch.setenv("AMBARA_TEST_STR_KNOB", "  hello-world  ")
    assert _resolve_str_env("AMBARA_TEST_STR_KNOB", "fallback") == "hello-world"


def test_anthropic_version_default(monkeypatch) -> None:
    # Pin both the historical default and the resolver's fallback path.
    monkeypatch.delenv("ANTHROPIC_VERSION", raising=False)
    assert _resolve_anthropic_version() == _ANTHROPIC_DEFAULT_VERSION
    assert _ANTHROPIC_DEFAULT_VERSION == "2023-06-01"  # pin historical default


def test_anthropic_version_honors_override(monkeypatch) -> None:
    monkeypatch.setenv("ANTHROPIC_VERSION", "2024-12-01")
    assert _resolve_anthropic_version() == "2024-12-01"


def test_anthropic_version_blank_falls_back(monkeypatch) -> None:
    # Anthropic rejects requests with a blank/missing anthropic-version
    # header (HTTP 400). Make sure misconfiguration cannot send one.
    monkeypatch.setenv("ANTHROPIC_VERSION", "   ")
    assert _resolve_anthropic_version() == _ANTHROPIC_DEFAULT_VERSION
