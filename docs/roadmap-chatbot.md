# Chatbot Assistant Roadmap

This document tracks planned support for a chatbot assistant that helps users build image processing pipelines.

## Goals

- Add an assistant side panel in the UI.
- Let users describe desired output in natural language.
- Recommend node sequences and parameter defaults.
- Validate generated graphs before applying changes.

## Planned Milestones

1. Define assistant request/response schema over existing graph API.
2. Add "assistant hooks" to import generated graph fragments.
3. Add safety checks and confirmation flow before mutating graph.
4. Add explainability output: why each node was selected.
5. Add reproducibility: prompt + generated graph snapshot history.

## Dependencies

- Stable graph import/export API (`export_graph_json`, `import_graph_json`)
- Plugin manifest and filter metadata inspection
- Validation and execution diagnostics exposed to UI

## Non-Goals (initial phase)

- Autonomous execution without user confirmation
- Uploading private data to third-party services by default
