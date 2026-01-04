import { memo, useMemo, useState } from 'react';
import { Handle, Position } from '@xyflow/react';
import { FilterNodeData, PortType } from '../../types';
import './PreviewNode.css';

// Color mapping for different port types
const portColors: Record<PortType, string> = {
  Image: '#4CAF50',
  Integer: '#2196F3',
  Float: '#03A9F4',
  Boolean: '#FF9800',
  String: '#9C27B0',
  Color: '#E91E63',
  Path: '#795548',
  ImageList: '#8BC34A',
  Any: '#607D8B',
};

interface PreviewNodeProps {
  data: FilterNodeData;
  selected?: boolean;
}

function PreviewNodeComponent({ data, selected }: PreviewNodeProps) {
  const [isExpanded, setIsExpanded] = useState(true);
  
  const inputHandles = useMemo(() => 
    data.inputs.map((input) => (
      <div key={`input-${input.name}`} className="preview-handle-row input-row">
        <Handle
          type="target"
          position={Position.Left}
          id={input.name}
          className="preview-handle"
          style={{ background: portColors[input.portType] }}
        />
        <span className="preview-handle-label">{input.name}</span>
      </div>
    )), [data.inputs]);

  const outputHandles = useMemo(() =>
    data.outputs.map((output) => (
      <div key={`output-${output.name}`} className="preview-handle-row output-row">
        <span className="preview-handle-label">{output.name}</span>
        <Handle
          type="source"
          position={Position.Right}
          id={output.name}
          className="preview-handle"
          style={{ background: portColors[output.portType] }}
        />
      </div>
    )), [data.outputs]);

  return (
    <div 
      className={`preview-node ${selected ? 'selected' : ''}`}
    >
      <div className="preview-node-header">
        <span className="preview-node-title">📷 {data.label}</span>
        <button 
          className="preview-toggle"
          onClick={() => setIsExpanded(!isExpanded)}
        >
          {isExpanded ? '▼' : '▶'}
        </button>
      </div>
      
      <div className="preview-node-body">
        <div className="preview-node-ports">
          <div className="preview-input-ports">
            {inputHandles}
          </div>
          <div className="preview-output-ports">
            {outputHandles}
          </div>
        </div>
        
        {isExpanded && (
          <div className="preview-image-container">
            {data.previewUrl ? (
              <>
                <img 
                  src={data.previewUrl} 
                  alt="Preview" 
                  className="preview-image"
                  draggable={false}
                />
                {data.previewWidth && data.previewHeight && (
                  <div className="preview-dimensions">
                    {data.previewWidth} × {data.previewHeight}
                  </div>
                )}
              </>
            ) : (
              <div className="preview-placeholder">
                <span className="preview-placeholder-icon">🖼️</span>
                <span className="preview-placeholder-text">No preview</span>
                <span className="preview-placeholder-hint">Connect an image to see preview</span>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

export const PreviewNode = memo(PreviewNodeComponent);
