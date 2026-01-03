import { useGraphStore } from '../../store/graphStore';
import { ParameterValue, FilterNodeData } from '../../types';
import './PropertiesPanel.css';

interface PropertiesPanelProps {
  onParameterChange: (nodeId: string, paramName: string, value: unknown) => void;
}

export function PropertiesPanel({ onParameterChange }: PropertiesPanelProps) {
  const { nodes, selectedNode, removeNode } = useGraphStore();

  const node = nodes.find((n) => n.id === selectedNode);

  if (!node) {
    return (
      <div className="properties-panel">
        <div className="properties-empty">
          <p>Select a node to view properties</p>
        </div>
      </div>
    );
  }

  const data = node.data as FilterNodeData;

  return (
    <div className="properties-panel">
      <div className="properties-header">
        <h3>{data.label}</h3>
        <span className="properties-category">{data.category}</span>
      </div>

      <div className="properties-section">
        <h4>Inputs</h4>
        <div className="port-list">
          {data.inputs.map((input) => (
            <div key={input.name} className="port-item">
              <span className="port-name">{input.name}</span>
              <span className="port-type">{input.portType}</span>
              {input.required && <span className="port-required">*</span>}
            </div>
          ))}
          {data.inputs.length === 0 && (
            <div className="port-empty">No inputs</div>
          )}
        </div>
      </div>

      <div className="properties-section">
        <h4>Outputs</h4>
        <div className="port-list">
          {data.outputs.map((output) => (
            <div key={output.name} className="port-item">
              <span className="port-name">{output.name}</span>
              <span className="port-type">{output.portType}</span>
            </div>
          ))}
          {data.outputs.length === 0 && (
            <div className="port-empty">No outputs</div>
          )}
        </div>
      </div>

      {data.parameters.length > 0 && (
        <div className="properties-section">
          <h4>Parameters</h4>
          <div className="parameters-list">
            {data.parameters.map((param) => (
              <ParameterInput
                key={param.name}
                param={param}
                onChange={(value) => onParameterChange(node.id, param.name, value)}
              />
            ))}
          </div>
        </div>
      )}

      {data.isValid === false && data.errorMessage && (
        <div className="properties-error">
          <h4>⚠ Validation Error</h4>
          <p>{data.errorMessage}</p>
        </div>
      )}

      <div className="properties-actions">
        <button 
          className="btn-delete" 
          onClick={() => removeNode(node.id)}
        >
          Delete Node
        </button>
      </div>
    </div>
  );
}

interface ParameterInputProps {
  param: ParameterValue;
  onChange: (value: unknown) => void;
}

function ParameterInput({ param, onChange }: ParameterInputProps) {
  switch (param.type) {
    case 'Integer':
      return (
        <div className="param-input">
          <label>{param.name}</label>
          <input
            type="number"
            step="1"
            value={param.value as number}
            onChange={(e) => onChange(parseInt(e.target.value, 10))}
          />
        </div>
      );

    case 'Float':
      return (
        <div className="param-input">
          <label>{param.name}</label>
          <input
            type="number"
            step="0.1"
            value={param.value as number}
            onChange={(e) => onChange(parseFloat(e.target.value))}
          />
        </div>
      );

    case 'Boolean':
      return (
        <div className="param-input checkbox">
          <label>
            <input
              type="checkbox"
              checked={param.value as boolean}
              onChange={(e) => onChange(e.target.checked)}
            />
            {param.name}
          </label>
        </div>
      );

    case 'String':
    case 'Path':
      return (
        <div className="param-input">
          <label>{param.name}</label>
          <input
            type="text"
            value={param.value as string}
            onChange={(e) => onChange(e.target.value)}
          />
        </div>
      );

    case 'Color':
      const color = param.value as { r: number; g: number; b: number };
      const hex = `#${Math.round(color.r * 255).toString(16).padStart(2, '0')}${Math.round(color.g * 255).toString(16).padStart(2, '0')}${Math.round(color.b * 255).toString(16).padStart(2, '0')}`;
      return (
        <div className="param-input">
          <label>{param.name}</label>
          <input
            type="color"
            value={hex}
            onChange={(e) => {
              const hex = e.target.value;
              onChange({
                r: parseInt(hex.slice(1, 3), 16) / 255,
                g: parseInt(hex.slice(3, 5), 16) / 255,
                b: parseInt(hex.slice(5, 7), 16) / 255,
                a: 1,
              });
            }}
          />
        </div>
      );

    default:
      return (
        <div className="param-input">
          <label>{param.name}</label>
          <input
            type="text"
            value={JSON.stringify(param.value)}
            onChange={(e) => {
              try {
                onChange(JSON.parse(e.target.value));
              } catch {
                onChange(e.target.value);
              }
            }}
          />
        </div>
      );
  }
}
