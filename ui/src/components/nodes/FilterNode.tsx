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
  // Normalize category for CSS attribute
  const categoryAttr = (data.category || 'utility').toLowerCase();

  const inputHandles = useMemo(() => 
    data.inputs.map((input) => (
      <div key={`input-${input.name}`} className="handle-row input-row">
        <Handle
          type="target"
          position={Position.Left}
          id={input.name}
          className="node-handle"
          style={{ background: portColors[input.portType] }}
        />
        <span className="handle-label">{input.name}</span>
        <span className="handle-type">{input.portType}</span>
      </div>
    )), [data.inputs]);

  const outputHandles = useMemo(() =>
    data.outputs.map((output) => {
      // Get the output value if it exists
      const outputValue = data.outputValues?.[output.name];
      let displayValue: string | null = null;
      
      // Only show non-image values
      if (outputValue !== undefined && output.portType !== 'Image' && output.portType !== 'ImageList') {
        if (typeof outputValue === 'number') {
          displayValue = Number.isInteger(outputValue) ? outputValue.toString() : outputValue.toFixed(2);
        } else if (typeof outputValue === 'boolean') {
          displayValue = outputValue ? '✓' : '✗';
        } else if (typeof outputValue === 'string') {
          displayValue = outputValue.length > 20 ? outputValue.slice(0, 17) + '...' : outputValue;
        } else if (typeof outputValue === 'object' && outputValue !== null) {
          // Handle arrays
          if (Array.isArray(outputValue)) {
            displayValue = `[${outputValue.length}]`;
          } else {
            displayValue = JSON.stringify(outputValue).slice(0, 20);
          }
        }
      }
      
      return (
        <div key={`output-${output.name}`} className="handle-row output-row">
          <span className="handle-type">{output.portType}</span>
          <div className="handle-label-container">
            <span className="handle-label">{output.name}</span>
            {displayValue && <span className="output-value">{displayValue}</span>}
          </div>
          <Handle
            type="source"
            position={Position.Right}
            id={output.name}
            className="node-handle"
            style={{ background: portColors[output.portType] }}
          />
        </div>
      );
    }), [data.outputs, data.outputValues]);

  return (
    <div 
      className={`filter-node ${selected ? 'selected' : ''} ${data.isValid === false ? 'invalid' : ''}`}
      data-category={categoryAttr}
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
