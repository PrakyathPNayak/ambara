"""In-memory session store with inactivity expiration."""

from __future__ import annotations

import datetime as dt
from typing import Any


class SessionStore:
    """Stores per-session message history with TTL-based cleanup."""

    def __init__(self, expiry_minutes: int = 30) -> None:
        """Create session store.

        Args:
            expiry_minutes: Session inactivity timeout.

        Returns:
            None.

        Raises:
            ValueError: If expiry is non-positive.
        """
        if expiry_minutes <= 0:
            raise ValueError("expiry_minutes must be > 0")
        self.expiry = dt.timedelta(minutes=expiry_minutes)
        self._sessions: dict[str, dict[str, Any]] = {}

    @staticmethod
    def _utc_now() -> dt.datetime:
        """Return timezone-aware UTC timestamp.

        Args:
            None.

        Returns:
            UTC datetime.

        Raises:
            RuntimeError: Never raised in normal operation.
        """
        return dt.datetime.now(dt.UTC)

    def _prune(self) -> None:
        """Remove expired sessions.

        Args:
            None.

        Returns:
            None.

        Raises:
            RuntimeError: Never raised in normal operation.
        """
        now = self._utc_now()
        expired = [sid for sid, data in self._sessions.items() if now - data["updated_at"] > self.expiry]
        for sid in expired:
            self._sessions.pop(sid, None)

    def get_history(self, session_id: str) -> list[dict[str, Any]]:
        """Get history for a session.

        Args:
            session_id: Session identifier.

        Returns:
            Message list.

        Raises:
            KeyError: Never raised; missing session returns empty list.
        """
        self._prune()
        if session_id not in self._sessions:
            return []
        self._sessions[session_id]["updated_at"] = self._utc_now()
        return list(self._sessions[session_id]["history"])

    def append_message(self, session_id: str, message: dict[str, Any]) -> None:
        """Append message to session history.

        Args:
            session_id: Session identifier.
            message: Message payload.

        Returns:
            None.

        Raises:
            TypeError: If message is not mapping-like.
        """
        self._prune()
        if session_id not in self._sessions:
            self._sessions[session_id] = {"history": [], "updated_at": self._utc_now()}
        self._sessions[session_id]["history"].append(message)
        self._sessions[session_id]["updated_at"] = self._utc_now()

    def clear(self, session_id: str) -> None:
        """Clear session history.

        Args:
            session_id: Session identifier.

        Returns:
            None.

        Raises:
            RuntimeError: Never raised in normal operation.
        """
        self._sessions.pop(session_id, None)
