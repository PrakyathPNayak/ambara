"""DEPRECATED: Keyword-based intent classifier.

This module has been superseded by the LLM-powered agentic router in
``chatbot/generation/agent.py`` which uses tool calls to decide how to
respond to user messages.  No code imports this module.
Safe to delete after verifying no external tools depend on it.
"""

from __future__ import annotations

from chatbot.generation.llm_client import LLMClient


class IntentClassifier:
    """Classifies user message into graph, question, clarification, or other."""

    GRAPH_KEYWORDS = {
        "blur",
        "resize",
        "crop",
        "rotate",
        "save",
        "load",
        "pipeline",
        "workflow",
        "graph",
        "apply",
    }
    QUESTION_KEYWORDS = {"what", "which", "how", "why", "filters", "available"}
    CLARIFICATION_KEYWORDS = {"instead", "change", "update", "modify", "also", "then"}

    def __init__(self) -> None:
        """Create intent classifier with LLM fallback client.

        Args:
            None.

        Returns:
            None.

        Raises:
            RuntimeError: If LLM fallback init fails.
        """
        self.fallback = LLMClient(force_mock=True)

    def classify(self, message: str) -> str:
        """Classify message intent.

        Args:
            message: User message.

        Returns:
            One of GRAPH_REQUEST, QUESTION, CLARIFICATION, OTHER.

        Raises:
            RuntimeError: Never raised in normal operation.
        """
        lowered = message.lower()

        if any(token in lowered for token in self.GRAPH_KEYWORDS):
            return "GRAPH_REQUEST"
        # A question mark is helpful but not required — natural language often omits it
        if any(token in lowered for token in self.QUESTION_KEYWORDS):
            return "QUESTION"
        if any(token in lowered for token in self.CLARIFICATION_KEYWORDS):
            return "CLARIFICATION"

        # LLM fallback for ambiguity (mocked deterministic for tests)
        probe = self.fallback.generate({"messages": [{"role": "user", "content": message}]})
        if "nodes" in probe and "connections" in probe:
            return "GRAPH_REQUEST"
        return "OTHER"
