import { useCallback, useMemo } from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  BackgroundVariant,
  Panel,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import { useGraphStore } from '../../store/graphStore';
import { FilterNode } from '../nodes/FilterNode';
import './GraphCanvas.css';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const nodeTypes: Record<string, any> = {
  filter: FilterNode,
};

interface GraphCanvasProps {
  onValidate: () => void;
  onExecute: () => void;
  onSave: () => void;
  onLoad: () => void;
}

export function GraphCanvas({ onValidate, onExecute, onSave, onLoad }: GraphCanvasProps) {
  const { nodes, edges, onNodesChange, onEdgesChange, onConnect, setSelectedNode } =
    useGraphStore();

  const handleNodeClick = useCallback(
    (_: React.MouseEvent, node: { id: string }) => {
      setSelectedNode(node.id);
    },
    [setSelectedNode]
  );

  const handlePaneClick = useCallback(() => {
    setSelectedNode(null);
  }, [setSelectedNode]);

  const minimapNodeColor = useMemo(() => {
    return (node: { data?: { category?: string } }) => {
      const categoryColors: Record<string, string> = {
        Source: '#4CAF50',
        Transform: '#2196F3',
        Color: '#E91E63',
        Filter: '#9C27B0',
        Analysis: '#FF9800',
        Output: '#F44336',
        Utility: '#607D8B',
      };
      return categoryColors[node.data?.category || ''] || '#607D8B';
    };
  }, []);

  return (
    <div className="graph-canvas">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onNodeClick={handleNodeClick}
        onPaneClick={handlePaneClick}
        nodeTypes={nodeTypes}
        fitView
        snapToGrid
        snapGrid={[15, 15]}
        defaultEdgeOptions={{
          type: 'smoothstep',
          animated: true,
        }}
      >
        <Background 
          variant={BackgroundVariant.Dots} 
          gap={20} 
          size={1}
          color="#333"
        />
        <Controls className="graph-controls" />
        <MiniMap 
          nodeColor={minimapNodeColor}
          maskColor="rgba(0, 0, 0, 0.8)"
          className="graph-minimap"
        />
        
        <Panel position="top-right" className="graph-toolbar">
          <button className="toolbar-btn" onClick={onValidate} title="Validate Graph">
            ✓ Validate
          </button>
          <button className="toolbar-btn primary" onClick={onExecute} title="Execute Graph">
            ▶ Execute
          </button>
          <div className="toolbar-separator" />
          <button className="toolbar-btn" onClick={onSave} title="Save Graph">
            💾 Save
          </button>
          <button className="toolbar-btn" onClick={onLoad} title="Load Graph">
            📂 Load
          </button>
        </Panel>
      </ReactFlow>
    </div>
  );
}
