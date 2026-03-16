import { useCallback, useEffect, useState } from 'react';
import { ReactFlowProvider } from '@xyflow/react';
import { useGraphStore } from './store/graphStore';
import { useSettingsStore } from './store/settingsStore';
import { GraphCanvas } from './components/canvas/GraphCanvas';
import { FilterPalette } from './components/sidebar/FilterPalette';
import { PluginPanel } from './components/sidebar/PluginPanel';
import { ChatPanel } from './components/chat/ChatPanel';
import { PropertiesPanel } from './components/sidebar/PropertiesPanel';
import { Settings } from './components/sidebar/Settings';
import { ToastContainer } from './components/Toast';
import { ConfirmDialog } from './components/ConfirmDialog';
import { useToast } from './hooks/useToast';
import { FilterInfo, FilterNodeData, ParameterValue, PluginInfo } from './types';
import * as api from './api/commands';
import './App.css';

// Empty fallback - rely on backend for real filters
const fallbackFilters: FilterInfo[] = [];

let nodeIdCounter = 0;

function App() {
  const [filters, setFilters] = useState<FilterInfo[]>(fallbackFilters);
  const [plugins, setPlugins] = useState<PluginInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [backendConnected, setBackendConnected] = useState(false);
  const [showClearConfirm, setShowClearConfirm] = useState(false);
  const [bottomTab, setBottomTab] = useState<'plugins' | 'chat'>('plugins');
  const { addNode, updateNodeData, getGraphState, loadGraph, clearGraph } = useGraphStore();
  const { settings, openSettings } = useSettingsStore();
  const toast = useToast();

  const refreshFilters = useCallback(async () => {
    const backendFilters = await api.getFilters();
    setFilters(backendFilters);
  }, []);

  const refreshPlugins = useCallback(async () => {
    const loadedPlugins = await api.getPlugins();
    setPlugins(loadedPlugins);
  }, []);

  // Load filters from backend
  useEffect(() => {
    let isMounted = true;

    Promise.all([api.getFilters(), api.getPlugins()])
      .then(([backendFilters, loadedPlugins]) => {
        if (!isMounted) return;
        console.log('Loaded filters from backend:', backendFilters);
        setFilters(backendFilters);
        setPlugins(loadedPlugins);
        setBackendConnected(true);
        setLoading(false);
        toast.success('Filters and plugins loaded successfully');
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

  const handleLoadPlugin = useCallback(async () => {
    try {
      const path = await api.openFileDialog([
        { name: 'Plugin Library', extensions: ['so', 'dll', 'dylib'] },
      ]);
      if (!path) return;

      const plugin = await api.loadPlugin(path);
      await Promise.all([refreshPlugins(), refreshFilters()]);
      toast.success(`Loaded plugin ${plugin.name}`);
    } catch (err) {
      toast.error(`Failed to load plugin: ${String(err)}`);
    }
  }, [refreshFilters, refreshPlugins, toast]);

  const handleUnloadPlugin = useCallback(async (pluginId: string) => {
    try {
      await api.unloadPlugin(pluginId);
      await Promise.all([refreshPlugins(), refreshFilters()]);
      toast.success(`Unloaded plugin ${pluginId}`);
    } catch (err) {
      toast.error(`Failed to unload plugin: ${String(err)}`);
    }
  }, [refreshFilters, refreshPlugins, toast]);

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
          const outputData = output as { thumbnail?: string; width?: number; height?: number;[key: string]: unknown };
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

  const handleInsertGeneratedGraph = useCallback((graph: Record<string, unknown>) => {
    const serializedNodes = Array.isArray((graph as { nodes?: unknown[] }).nodes)
      ? ((graph as { nodes?: unknown[] }).nodes as Array<Record<string, unknown>>)
      : [];
    const serializedConnections = Array.isArray((graph as { connections?: unknown[] }).connections)
      ? ((graph as { connections?: unknown[] }).connections as Array<Record<string, unknown>>)
      : [];

    const nodes = serializedNodes.map((node, index) => {
      const filterType = String(node.filter_id ?? 'unknown_filter');
      const filter = filters.find((f) => f.id === filterType);
      const params = (node.parameters && typeof node.parameters === 'object') ? (node.parameters as Record<string, unknown>) : {};

      const nodeData: FilterNodeData = {
        filterType,
        label: filter?.name ?? filterType,
        category: filter?.category ?? 'Custom',
        inputs: filter?.inputs ?? [],
        outputs: filter?.outputs ?? [],
        parameters: Object.entries(params).map(([name, value]) => ({
          name,
          value,
          type: 'Any',
        })),
        isValid: true,
      };

      const position = (node.position && typeof node.position === 'object') ? (node.position as { x?: number; y?: number }) : {};

      return {
        id: String(node.id ?? `generated_${index}`),
        type: filterType === 'image_preview' ? 'preview' : 'filter',
        position: {
          x: typeof position.x === 'number' ? position.x : 100 + index * 80,
          y: typeof position.y === 'number' ? position.y : 100,
        },
        data: nodeData,
      };
    });

    const edges = serializedConnections.map((conn, index) => {
      const fromNode = String(conn.from_node ?? '');
      const toNode = String(conn.to_node ?? '');
      const fromPort = String(conn.from_port ?? 'output');
      const toPort = String(conn.to_port ?? 'input');
      return {
        id: `e_generated_${index}_${fromNode}_${toNode}`,
        source: fromNode,
        target: toNode,
        sourceHandle: fromPort,
        targetHandle: toPort,
        type: 'smoothstep',
        animated: true,
      };
    });

    loadGraph({ nodes, edges });
    toast.success('Inserted generated graph into canvas');
  }, [filters, loadGraph, toast]);

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
        <div className="left-sidebar">
          <FilterPalette filters={filters} onAddFilter={handleAddFilter} />
          <div className="sidebar-bottom">
            <div className="sidebar-tab-bar">
              <button
                className={`sidebar-tab${bottomTab === 'plugins' ? ' active' : ''}`}
                onClick={() => setBottomTab('plugins')}
              >
                Plugins
              </button>
              <button
                className={`sidebar-tab${bottomTab === 'chat' ? ' active' : ''}`}
                onClick={() => setBottomTab('chat')}
              >
                AI Chat
              </button>
            </div>
            <div className="sidebar-tab-content">
              {bottomTab === 'plugins' ? (
                <PluginPanel
                  plugins={plugins}
                  onLoadPlugin={handleLoadPlugin}
                  onUnloadPlugin={handleUnloadPlugin}
                  onRefresh={refreshPlugins}
                />
              ) : (
                <ChatPanel onInsertGraph={handleInsertGeneratedGraph} />
              )}
            </div>
          </div>
        </div>
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
