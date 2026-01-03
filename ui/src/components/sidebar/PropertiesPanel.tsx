import { useGraphStore } from '../../store/graphStore';
import { ParameterValue, FilterNodeData } from '../../types';
import { openFileDialog, saveFileDialog, openDirectoryDialog } from '../../api/commands';
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
            value={(param.value as number) ?? 0}
            onChange={(e) => onChange(parseInt(e.target.value, 10) || 0)}
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
            value={(param.value as number) ?? 0}
            onChange={(e) => onChange(parseFloat(e.target.value) || 0)}
          />
        </div>
      );

    case 'Boolean':
      return (
        <div className="param-input checkbox">
          <label>
            <input
              type="checkbox"
              checked={(param.value as boolean) ?? false}
              onChange={(e) => onChange(e.target.checked)}
            />
            {param.name}
          </label>
        </div>
      );

    case 'String':
    case 'Path':
      const paramNameLower = param.name.toLowerCase();
      const isDirectory = paramNameLower.includes('directory') || paramNameLower.includes('folder');
      const isPath = param.type === 'Path' || paramNameLower.includes('path') || isDirectory;
      const isFormat = paramNameLower === 'format';
      
      // Handle format dropdown
      if (isFormat) {
        const formats = ['png', 'jpg', 'webp', 'bmp', 'tiff'];
        return (
          <div className="param-input">
            <label>{param.name}</label>
            <select
              value={(param.value as string) ?? 'png'}
              onChange={(e) => onChange(e.target.value)}
              className="param-select"
            >
              {formats.map(fmt => (
                <option key={fmt} value={fmt}>{fmt.toUpperCase()}</option>
              ))}
            </select>
          </div>
        );
      }
      
      const handleBrowse = async () => {
        let path: string | null = null;
        
        if (isDirectory) {
          path = await openDirectoryDialog();
        } else {
          const isOutput = paramNameLower.includes('output') || paramNameLower.includes('save');
          if (isOutput) {
            path = await saveFileDialog([
              { name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp'] },
              { name: 'All Files', extensions: ['*'] }
            ]);
          } else {
            path = await openFileDialog([
              { name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'gif'] },
              { name: 'All Files', extensions: ['*'] }
            ]);
          }
        }
        
        if (path) {
          onChange(path);
        }
      };
      
      return (
        <div className="param-input">
          <label>{param.name}</label>
          <div className="path-input-group">
            <input
              type="text"
              value={(param.value as string) ?? ''}
              onChange={(e) => onChange(e.target.value)}
              placeholder={isDirectory ? 'Select a folder...' : isPath ? 'Select a file...' : ''}
            />
            {isPath && (
              <button 
                className="browse-btn" 
                onClick={handleBrowse}
                title={isDirectory ? 'Browse folder...' : 'Browse...'}
              >
                📁
              </button>
            )}
          </div>
        </div>
      );

    case 'Color':
      const color = param.value as { r: number; g: number; b: number; a?: number } | null;
      const r = color?.r ?? 255;
      const g = color?.g ?? 255;
      const b = color?.b ?? 255;
      const hex = `#${r.toString(16).padStart(2, '0')}${g.toString(16).padStart(2, '0')}${b.toString(16).padStart(2, '0')}`;
      return (
        <div className="param-input">
          <label>{param.name}</label>
          <input
            type="color"
            value={hex}
            onChange={(e) => {
              const hex = e.target.value;
              onChange({
                r: parseInt(hex.slice(1, 3), 16),
                g: parseInt(hex.slice(3, 5), 16),
                b: parseInt(hex.slice(5, 7), 16),
                a: 255,
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
