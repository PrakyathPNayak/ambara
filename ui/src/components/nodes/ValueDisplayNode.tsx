import React from 'react';
import { Handle, Position, NodeProps, Node } from '@xyflow/react';
import './ValueDisplayNode.css';

interface ValueDisplayData {
  filter_type: string;
  name: string;
  parameters: Array<{
    name: string;
    value: any;
  }>;
  displayValue?: string;
  valueType?: string;
  [key: string]: unknown;
}

type ValueDisplayNodeType = Node<ValueDisplayData>;

const ValueDisplayNode: React.FC<NodeProps<ValueDisplayNodeType>> = ({ data }) => {
  const displayValue = data.displayValue || 'No value';
  const valueType = data.valueType || 'Unknown';
  
  // Determine color scheme based on type
  const typeClass = `type-${valueType.toLowerCase()}`;
  
  return (
    <div className={`value-display-node ${typeClass}`}>
      <div className="value-display-header">
        <h3 className="value-display-title">
          <svg className="value-display-icon" fill="currentColor" viewBox="0 0 20 20">
            <path d="M10 12a2 2 0 100-4 2 2 0 000 4z" />
            <path fillRule="evenodd" d="M.458 10C1.732 5.943 5.522 3 10 3s8.268 2.943 9.542 7c-1.274 4.057-5.064 7-9.542 7S1.732 14.057.458 10zM14 10a4 4 0 11-8 0 4 4 0 018 0z" clipRule="evenodd" />
          </svg>
          {data.name || 'Value Display'}
        </h3>
      </div>
      
      <div className="value-display-content">
        <div className="value-display-value">
          <div className="value-display-label">Value</div>
          <div className="value-display-text">{displayValue}</div>
        </div>
        
        <div className="value-display-type">{valueType}</div>
      </div>
      
      <div className="value-display-ports">
        <div className="value-display-port-row">
          <Handle
            type="target"
            position={Position.Left}
            id="value"
            className="node-handle target"
          />
          <span className="value-display-port-label">value</span>
        </div>
        
        <div className="value-display-port-row">
          <span className="value-display-port-label">value</span>
          <Handle
            type="source"
            position={Position.Right}
            id="value"
            className="node-handle source"
          />
        </div>
        
        <div className="value-display-port-row">
          <span className="value-display-port-label">display</span>
          <Handle
            type="source"
            position={Position.Right}
            id="display"
            className="node-handle source"
          />
        </div>
        
        <div className="value-display-port-row">
          <span className="value-display-port-label">type</span>
          <Handle
            type="source"
            position={Position.Right}
            id="type"
            className="node-handle source"
          />
        </div>
      </div>
    </div>
  );
};

export default ValueDisplayNode;
