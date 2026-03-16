"""Tests for FastAPI chatbot endpoints."""

from __future__ import annotations

from fastapi.testclient import TestClient

from chatbot.api.main import app


client = TestClient(app)


def test_health() -> None:
    response = client.get("/health")
    assert response.status_code == 200
    assert response.json()["status"] == "ok"


def test_filters() -> None:
    response = client.get("/filters")
    assert response.status_code == 200
    assert isinstance(response.json(), list)


def test_graph_generate_mock() -> None:
    response = client.post("/graph/generate", json={"query": "apply blur", "partial_graph": None})
    assert response.status_code == 200
    assert "valid" in response.json()


def test_chat_mock() -> None:
    response = client.post(
        "/chat",
        json={"message": "apply blur and save image", "session_id": "sess-1", "context": []},
    )
    assert response.status_code == 200
    payload = response.json()
    assert "reply" in payload
    assert payload["session_id"] == "sess-1"
