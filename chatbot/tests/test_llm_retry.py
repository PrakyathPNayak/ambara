"""Tests for `LLMClient._post_with_retry` retry semantics.

Covers the previously-untested critical path that every paid-provider
(Anthropic / OpenAI / Groq) request routes through. Mocks `requests.post`
and `time.sleep` to keep the tests deterministic and fast.
"""

from __future__ import annotations

from unittest.mock import MagicMock, patch

import pytest
import requests

from chatbot.generation.llm_client import LLMClient


def _resp(status: int, body: dict | None = None) -> MagicMock:
    """Build a stand-in `requests.Response` with the given status."""
    response = MagicMock(spec=requests.Response)
    response.status_code = status
    response.text = "" if body is None else str(body)
    response.json.return_value = body or {}
    return response


def test_post_with_retry_returns_first_success() -> None:
    ok = _resp(200, {"ok": True})
    with patch("chatbot.generation.llm_client.requests.post", return_value=ok) as post, \
         patch("chatbot.generation.llm_client.time.sleep") as sleep:
        result = LLMClient._post_with_retry(
            "https://example.test", None, {}, 5, "TestProvider",
        )
    assert result is ok
    assert post.call_count == 1
    sleep.assert_not_called()


def test_post_with_retry_retries_on_503_then_succeeds() -> None:
    flaky = _resp(503)
    ok = _resp(200, {"ok": True})
    with patch(
        "chatbot.generation.llm_client.requests.post",
        side_effect=[flaky, ok],
    ) as post, patch("chatbot.generation.llm_client.time.sleep") as sleep:
        result = LLMClient._post_with_retry(
            "https://example.test", None, {}, 5, "TestProvider",
        )
    assert result is ok
    assert post.call_count == 2
    assert sleep.call_count == 1


def test_post_with_retry_returns_final_5xx_after_exhaustion() -> None:
    # When every retryable attempt returns the same 5xx, the final response
    # must be returned (caller is responsible for converting it to a raise).
    bad = _resp(503)
    with patch(
        "chatbot.generation.llm_client.requests.post",
        side_effect=[bad, bad],
    ) as post, patch("chatbot.generation.llm_client.time.sleep") as sleep:
        result = LLMClient._post_with_retry(
            "https://example.test", None, {}, 5, "TestProvider",
        )
    assert result is bad
    assert post.call_count == 2
    assert sleep.call_count == 1


def test_post_with_retry_does_not_retry_non_retryable_status() -> None:
    # 400 is not in _RETRYABLE_STATUS — should return immediately.
    bad = _resp(400, {"error": "bad request"})
    with patch(
        "chatbot.generation.llm_client.requests.post", return_value=bad,
    ) as post, patch("chatbot.generation.llm_client.time.sleep") as sleep:
        result = LLMClient._post_with_retry(
            "https://example.test", None, {}, 5, "TestProvider",
        )
    assert result is bad
    assert post.call_count == 1
    sleep.assert_not_called()


def test_post_with_retry_does_not_retry_auth_failure() -> None:
    # 401/403 must NOT trigger retry — retrying an auth failure burns
    # latency budget and may aggravate rate limiters.
    for status in (401, 403):
        bad = _resp(status)
        with patch(
            "chatbot.generation.llm_client.requests.post", return_value=bad,
        ) as post, patch("chatbot.generation.llm_client.time.sleep") as sleep:
            result = LLMClient._post_with_retry(
                "https://example.test", None, {}, 5, "TestProvider",
            )
        assert result is bad
        assert post.call_count == 1, f"status {status} must not retry"
        sleep.assert_not_called()


def test_post_with_retry_retries_on_request_exception_then_succeeds() -> None:
    ok = _resp(200, {"ok": True})
    with patch(
        "chatbot.generation.llm_client.requests.post",
        side_effect=[requests.ConnectionError("boom"), ok],
    ) as post, patch("chatbot.generation.llm_client.time.sleep") as sleep:
        result = LLMClient._post_with_retry(
            "https://example.test", None, {}, 5, "TestProvider",
        )
    assert result is ok
    assert post.call_count == 2
    assert sleep.call_count == 1


def test_post_with_retry_raises_after_exception_exhaustion() -> None:
    boom = requests.Timeout("timeout")
    with patch(
        "chatbot.generation.llm_client.requests.post",
        side_effect=[boom, boom],
    ) as post, patch("chatbot.generation.llm_client.time.sleep") as sleep:
        with pytest.raises(RuntimeError, match="TestProvider request failed"):
            LLMClient._post_with_retry(
                "https://example.test", None, {}, 5, "TestProvider",
            )
    assert post.call_count == 2
    assert sleep.call_count == 1
