import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';
import { 
  FilterInfo, 
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
