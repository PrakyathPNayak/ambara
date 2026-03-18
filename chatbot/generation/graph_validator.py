"""Validation utilities for generated SerializedGraph JSON payloads."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import jsonschema

from chatbot.models import ValidationResultModel


class GraphValidator:
    """Validates graph schema, filter IDs, and port connection references."""

    def __init__(self, schema_path: str, filter_id_set_path: str, corpus_path: str) -> None:
        """Initialize validator from schema and corpus files.

        Args:
            schema_path: Graph JSON schema path.
            filter_id_set_path: Valid filter IDs path.
            corpus_path: Filter corpus path.

        Returns:
            None.

        Raises:
            OSError: If files cannot be read.
        """
        self.schema = json.loads(Path(schema_path).read_text())
        self.filter_ids = set(json.loads(Path(filter_id_set_path).read_text())) if Path(filter_id_set_path).exists() else set()
        corpus = json.loads(Path(corpus_path).read_text()) if Path(corpus_path).exists() else []
        self.corpus_by_id = {item.get("id"): item for item in corpus}

    def validate_schema(self, graph_json: str) -> ValidationResultModel:
        """Validate graph against JSON schema.

        Args:
            graph_json: JSON string of graph.

        Returns:
            Validation result.

        Raises:
            json.JSONDecodeError: If input is invalid JSON.
        """
        errors: list[str] = []
        try:
            obj = json.loads(graph_json)
            jsonschema.validate(instance=obj, schema=self.schema)
        except json.JSONDecodeError as err:
            errors.append(f"Invalid JSON: {err}")
        except jsonschema.ValidationError as err:
            errors.append(f"Schema validation failed: {err.message}")
        return ValidationResultModel(valid=len(errors) == 0, errors=errors)

    def validate_filter_ids(self, graph_json: str) -> ValidationResultModel:
        """Validate that all graph filter IDs exist in registry set.

        Args:
            graph_json: JSON string of graph.

        Returns:
            Validation result.

        Raises:
            json.JSONDecodeError: If JSON parsing fails.
        """
        errors: list[str] = []
        try:
            data = json.loads(graph_json)
        except json.JSONDecodeError as err:
            return ValidationResultModel(valid=False, errors=[f"Invalid JSON: {err}"])
        for node in data.get("nodes", []):
            filter_id = node.get("filter_id")
            if filter_id not in self.filter_ids:
                errors.append(f"filter_id {filter_id} not found in registry")
        return ValidationResultModel(valid=len(errors) == 0, errors=errors)

    def validate_connections(self, graph_json: str) -> ValidationResultModel:
        """Validate port references for each connection.

        Args:
            graph_json: JSON string of graph.

        Returns:
            Validation result.

        Raises:
            json.JSONDecodeError: If JSON parsing fails.
        """
        errors: list[str] = []
        try:
            data = json.loads(graph_json)
        except json.JSONDecodeError as err:
            return ValidationResultModel(valid=False, errors=[f"Invalid JSON: {err}"])
        nodes = {n.get("id"): n for n in data.get("nodes", [])}
        used_input_ports: set[tuple[str, str]] = set()

        for conn in data.get("connections", []):
            src_node = nodes.get(conn.get("from_node"))
            dst_node = nodes.get(conn.get("to_node"))
            if src_node is None or dst_node is None:
                errors.append(f"Connection references missing node: {conn}")
                continue

            src_meta = self.corpus_by_id.get(src_node.get("filter_id"), {})
            dst_meta = self.corpus_by_id.get(dst_node.get("filter_id"), {})
            src_ports = src_meta.get("output_ports", src_meta.get("outputs", []))
            dst_ports = dst_meta.get("input_ports", dst_meta.get("inputs", []))

            src_names = {p.get("name") for p in src_ports if isinstance(p, dict)}
            dst_names = {p.get("name") for p in dst_ports if isinstance(p, dict)}

            if src_names and conn.get("from_port") not in src_names:
                errors.append(f"Invalid from_port {conn.get('from_port')} for node {conn.get('from_node')}")
            if dst_names and conn.get("to_port") not in dst_names:
                errors.append(f"Invalid to_port {conn.get('to_port')} for node {conn.get('to_node')}")

            # Disallow fan-in collisions to the exact same destination input port.
            port_key = (str(conn.get("to_node")), str(conn.get("to_port")))
            if port_key in used_input_ports:
                errors.append(
                    f"Duplicate connection into input port {conn.get('to_port')} for node {conn.get('to_node')}"
                )
            else:
                used_input_ports.add(port_key)

        return ValidationResultModel(valid=len(errors) == 0, errors=errors)

    def validate_all(self, graph_json: str) -> ValidationResultModel:
        """Run schema, filter ID, and connection checks.

        Args:
            graph_json: JSON string of graph.

        Returns:
            Aggregated validation result.

        Raises:
            json.JSONDecodeError: If parsing fails before checks.
        """
        checks = [
            self.validate_schema(graph_json),
            self.validate_filter_ids(graph_json),
            self.validate_connections(graph_json),
        ]
        errors: list[str] = []
        for result in checks:
            errors.extend(result.errors)
        return ValidationResultModel(valid=len(errors) == 0, errors=errors)
