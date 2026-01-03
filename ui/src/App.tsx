import { useCallback, useEffect, useState } from 'react';
import { ReactFlowProvider } from '@xyflow/react';
import { useGraphStore } from './store/graphStore';
import { GraphCanvas } from './components/canvas/GraphCanvas';
import { FilterPalette } from './components/sidebar/FilterPalette';
import { PropertiesPanel } from './components/sidebar/PropertiesPanel';
import { FilterInfo, FilterNodeData } from './types';
import * as api from './api/commands';
import './App.css';

// Mock filters for development (will be replaced by Tauri commands)
const mockFilters: FilterInfo[] = [
  {
    id: 'load_image',
    name: 'Load Image',
    description: 'Load an image from disk',
    category: 'Source',
    inputs: [],
    outputs: [{ name: 'image', portType: 'Image', required: true }],
  },
  {
    id: 'load_folder',
    name: 'Load Folder',
    description: 'Load all images from a folder',
    category: 'Source',
    inputs: [],
    outputs: [{ name: 'images', portType: 'ImageList', required: true }],
  },
  {
    id: 'resize',
    name: 'Resize',
    description: 'Resize an image to new dimensions',
    category: 'Transform',
    inputs: [{ name: 'image', portType: 'Image', required: true }],
    outputs: [{ name: 'image', portType: 'Image', required: true }],
  },
  {
    id: 'crop',
    name: 'Crop',
    description: 'Crop an image to a region',
    category: 'Transform',
    inputs: [{ name: 'image', portType: 'Image', required: true }],
    outputs: [{ name: 'image', portType: 'Image', required: true }],
  },
  {
    id: 'rotate',
    name: 'Rotate',
    description: 'Rotate an image by degrees',
    category: 'Transform',
    inputs: [{ name: 'image', portType: 'Image', required: true }],
    outputs: [{ name: 'image', portType: 'Image', required: true }],
  },
  {
    id: 'flip',
    name: 'Flip',
    description: 'Flip an image horizontally or vertically',
    category: 'Transform',
    inputs: [{ name: 'image', portType: 'Image', required: true }],
    outputs: [{ name: 'image', portType: 'Image', required: true }],
  },
  {
    id: 'brightness',
    name: 'Brightness',
    description: 'Adjust image brightness',
    category: 'Color',
    inputs: [{ name: 'image', portType: 'Image', required: true }],
    outputs: [{ name: 'image', portType: 'Image', required: true }],
  },
  {
    id: 'contrast',
    name: 'Contrast',
    description: 'Adjust image contrast',
    category: 'Color',
    inputs: [{ name: 'image', portType: 'Image', required: true }],
    outputs: [{ name: 'image', portType: 'Image', required: true }],
  },
  {
    id: 'saturation',
    name: 'Saturation',
    description: 'Adjust image saturation',
    category: 'Color',
    inputs: [{ name: 'image', portType: 'Image', required: true }],
    outputs: [{ name: 'image', portType: 'Image', required: true }],
  },
  {
    id: 'hue_rotate',
    name: 'Hue Rotate',
    description: 'Rotate the hue of an image',
    category: 'Color',
    inputs: [{ name: 'image', portType: 'Image', required: true }],
    outputs: [{ name: 'image', portType: 'Image', required: true }],
  },
  {
    id: 'grayscale',
    name: 'Grayscale',
    description: 'Convert image to grayscale',
    category: 'Color',
    inputs: [{ name: 'image', portType: 'Image', required: true }],
    outputs: [{ name: 'image', portType: 'Image', required: true }],
  },
  {
    id: 'invert',
    name: 'Invert',
    description: 'Invert image colors',
    category: 'Color',
    inputs: [{ name: 'image', portType: 'Image', required: true }],
    outputs: [{ name: 'image', portType: 'Image', required: true }],
  },
  {
    id: 'blur',
    name: 'Blur',
    description: 'Apply gaussian blur to an image',
    category: 'Filter',
    inputs: [{ name: 'image', portType: 'Image', required: true }],
    outputs: [{ name: 'image', portType: 'Image', required: true }],
  },
  {
    id: 'sharpen',
    name: 'Sharpen',
    description: 'Sharpen an image',
    category: 'Filter',
    inputs: [{ name: 'image', portType: 'Image', required: true }],
    outputs: [{ name: 'image', portType: 'Image', required: true }],
  },
  {
    id: 'edge_detect',
    name: 'Edge Detection',
    description: 'Detect edges in an image',
    category: 'Filter',
    inputs: [{ name: 'image', portType: 'Image', required: true }],
    outputs: [{ name: 'image', portType: 'Image', required: true }],
  },
  {
    id: 'histogram',
    name: 'Histogram',
    description: 'Generate image histogram',
    category: 'Analysis',
    inputs: [{ name: 'image', portType: 'Image', required: true }],
    outputs: [{ name: 'histogram', portType: 'Any', required: true }],
  },
  {
    id: 'save_image',
    name: 'Save Image',
    description: 'Save an image to disk',
    category: 'Output',
    inputs: [{ name: 'image', portType: 'Image', required: true }],
    outputs: [],
  },
  {
    id: 'preview',
    name: 'Preview',
    description: 'Display image preview',
    category: 'Output',
    inputs: [{ name: 'image', portType: 'Image', required: true }],
    outputs: [],
  },
  {
    id: 'passthrough',
    name: 'Passthrough',
    description: 'Pass value through unchanged',
    category: 'Utility',
    inputs: [{ name: 'input', portType: 'Any', required: true }],
    outputs: [{ name: 'output', portType: 'Any', required: true }],
  },
  {
    id: 'switch',
    name: 'Switch',
    description: 'Select between two inputs based on condition',
    category: 'Utility',
    inputs: [
      { name: 'condition', portType: 'Boolean', required: true },
      { name: 'true_value', portType: 'Any', required: true },
      { name: 'false_value', portType: 'Any', required: true },
    ],
    outputs: [{ name: 'output', portType: 'Any', required: true }],
  },
];

let nodeIdCounter = 0;

function App() {
  const [filters, setFilters] = useState<FilterInfo[]>(mockFilters);
  const { addNode, updateNodeData, getGraphState, loadGraph } = useGraphStore();

  // Try to load filters from backend, fall back to mock data
  useEffect(() => {
    api.getFilters()
      .then(setFilters)
      .catch(() => {
        console.log('Using mock filters (backend not available)');
      });
  }, []);

  const handleAddFilter = useCallback((filter: FilterInfo) => {
    const id = `node_${++nodeIdCounter}`;
    const nodeData: FilterNodeData = {
      filterType: filter.id,
      label: filter.name,
      category: filter.category,
      inputs: filter.inputs,
      outputs: filter.outputs,
      parameters: [], // Will be populated based on filter type
      isValid: true,
    };

    addNode({
      id,
      type: 'filter',
      position: { x: 250 + Math.random() * 200, y: 100 + Math.random() * 200 },
      data: nodeData,
    });
  }, [addNode]);

  const handleParameterChange = useCallback((nodeId: string, paramName: string, value: unknown) => {
    updateNodeData(nodeId, {
      parameters: useGraphStore.getState().nodes
        .find((n) => n.id === nodeId)?.data.parameters
        .map((p) => p.name === paramName ? { ...p, value } : p) || [],
    });
  }, [updateNodeData]);

  const handleValidate = useCallback(async () => {
    const graph = getGraphState();
    try {
      const result = await api.validateGraph(graph);
      if (result.valid) {
        alert('Graph is valid!');
      } else {
        alert(`Validation errors:\n${result.errors.map(e => e.message).join('\n')}`);
      }
    } catch {
      console.log('Validation not available (backend not connected)');
      alert('Validation requires the Tauri backend to be running');
    }
  }, [getGraphState]);

  const handleExecute = useCallback(async () => {
    const graph = getGraphState();
    try {
      const result = await api.executeGraph(graph);
      if (result.success) {
        alert(`Execution completed in ${result.executionTime}ms`);
      } else {
        alert(`Execution errors:\n${result.errors.map(e => e.message).join('\n')}`);
      }
    } catch {
      console.log('Execution not available (backend not connected)');
      alert('Execution requires the Tauri backend to be running');
    }
  }, [getGraphState]);

  const handleSave = useCallback(async () => {
    try {
      const path = await api.saveFileDialog([{ name: 'Ambara Graph', extensions: ['json'] }]);
      if (path) {
        await api.saveGraph(getGraphState(), path);
        alert('Graph saved!');
      }
    } catch {
      // Fallback: download as JSON
      const graph = getGraphState();
      const blob = new Blob([JSON.stringify(graph, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'graph.json';
      a.click();
      URL.revokeObjectURL(url);
    }
  }, [getGraphState]);

  const handleLoad = useCallback(async () => {
    try {
      const path = await api.openFileDialog([{ name: 'Ambara Graph', extensions: ['json'] }]);
      if (path) {
        const graph = await api.loadGraph(path);
        loadGraph(graph);
      }
    } catch {
      // Fallback: file input
      const input = document.createElement('input');
      input.type = 'file';
      input.accept = '.json';
      input.onchange = async (e) => {
        const file = (e.target as HTMLInputElement).files?.[0];
        if (file) {
          const text = await file.text();
          const graph = JSON.parse(text);
          loadGraph(graph);
        }
      };
      input.click();
    }
  }, [loadGraph]);

  return (
    <ReactFlowProvider>
      <div className="app">
        <FilterPalette filters={filters} onAddFilter={handleAddFilter} />
        <GraphCanvas
          onValidate={handleValidate}
          onExecute={handleExecute}
          onSave={handleSave}
          onLoad={handleLoad}
        />
        <PropertiesPanel onParameterChange={handleParameterChange} />
      </div>
    </ReactFlowProvider>
  );
}

export default App;
