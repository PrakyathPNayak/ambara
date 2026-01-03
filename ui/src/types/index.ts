import { Node, Edge } from '@xyflow/react';

// Port types that match the Rust backend
export type PortType = 
  | 'Image'
  | 'Integer'
  | 'Float'
  | 'Boolean'
  | 'String'
  | 'Color'
  | 'Path'
  | 'ImageList'
  | 'Any';

// Port definition for inputs/outputs
export interface PortDefinition {
  name: string;
  portType: PortType;
  required: boolean;
  defaultValue?: unknown;
}

// Filter category
export type FilterCategory = 
  | 'Source'
  | 'Transform'
  | 'Color'
  | 'Filter'
  | 'Analysis'
  | 'Output'
  | 'Utility';

// Filter metadata from the registry
export interface FilterInfo {
  id: string;
  name: string;
  description: string;
  category: FilterCategory;
  inputs: PortDefinition[];
  outputs: PortDefinition[];
}

// Parameter value in the UI
export interface ParameterValue {
  name: string;
  value: unknown;
  type: PortType;
}

// Data stored in each node - using index signature for ReactFlow compatibility
export interface FilterNodeData {
  filterType: string;
  label: string;
  category: FilterCategory;
  inputs: PortDefinition[];
  outputs: PortDefinition[];
  parameters: ParameterValue[];
  isValid?: boolean;
  errorMessage?: string;
  [key: string]: unknown; // Index signature for ReactFlow
}

// Type alias for filter nodes
export type FilterNode = Node<FilterNodeData>;

// Graph state for serialization
export interface GraphState {
  nodes: FilterNode[];
  edges: Edge[];
}

// Execution result from backend
export interface ExecutionResult {
  success: boolean;
  errors: ExecutionError[];
  outputs: Record<string, unknown>;
  executionTime: number;
}

export interface ExecutionError {
  nodeId: string;
  message: string;
}

// Validation result
export interface ValidationResult {
  valid: boolean;
  errors: ValidationError[];
  warnings: ValidationWarning[];
}

export interface ValidationError {
  nodeId?: string;
  connectionId?: string;
  message: string;
  errorType: string;
}

export interface ValidationWarning {
  nodeId?: string;
  message: string;
}

// Progress update during execution
export interface ProgressUpdate {
  nodeId: string;
  nodeName: string;
  progress: number;
  status: 'pending' | 'running' | 'completed' | 'error';
}

// Color type for color inputs
export interface Color {
  r: number;
  g: number;
  b: number;
  a: number;
}
