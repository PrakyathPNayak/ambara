import { useCallback, useEffect, useState } from 'react';
import { ReactFlowProvider } from '@xyflow/react';
import { useGraphStore } from './store/graphStore';
import { useSettingsStore } from './store/settingsStore';
import { GraphCanvas } from './components/canvas/GraphCanvas';
import { FilterPalette } from './components/sidebar/FilterPalette';
import { PropertiesPanel } from './components/sidebar/PropertiesPanel';
import { Settings } from './components/sidebar/Settings';
import { ToastContainer } from './components/Toast';
import { ConfirmDialog } from './components/ConfirmDialog';
import { useToast } from './hooks/useToast';
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
  const [showClearConfirm, setShowClearConfirm] = useState(false);
  const { addNode, updateNodeData, getGraphState, loadGraph, clearGraph } = useGraphStore();
  const { settings, openSettings } = useSettingsStore();
  const toast = useToast();

  // Load filters from backend
  useEffect(() => {
    let isMounted = true;
    
    api.getFilters()
      .then((backendFilters) => {
        if (!isMounted) return;
        console.log('Loaded filters from backend:', backendFilters);
        setFilters(backendFilters);
        setBackendConnected(true);
        setLoading(false);
        toast.success('Filters loaded successfully');
      })
      .catch((err) => {
        if (!isMounted) return;
        console.error('Failed to load filters from backend:', err);
        setBackendConnected(false);
        setLoading(false);
        toast.error('Failed to connect to backend');
      });
    
    return () => {
      isMounted = false;
    };
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

    // Use preview node type for image_preview filter
    const nodeType = filter.id === 'image_preview' ? 'preview' : 'filter';

    addNode({
      id,
      type: nodeType,
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
        toast.success('Graph is valid!');
      } else {
        toast.error(`Validation failed: ${result.errors.length} error(s)`);
        result.errors.forEach((err) => {
          toast.error(err.message, 5000);
        });
      }
    } catch {
      console.log('Validation not available (backend not connected)');
      toast.warning('Validation requires the Tauri backend to be running');
    }
  }, [getGraphState, toast]);

  const handleExecute = useCallback(async () => {
    const graph = getGraphState();
    try {
      toast.info('Executing graph...');
      // Pass execution settings to the backend
      const executionSettings = {
        memoryLimitMb: settings.memoryLimitMb,
        autoChunk: settings.autoChunk,
        tileSize: settings.tileSize,
        parallel: settings.parallel,
        useCache: settings.useCache,
      };
      const result = await api.executeGraph(graph, executionSettings);
      if (result.success) {
        // Update preview nodes with their thumbnails and all nodes with output values
        Object.entries(result.outputs).forEach(([nodeId, output]) => {
          const outputData = output as { thumbnail?: string; width?: number; height?: number; [key: string]: unknown };
          const updates: Partial<FilterNodeData> = {};
          
          // Set preview data for preview nodes
          if (outputData.thumbnail) {
            updates.previewUrl = outputData.thumbnail;
            updates.previewWidth = outputData.width;
            updates.previewHeight = outputData.height;
          }
          
          // Set output values for all nodes
          updates.outputValues = outputData;
          
          updateNodeData(nodeId, updates);
        });
        toast.success(`Execution completed in ${result.executionTime}ms`);
      } else {
        toast.error(`Execution failed: ${result.errors.length} error(s)`);
        result.errors.forEach((err) => {
          toast.error(err.message, 5000);
        });
      }
    } catch (err) {
      console.log('Execution not available (backend not connected)', err);
      toast.error('Execution requires the Tauri backend to be running');
    }
  }, [getGraphState, updateNodeData, toast, settings]);

  const handleSave = useCallback(async () => {
    try {
      const path = await api.saveFileDialog([{ name: 'Ambara Graph', extensions: ['json'] }]);
      if (path) {
        await api.saveGraph(getGraphState(), path);
        toast.success('Graph saved successfully!');
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
      toast.success('Graph downloaded as graph.json');
    }
  }, [getGraphState, toast]);

  const handleLoad = useCallback(async () => {
    try {
      const path = await api.openFileDialog([{ name: 'Ambara Graph', extensions: ['json'] }]);
      if (path) {
        const graph = await api.loadGraph(path);
        loadGraph(graph);
        toast.success('Graph loaded successfully!');
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
          toast.success('Graph loaded successfully!');
        }
      };
      input.click();
    }
  }, [loadGraph, toast]);

  const handleClearGraph = useCallback(() => {
    setShowClearConfirm(true);
  }, []);

  const confirmClearGraph = useCallback(() => {
    clearGraph();
    setShowClearConfirm(false);
    toast.info('Graph cleared');
  }, [clearGraph, toast]);

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
          onClear={handleClearGraph}
          onSettings={openSettings}
        />
        <PropertiesPanel onParameterChange={handleParameterChange} />
        <Settings />
        <ToastContainer toasts={toast.toasts} onClose={toast.closeToast} />
        {showClearConfirm && (
          <ConfirmDialog
            title="Clear Graph"
            message="Are you sure you want to clear the entire graph? This action cannot be undone."
            confirmLabel="Clear"
            cancelLabel="Cancel"
            type="warning"
            onConfirm={confirmClearGraph}
            onCancel={() => setShowClearConfirm(false)}
          />
        )}
      </div>
    </ReactFlowProvider>
  );
}

export default App;
