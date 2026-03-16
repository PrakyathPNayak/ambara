import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';
import {
  FilterInfo,
  PluginInfo,
  ExternalApiCapabilities,
  GraphExchangeEnvelope,
  PluginManifestPreview,
  PluginImportSummary,
  GraphState,
  ValidationResult,
  ExecutionResult
} from '../types';

// Execution settings interface matching the backend
export interface ExecutionSettings {
  memoryLimitMb: number;
  autoChunk: boolean;
  tileSize: number;
  parallel: boolean;
  useCache: boolean;
}

// Get all available filters from the registry
export async function getFilters(): Promise<FilterInfo[]> {
  return invoke<FilterInfo[]>('get_filters');
}

export async function getPlugins(): Promise<PluginInfo[]> {
  return invoke<PluginInfo[]>('get_plugins');
}

export async function loadPlugin(path: string): Promise<PluginInfo> {
  return invoke<PluginInfo>('load_plugin', { path });
}

export async function unloadPlugin(pluginId: string): Promise<void> {
  return invoke('unload_plugin', { pluginId });
}

export async function getPluginFilters(pluginId: string): Promise<FilterInfo[]> {
  return invoke<FilterInfo[]>('get_plugin_filters', { pluginId });
}

export async function getExternalApiCapabilities(): Promise<ExternalApiCapabilities> {
  return invoke<ExternalApiCapabilities>('get_external_api_capabilities');
}

export async function exportGraphJson(graph: GraphState): Promise<string> {
  return invoke<string>('export_graph_json', { graph });
}

export async function importGraphJson(content: string): Promise<GraphState> {
  return invoke<GraphState>('import_graph_json', { content });
}

export async function inspectPluginManifest(path: string): Promise<PluginManifestPreview> {
  return invoke<PluginManifestPreview>('inspect_plugin_manifest', { path });
}

export async function importPluginsFromDirectory(dir: string): Promise<PluginImportSummary> {
  return invoke<PluginImportSummary>('import_plugins_from_directory', { dir });
}

export async function exportPluginInventoryJson(): Promise<string> {
  return invoke<string>('export_plugin_inventory_json');
}

export async function exportGraphEnvelope(graph: GraphState): Promise<GraphExchangeEnvelope> {
  const json = await exportGraphJson(graph);
  return JSON.parse(json) as GraphExchangeEnvelope;
}

// Validate the current graph
export async function validateGraph(graph: GraphState): Promise<ValidationResult> {
  return invoke<ValidationResult>('validate_graph', { graph });
}

// Execute the graph with optional settings
export async function executeGraph(graph: GraphState, settings?: ExecutionSettings): Promise<ExecutionResult> {
  return invoke<ExecutionResult>('execute_graph', { graph, settings });
}

// Get default execution settings
export async function getExecutionSettings(): Promise<ExecutionSettings> {
  return invoke<ExecutionSettings>('get_execution_settings');
}

// Save graph to file
export async function saveGraph(graph: GraphState, path: string): Promise<void> {
  return invoke('save_graph', { graph, path });
}

// Load graph from file
export async function loadGraph(path: string): Promise<GraphState> {
  return invoke<GraphState>('load_graph', { path });
}

// Open file picker dialog using Tauri dialog plugin
export async function openFileDialog(filters?: { name: string; extensions: string[] }[]): Promise<string | null> {
  const result = await open({
    multiple: false,
    filters: filters,
  });
  return result as string | null;
}

// Open save dialog using Tauri dialog plugin
export async function saveFileDialog(filters?: { name: string; extensions: string[] }[]): Promise<string | null> {
  const result = await save({
    filters: filters,
  });
  return result as string | null;
}

// Open directory picker dialog
export async function openDirectoryDialog(): Promise<string | null> {
  const result = await open({
    directory: true,
    multiple: false,
  });
  return result as string | null;
}
