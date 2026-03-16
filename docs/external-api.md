# External API Reference

Ambara exposes a Tauri command API for programmatic integration.

## Version

Use `get_external_api_capabilities` to discover API support and version.

## Graph Commands

- `export_graph_json(graph: GraphState) -> string`
- `import_graph_json(content: string) -> GraphState`
- `save_graph(graph: GraphState, path: string) -> Result<(), string>`
- `load_graph(path: string) -> Result<GraphState, string>`

`export_graph_json` returns a `GraphExchangeEnvelope` JSON payload:

```json
{
  "format": "ambara-graph",
  "schemaVersion": "1",
  "exportedAtUnixMs": 0,
  "graph": { "nodes": [], "edges": [] }
}
```

`import_graph_json` accepts either this envelope or raw `GraphState`.

## Plugin Commands

- `get_plugins() -> PluginInfo[]`
- `load_plugin(path: string) -> PluginInfo`
- `unload_plugin(pluginId: string) -> void`
- `get_plugin_filters(pluginId: string) -> FilterInfo[]`
- `inspect_plugin_manifest(path: string) -> PluginManifestPreview`
- `import_plugins_from_directory(dir: string) -> PluginImportSummary`
- `export_plugin_inventory_json() -> string`

## Intended Consumers

- UI import/export tools
- CI automation for plugin compatibility checks
- Future chatbot assistant integration for pipeline authoring
