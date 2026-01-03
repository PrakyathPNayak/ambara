import { memo, useMemo } from 'react';
import { Handle, Position } from '@xyflow/react';
import { FilterNodeData, PortType } from '../../types';
import './FilterNode.css';

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

// Category colors
const categoryColors: Record<string, string> = {
  Source: '#4CAF50',
  Transform: '#2196F3',
  Color: '#E91E63',
  Filter: '#9C27B0',
  Analysis: '#FF9800',
  Output: '#F44336',
  Utility: '#607D8B',
};

interface FilterNodeProps {
  data: FilterNodeData;
  selected?: boolean;
}

function FilterNodeComponent({ data, selected }: FilterNodeProps) {
  const categoryColor = categoryColors[data.category] || '#607D8B';

  const inputHandles = useMemo(() => 
    data.inputs.map((input, index) => (
      <div key={`input-${input.name}`} className="handle-wrapper input-handle">
        <Handle
          type="target"
          position={Position.Left}
          id={input.name}
          style={{
            background: portColors[input.portType],
            top: `${30 + index * 24}px`,
          }}
        />
        <span className="handle-label input-label">{input.name}</span>
        <span className="handle-type">({input.portType})</span>
      </div>
    )), [data.inputs]);

  const outputHandles = useMemo(() =>
    data.outputs.map((output, index) => (
      <div key={`output-${output.name}`} className="handle-wrapper output-handle">
        <span className="handle-type">({output.portType})</span>
        <span className="handle-label output-label">{output.name}</span>
        <Handle
          type="source"
          position={Position.Right}
          id={output.name}
          style={{
            background: portColors[output.portType],
            top: `${30 + index * 24}px`,
          }}
        />
      </div>
    )), [data.outputs]);

  return (
    <div 
      className={`filter-node ${selected ? 'selected' : ''} ${data.isValid === false ? 'invalid' : ''}`}
    >
      <div 
        className="filter-node-header"
        style={{ backgroundColor: categoryColor }}
      >
        <span className="filter-node-category">{data.category}</span>
        <span className="filter-node-title">{data.label}</span>
      </div>
      
      <div className="filter-node-body">
        <div className="filter-node-ports">
          <div className="input-ports">
            {inputHandles}
          </div>
          <div className="output-ports">
            {outputHandles}
          </div>
        </div>
        
        {data.parameters.length > 0 && (
          <div className="filter-node-params">
            {data.parameters.slice(0, 3).map((param) => (
              <div key={param.name} className="param-preview">
                <span className="param-name">{param.name}:</span>
                <span className="param-value">
                  {typeof param.value === 'object' 
                    ? JSON.stringify(param.value).slice(0, 10) + '...'
                    : String(param.value).slice(0, 15)}
                </span>
              </div>
            ))}
            {data.parameters.length > 3 && (
              <div className="param-more">+{data.parameters.length - 3} more</div>
            )}
          </div>
        )}
      </div>
      
      {data.isValid === false && data.errorMessage && (
        <div className="filter-node-error">
          ⚠ {data.errorMessage}
        </div>
      )}
    </div>
  );
}

export const FilterNode = memo(FilterNodeComponent);
