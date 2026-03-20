"""LLM backend abstraction for Ambara graph and chat generation."""

from __future__ import annotations

import json
import logging
import os
from typing import Any

import requests
from dotenv import load_dotenv

LOGGER = logging.getLogger(__name__)


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
            force_mock: Force deterministic mock backend.

        Returns:
            None.

        Raises:
            RuntimeError: If backend configuration is invalid.
        """
        load_dotenv()
        self.force_mock = force_mock
        self.anthropic_key = os.getenv("ANTHROPIC_API_KEY")
        self.openai_key = os.getenv("OPENAI_API_KEY")
        self.groq_key = os.getenv("GROQ_API_KEY")
        self.ollama_url = os.getenv("OLLAMA_URL", "http://localhost:11434")

        if force_mock:
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
        except RuntimeError:
            LOGGER.warning("Ollama unavailable; real LLM backend is not reachable")
            raise RuntimeError(
                "Ollama backend is unavailable. Configure OPENAI_API_KEY or ANTHROPIC_API_KEY, "
                "or run a local Ollama server with a chat model."
            )

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
            "max_tokens": 1200,
            "temperature": temperature,
            "system": system,
            "messages": messages,
        }
        try:
            response = requests.post(url, headers=headers, json=body, timeout=60)
        except requests.RequestException as err:
            raise RuntimeError(f"Anthropic request failed: {err}") from err
        if response.status_code >= 400:
            raise RuntimeError(f"Anthropic request failed: {response.status_code} {response.text}")
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

        url = "https://api.openai.com/v1/chat/completions"
        headers = {
            "authorization": f"Bearer {self.openai_key}",
            "content-type": "application/json",
        }
        body = {
            "model": self.model_name,
            "temperature": temperature,
            "messages": prompt.get("messages", []),
        }
        try:
            response = requests.post(url, headers=headers, json=body, timeout=60)
        except requests.RequestException as err:
            raise RuntimeError(f"OpenAI request failed: {err}") from err
        if response.status_code >= 400:
            raise RuntimeError(f"OpenAI request failed: {response.status_code} {response.text}")
        data = response.json()
        return data["choices"][0]["message"]["content"]

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
        try:
            response = requests.post(url, json=body, timeout=60)
        except requests.RequestException as err:
            raise RuntimeError(f"Ollama request failed: {err}") from err
        if response.status_code >= 400:
            raise RuntimeError(f"Ollama request failed: {response.status_code} {response.text}")
        data = response.json()
        message = data.get("message", {})
        return message.get("content", _mock_graph_json())

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

        url = "https://api.groq.com/openai/v1/chat/completions"
        headers = {
            "authorization": f"Bearer {self.groq_key}",
            "content-type": "application/json",
        }
        body = {
            "model": self.model_name,
            "temperature": temperature,
            "messages": prompt.get("messages", []),
        }
        try:
            response = requests.post(url, headers=headers, json=body, timeout=60)
        except requests.RequestException as err:
            raise RuntimeError(f"Groq request failed: {err}") from err
        if response.status_code >= 400:
            raise RuntimeError(f"Groq request failed: {response.status_code} {response.text}")
        data = response.json()
        return data["choices"][0]["message"]["content"]
