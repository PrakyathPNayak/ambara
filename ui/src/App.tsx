import { useCallback, useEffect, useState } from 'react';
import { ReactFlowProvider } from '@xyflow/react';
import { useGraphStore } from './store/graphStore';
import { GraphCanvas } from './components/canvas/GraphCanvas';
import { FilterPalette } from './components/sidebar/FilterPalette';
import { PropertiesPanel } from './components/sidebar/PropertiesPanel';
import { FilterInfo, FilterNodeData, ParameterValue } from './types';
import * as api from './api/commands';
import './App.css';

// Empty fallback - rely on backend for real filters
const fallbackFilters: FilterInfo[] = [];

let nodeIdCounter = 0;

function App() {
  const [filters, setFilters] = useState<FilterInfo[]>(fallbackFilters);
  const [loading, setLoading] = useState(true);
  const [backendConnected, setBackendConnected] = useState(false);
  const { addNode, updateNodeData, getGraphState, loadGraph } = useGraphStore();

  // Load filters from backend
  useEffect(() => {
    api.getFilters()
      .then((backendFilters) => {
        console.log('Loaded filters from backend:', backendFilters);
        setFilters(backendFilters);
        setBackendConnected(true);
        setLoading(false);
      })
      .catch((err) => {
        console.error('Failed to load filters from backend:', err);
        setBackendConnected(false);
        setLoading(false);
      });
  }, []);

  const handleAddFilter = useCallback((filter: FilterInfo) => {
    const id = `node_${++nodeIdCounter}`;
    
    // Convert filter parameters to ParameterValue objects with defaults
    const parameters: ParameterValue[] = (filter.parameters || []).map(param => ({
      name: param.name,
      value: param.defaultValue ?? null,
      type: param.portType as ParameterValue['type'],
    }));

    const nodeData: FilterNodeData = {
      filterType: filter.id,
      label: filter.name,
      category: filter.category,
      inputs: filter.inputs,
      outputs: filter.outputs,
      parameters,
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
        {loading ? (
          <div className="loading-overlay">
            <div className="loading-spinner">Loading filters...</div>
          </div>
        ) : !backendConnected ? (
          <div className="backend-warning">
            <p>⚠️ Backend not connected. Make sure Tauri is running.</p>
          </div>
        ) : null}
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
