"""LLM backend abstraction for Ambara graph and chat generation."""

from __future__ import annotations

import json
import logging
import os
import time
from datetime import datetime, timezone
from email.utils import parsedate_to_datetime
from typing import Any

import requests
from dotenv import load_dotenv

LOGGER = logging.getLogger(__name__)

_RETRYABLE_STATUS = {429, 502, 503, 504}
_MAX_RETRIES = 1
_RETRY_DELAY_S = 2.0
_RETRY_AFTER_MAX_S = 30.0
_PAID_PROVIDER_TIMEOUT_S = 60
_OLLAMA_DEFAULT_TIMEOUT_S = 180
_ANTHROPIC_DEFAULT_MAX_TOKENS = 4096


def _resolve_positive_int_env(var_name: str, default: int, *, unit: str = "") -> int:
    """Read a positive integer from `os.environ` with a sane fallback.

    Used for environment-driven knobs that must be strictly positive
    (timeouts, token budgets, retry counts). Missing, blank,
    non-integer, or non-positive values fall back to ``default``;
    rejected values are logged at WARNING with the offending input
    so misconfiguration is debuggable.

    Args:
        var_name: Name of the environment variable to read.
        default: Fallback value applied when the variable is unset
            or the parsed value is rejected. Must be positive.
        unit: Optional suffix appended to the default in the warning
            message (for example ``"s"`` for seconds). Cosmetic only.

    Returns:
        The parsed positive integer, or ``default``.
    """
    raw = os.getenv(var_name, "").strip()
    if not raw:
        return default
    try:
        value = int(raw)
    except ValueError:
        LOGGER.warning(
            "Ignoring non-integer %s=%r; using default %d%s",
            var_name, raw, default, unit,
        )
        return default
    if value <= 0:
        LOGGER.warning(
            "Ignoring non-positive %s=%d; using default %d%s",
            var_name, value, default, unit,
        )
        return default
    return value


def _resolve_ollama_timeout() -> int:
    """Resolve the Ollama HTTP timeout, defaulting to a CPU-friendly value.

    Local Ollama inference on CPU can exceed 60 seconds for first
    token on cold-start. The default is intentionally generous;
    operators on slower hardware can raise it further via
    ``OLLAMA_TIMEOUT_S``.

    Returns:
        Positive integer timeout in seconds.
    """
    return _resolve_positive_int_env(
        "OLLAMA_TIMEOUT_S", _OLLAMA_DEFAULT_TIMEOUT_S, unit="s",
    )


def _resolve_anthropic_max_tokens() -> int:
    """Resolve the Anthropic ``max_tokens`` request parameter.

    Anthropic's Messages API requires ``max_tokens`` on every
    request and silently truncates output that exceeds it. The
    default of 4096 is enough for graph-generation prompts but
    can clip long chat responses; operators can raise it via
    ``ANTHROPIC_MAX_TOKENS`` (subject to the model's per-request
    upper bound, which the API enforces).

    Returns:
        Positive integer token budget.
    """
    return _resolve_positive_int_env(
        "ANTHROPIC_MAX_TOKENS", _ANTHROPIC_DEFAULT_MAX_TOKENS,
    )


def _mock_graph_json() -> str:
    """Return a deterministic valid graph JSON string for tests.

    Args:
        None.

    Returns:
        Serialized graph JSON string.

    Raises:
        RuntimeError: Never raised in normal operation.
    """
    payload = {
        "version": "1.0.0",
        "metadata": {"generatedBy": "mock"},
        "nodes": [
            {"id": "n1", "filter_id": "load_image", "position": {"x": 100, "y": 120}, "parameters": {"path": "input.png"}},
            {"id": "n2", "filter_id": "gaussian_blur", "position": {"x": 320, "y": 120}, "parameters": {"sigma": 2.0}},
            {"id": "n3", "filter_id": "save_image", "position": {"x": 540, "y": 120}, "parameters": {"path": "output.png"}}
        ],
        "connections": [
            {"from_node": "n1", "from_port": "image", "to_node": "n2", "to_port": "image"},
            {"from_node": "n2", "from_port": "image", "to_node": "n3", "to_port": "image"}
        ]
    }
    return json.dumps(payload)


class LLMClient:
    """Select and use Anthropic/OpenAI/Ollama with deterministic mock fallback."""

    def __init__(self, force_mock: bool = False) -> None:
        """Create LLM client with backend auto-selection.

        Args:
            force_mock: Force deterministic mock backend. The environment variable
                ``AMBARA_FORCE_MOCK_LLM`` (set to ``1``/``true``/``yes``) overrides
                auto-selection and forces the mock backend; this is intended for
                deterministic smoke/e2e tests and offline CI.

        Returns:
            None.

        Raises:
            RuntimeError: If backend configuration is invalid.
        """
        load_dotenv()
        env_force = os.getenv("AMBARA_FORCE_MOCK_LLM", "").strip().lower() in {"1", "true", "yes", "on"}
        self.force_mock = bool(force_mock or env_force)
        self.anthropic_key = os.getenv("ANTHROPIC_API_KEY")
        self.openai_key = os.getenv("OPENAI_API_KEY")
        self.groq_key = os.getenv("GROQ_API_KEY")
        self.ollama_url = os.getenv("OLLAMA_URL", "http://localhost:11434")

        if self.force_mock:
            self.backend = "mock"
            self.model_name = "mock"
        elif self.anthropic_key:
            self.backend = "anthropic"
            self.model_name = "claude-sonnet-4-5"
        elif self.groq_key:
            self.backend = "groq"
            self.model_name = os.getenv("GROQ_MODEL", "llama-3.3-70b-versatile")
        elif self.openai_key:
            self.backend = "openai"
            self.model_name = "gpt-4o"
        else:
            self.backend = "ollama"
            self.model_name = os.getenv("OLLAMA_MODEL", "qwen3:8b")

    def generate(self, prompt: dict[str, Any], temperature: float = 0.0) -> str:
        """Generate model response for prompt.

        Args:
            prompt: Prompt payload with messages.
            temperature: Sampling temperature.

        Returns:
            Generated text response.

        Raises:
            RuntimeError: If provider request fails.
        """
        if self.backend == "mock":
            return _mock_graph_json()

        if self.backend == "anthropic":
            return self._generate_anthropic(prompt, temperature)

        if self.backend == "openai":
            return self._generate_openai(prompt, temperature)

        if self.backend == "groq":
            return self._generate_groq(prompt, temperature)

        try:
            return self._generate_ollama(prompt, temperature)
        except RuntimeError as err:
            LOGGER.warning("Ollama unavailable; real LLM backend is not reachable: %s", err)
            raise RuntimeError(
                "Ollama backend is unavailable. Configure OPENAI_API_KEY or ANTHROPIC_API_KEY, "
                "or run a local Ollama server with a chat model."
            ) from err

    @staticmethod
    def _parse_retry_after(header_value: str | None) -> float | None:
        """Parse an HTTP `Retry-After` header to a non-negative second count.

        Per RFC 7231 §7.1.3 the header is either:
        - delta-seconds (an integer), e.g. ``"30"``
        - HTTP-date, e.g. ``"Wed, 21 Oct 2015 07:28:00 GMT"``

        The result is clamped to ``[0, _RETRY_AFTER_MAX_S]`` so that a
        misconfigured server cannot force the client to block for an
        unbounded interval. Returns ``None`` for missing or malformed
        values so the caller can fall back to ``_RETRY_DELAY_S``.

        Args:
            header_value: Raw `Retry-After` header value or ``None``.

        Returns:
            Wait duration in seconds, or ``None`` if unparseable.

        Raises:
            None.
        """
        if not header_value:
            return None
        candidate = header_value.strip()
        if not candidate:
            return None
        try:
            seconds = float(candidate)
        except ValueError:
            try:
                target = parsedate_to_datetime(candidate)
            except (TypeError, ValueError):
                return None
            if target is None:
                return None
            if target.tzinfo is None:
                target = target.replace(tzinfo=timezone.utc)
            seconds = (target - datetime.now(timezone.utc)).total_seconds()
        if seconds <= 0:
            return 0.0
        return min(seconds, _RETRY_AFTER_MAX_S)

    @staticmethod
    def _post_with_retry(
        url: str,
        headers: dict[str, str] | None,
        body: dict[str, Any],
        timeout: int,
        provider: str,
    ) -> requests.Response:
        """POST with a single retry on transient failures.

        On retryable HTTP statuses (429/502/503/504) the helper honors a
        server-supplied ``Retry-After`` header (RFC 7231) when present and
        parseable, clamped to ``_RETRY_AFTER_MAX_S``. Otherwise it falls
        back to the constant ``_RETRY_DELAY_S``. On
        :class:`requests.RequestException` the constant delay is used (no
        header is available on a failed connection).
        """
        for attempt in range(_MAX_RETRIES + 1):
            try:
                response = requests.post(url, headers=headers, json=body, timeout=timeout)
                if response.status_code in _RETRYABLE_STATUS and attempt < _MAX_RETRIES:
                    retry_after_header = None
                    try:
                        retry_after_header = response.headers.get("Retry-After")
                    except AttributeError:
                        retry_after_header = None
                    parsed = LLMClient._parse_retry_after(retry_after_header)
                    delay = parsed if parsed is not None else _RETRY_DELAY_S
                    LOGGER.warning(
                        "%s returned %d, retrying in %.1fs…",
                        provider, response.status_code, delay,
                    )
                    time.sleep(delay)
                    continue
                return response
            except requests.RequestException as err:
                if attempt < _MAX_RETRIES:
                    LOGGER.warning(
                        "%s request failed (%s), retrying in %.1fs…",
                        provider, err, _RETRY_DELAY_S,
                    )
                    time.sleep(_RETRY_DELAY_S)
                    continue
                raise RuntimeError(f"{provider} request failed: {err}") from err
        # Unreachable: the for-loop's body always returns or raises before
        # exhausting the iterator. Kept as a defensive guard for future
        # refactors that might add new exit conditions.
        raise RuntimeError(f"{provider} request failed after retries")  # pragma: no cover

    def _call_openai_compatible(
        self,
        url: str,
        key: str,
        prompt: dict[str, Any],
        temperature: float,
        provider: str,
    ) -> str:
        """POST to an OpenAI-shape chat-completions endpoint and extract content.

        Shared by `_generate_openai` and `_generate_groq` because Groq exposes
        an OpenAI-compatible API. The caller is responsible for verifying the
        API key is present and producing the correct missing-key error
        message.

        Args:
            url: Full chat-completions URL.
            key: Bearer token (must be non-empty).
            prompt: Messages payload.
            temperature: Sampling temperature.
            provider: Human-readable provider label for error messages.

        Returns:
            Model text response from `choices[0].message.content`.

        Raises:
            RuntimeError: If the request fails or returns a non-2xx status.
        """
        headers = {
            "authorization": f"Bearer {key}",
            "content-type": "application/json",
        }
        body = {
            "model": self.model_name,
            "temperature": temperature,
            "messages": prompt.get("messages", []),
        }
        response = self._post_with_retry(url, headers, body, _PAID_PROVIDER_TIMEOUT_S, provider)
        if response.status_code >= 400:
            raise RuntimeError(
                f"{provider} request failed: {response.status_code} {response.text[:200]}"
            )
        data = response.json()
        return data["choices"][0]["message"]["content"]

    def _generate_anthropic(self, prompt: dict[str, Any], temperature: float) -> str:
        """Call Anthropic messages API.

        Args:
            prompt: Messages payload.
            temperature: Sampling temperature.

        Returns:
            Model text response.

        Raises:
            RuntimeError: If request fails.
        """
        if not self.anthropic_key:
            raise RuntimeError("ANTHROPIC_API_KEY missing")

        url = "https://api.anthropic.com/v1/messages"
        headers = {
            "x-api-key": self.anthropic_key,
            "anthropic-version": "2023-06-01",
            "content-type": "application/json",
        }
        messages = [m for m in prompt.get("messages", []) if m.get("role") != "system"]
        system = "\n".join(m.get("content", "") for m in prompt.get("messages", []) if m.get("role") == "system")
        body = {
            "model": self.model_name,
            "max_tokens": _resolve_anthropic_max_tokens(),
            "temperature": temperature,
            "system": system,
            "messages": messages,
        }
        response = self._post_with_retry(url, headers, body, _PAID_PROVIDER_TIMEOUT_S, "Anthropic")
        if response.status_code >= 400:
            raise RuntimeError(f"Anthropic request failed: {response.status_code} {response.text[:200]}")
        data = response.json()
        content = data.get("content", [])
        if content and isinstance(content, list):
            return content[0].get("text", "")
        raise RuntimeError("Anthropic response missing content")

    def _generate_openai(self, prompt: dict[str, Any], temperature: float) -> str:
        """Call OpenAI chat completions API.

        Args:
            prompt: Messages payload.
            temperature: Sampling temperature.

        Returns:
            Model text response.

        Raises:
            RuntimeError: If request fails.
        """
        if not self.openai_key:
            raise RuntimeError("OPENAI_API_KEY missing")
        return self._call_openai_compatible(
            "https://api.openai.com/v1/chat/completions",
            self.openai_key,
            prompt,
            temperature,
            "OpenAI",
        )

    def _generate_ollama(self, prompt: dict[str, Any], temperature: float) -> str:
        """Call local Ollama chat API.

        Args:
            prompt: Messages payload.
            temperature: Sampling temperature.

        Returns:
            Model text response.

        Raises:
            RuntimeError: If request fails.
        """
        url = f"{self.ollama_url}/api/chat"
        body = {
            "model": self.model_name,
            "stream": False,
            "options": {"temperature": temperature},
            "messages": prompt.get("messages", []),
        }
        response = self._post_with_retry(url, None, body, _resolve_ollama_timeout(), "Ollama")
        if response.status_code >= 400:
            raise RuntimeError(f"Ollama request failed: {response.status_code} {response.text[:200]}")
        data = response.json()
        message = data.get("message", {})
        content = message.get("content", "")
        if not content:
            LOGGER.warning("Ollama returned empty content for model %s", self.model_name)
        return content

    def _generate_groq(self, prompt: dict[str, Any], temperature: float) -> str:
        """Call Groq chat completions API (OpenAI-compatible).

        Args:
            prompt: Messages payload.
            temperature: Sampling temperature.

        Returns:
            Model text response.

        Raises:
            RuntimeError: If request fails.
        """
        if not self.groq_key:
            raise RuntimeError("GROQ_API_KEY missing")
        return self._call_openai_compatible(
            "https://api.groq.com/openai/v1/chat/completions",
            self.groq_key,
            prompt,
            temperature,
            "Groq",
        )
