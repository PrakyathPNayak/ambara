import { useState } from 'react';
import { PluginInfo } from '../../types';
import './PluginPanel.css';

interface PluginPanelProps {
    plugins: PluginInfo[];
    onLoadPlugin: () => void;
    onUnloadPlugin: (pluginId: string) => void;
    onRefresh: () => void;
}

export function PluginPanel({ plugins, onLoadPlugin, onUnloadPlugin, onRefresh }: PluginPanelProps) {
    const [expanded, setExpanded] = useState(true);

    return (
        <div className="plugin-panel">
            <button className="plugin-panel-header" onClick={() => setExpanded((v) => !v)}>
                <span>{expanded ? '▼' : '▶'}</span>
                <span>Plugins</span>
                <span className="plugin-count">{plugins.length}</span>
            </button>

            {expanded && (
                <div className="plugin-panel-content">
                    <div className="plugin-actions">
                        <button className="plugin-btn" onClick={onLoadPlugin}>Load Plugin</button>
                        <button className="plugin-btn secondary" onClick={onRefresh}>Refresh</button>
                    </div>

                    {plugins.length === 0 ? (
                        <p className="plugin-empty">No plugins loaded</p>
                    ) : (
                        <div className="plugin-list">
                            {plugins.map((plugin) => (
                                <div key={plugin.id} className="plugin-item">
                                    <div className="plugin-row">
                                        <span className="plugin-name">{plugin.name}</span>
                                        <span className={plugin.healthy ? 'plugin-health ok' : 'plugin-health bad'}>
                                            {plugin.healthy ? 'Healthy' : 'Unhealthy'}
                                        </span>
                                    </div>
                                    <div className="plugin-meta">{plugin.id} · v{plugin.version}</div>
                                    <div className="plugin-meta">{plugin.filterCount} filters</div>
                                    <button className="plugin-btn danger" onClick={() => onUnloadPlugin(plugin.id)}>
                                        Unload
                                    </button>
                                </div>
                            ))}
                        </div>
                    )}
                </div>
            )}
        </div>
    );
}
