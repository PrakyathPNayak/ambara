"""Tests for in-memory session store with TTL expiration."""

from __future__ import annotations

import datetime as dt
from unittest.mock import patch

import pytest

from chatbot.api.session import SessionStore


def test_append_and_retrieve() -> None:
    store = SessionStore(expiry_minutes=30)
    store.append_message("s1", {"role": "user", "content": "hello"})
    history = store.get_history("s1")
    assert len(history) == 1
    assert history[0]["content"] == "hello"


def test_missing_session_returns_empty() -> None:
    store = SessionStore(expiry_minutes=30)
    assert store.get_history("nonexistent") == []


def test_clear_removes_session() -> None:
    store = SessionStore(expiry_minutes=30)
    store.append_message("s1", {"role": "user", "content": "hello"})
    store.clear("s1")
    assert store.get_history("s1") == []


def test_multiple_sessions_isolated() -> None:
    store = SessionStore(expiry_minutes=30)
    store.append_message("s1", {"role": "user", "content": "a"})
    store.append_message("s2", {"role": "user", "content": "b"})
    assert len(store.get_history("s1")) == 1
    assert len(store.get_history("s2")) == 1
    assert store.get_history("s1")[0]["content"] == "a"
    assert store.get_history("s2")[0]["content"] == "b"


def test_expiration_prunes_old_sessions() -> None:
    store = SessionStore(expiry_minutes=1)
    past = dt.datetime.now(dt.UTC) - dt.timedelta(minutes=5)
    store._sessions["old"] = {"history": [{"role": "user", "content": "old"}], "updated_at": past}
    # Force prune by backdating the last prune time
    store._last_prune = past
    store.append_message("new", {"role": "user", "content": "new"})
    # Old session should have been pruned
    assert store.get_history("old") == []
    assert len(store.get_history("new")) == 1


def test_get_history_returns_copy() -> None:
    """Modifying the returned list should not affect the store."""
    store = SessionStore(expiry_minutes=30)
    store.append_message("s1", {"role": "user", "content": "hello"})
    history = store.get_history("s1")
    history.append({"role": "assistant", "content": "bye"})
    assert len(store.get_history("s1")) == 1


def test_invalid_expiry_raises() -> None:
    with pytest.raises(ValueError, match="expiry_minutes must be > 0"):
        SessionStore(expiry_minutes=0)
    with pytest.raises(ValueError, match="expiry_minutes must be > 0"):
        SessionStore(expiry_minutes=-5)
