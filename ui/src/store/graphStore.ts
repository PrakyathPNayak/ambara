import { create } from 'zustand';
import {
  Edge,
  NodeChange,
  EdgeChange,
  Connection,
  applyNodeChanges,
  applyEdgeChanges,
  addEdge,
} from '@xyflow/react';
import { FilterNodeData, GraphState, FilterNode } from '../types';

interface GraphStore {
  nodes: FilterNode[];
  edges: Edge[];
  selectedNode: string | null;
  
  // Actions
  onNodesChange: (changes: NodeChange[]) => void;
  onEdgesChange: (changes: EdgeChange[]) => void;
  onConnect: (connection: Connection) => void;
  addNode: (node: FilterNode) => void;
  removeNode: (nodeId: string) => void;
  updateNodeData: (nodeId: string, data: Partial<FilterNodeData>) => void;
  setSelectedNode: (nodeId: string | null) => void;
  loadGraph: (state: GraphState) => void;
  clearGraph: () => void;
  getGraphState: () => GraphState;
}

export const useGraphStore = create<GraphStore>((set, get) => ({
  nodes: [],
  edges: [],
  selectedNode: null,

  onNodesChange: (changes) => {
    set({
      nodes: applyNodeChanges(changes, get().nodes) as FilterNode[],
    });
  },

  onEdgesChange: (changes) => {
    set({
      edges: applyEdgeChanges(changes, get().edges),
    });
  },

  onConnect: (connection) => {
    const { edges } = get();
    
    // Prevent multiple connections to the same input
    if (connection.target && connection.targetHandle) {
      const existingConnection = edges.find(
        (edge) => edge.target === connection.target && edge.targetHandle === connection.targetHandle
      );
      
      if (existingConnection) {
        // Remove the existing connection first
        set({
          edges: edges.filter((edge) => edge.id !== existingConnection.id),
        });
      }
    }
    
    set({
      edges: addEdge(
        { ...connection, type: 'smoothstep', animated: true },
        get().edges
      ),
    });
  },

  addNode: (node) => {
    set({
      nodes: [...get().nodes, node],
    });
  },

  removeNode: (nodeId) => {
    set({
      nodes: get().nodes.filter((n) => n.id !== nodeId),
      edges: get().edges.filter(
        (e) => e.source !== nodeId && e.target !== nodeId
      ),
    });
  },

  updateNodeData: (nodeId, data) => {
    set({
      nodes: get().nodes.map((node) =>
        node.id === nodeId
          ? { ...node, data: { ...node.data, ...data } }
          : node
      ),
    });
  },

  setSelectedNode: (nodeId) => {
    set({ selectedNode: nodeId });
  },

  loadGraph: (state) => {
    set({
      nodes: state.nodes,
      edges: state.edges,
    });
  },

  clearGraph: () => {
    set({
      nodes: [],
      edges: [],
      selectedNode: null,
    });
  },

  getGraphState: () => ({
    nodes: get().nodes,
    edges: get().edges,
  }),
}));
