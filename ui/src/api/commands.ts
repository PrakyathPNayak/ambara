import { invoke } from '@tauri-apps/api/core';
import { 
  FilterInfo, 
  GraphState, 
  ValidationResult, 
  ExecutionResult 
} from '../types';

// Get all available filters from the registry
export async function getFilters(): Promise<FilterInfo[]> {
  return invoke<FilterInfo[]>('get_filters');
}

// Validate the current graph
export async function validateGraph(graph: GraphState): Promise<ValidationResult> {
  return invoke<ValidationResult>('validate_graph', { graph });
}

// Execute the graph
export async function executeGraph(graph: GraphState): Promise<ExecutionResult> {
  return invoke<ExecutionResult>('execute_graph', { graph });
}

// Save graph to file
export async function saveGraph(graph: GraphState, path: string): Promise<void> {
  return invoke('save_graph', { graph, path });
}

// Load graph from file
export async function loadGraph(path: string): Promise<GraphState> {
  return invoke<GraphState>('load_graph', { path });
}

// Open file picker dialog
export async function openFileDialog(filters?: { name: string; extensions: string[] }[]): Promise<string | null> {
  return invoke<string | null>('open_file_dialog', { filters });
}

// Open save dialog
export async function saveFileDialog(filters?: { name: string; extensions: string[] }[]): Promise<string | null> {
  return invoke<string | null>('save_file_dialog', { filters });
}
