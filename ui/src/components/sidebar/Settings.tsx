import React from 'react';
import { useSettingsStore } from '../../store/settingsStore';
import './Settings.css';

/**
 * Format memory size for display.
 */
const formatMemory = (mb: number): string => {
  if (mb >= 1024) {
    return `${(mb / 1024).toFixed(1)} GB`;
  }
  return `${mb} MB`;
};

/**
 * Settings panel for configuring execution parameters.
 * Includes memory limit slider and processing options.
 */
export const Settings: React.FC = () => {
  const { 
    settings, 
    isSettingsOpen, 
    closeSettings,
    setMemoryLimit,
    setAutoChunk,
    setTileSize,
    setParallel,
    setUseCache,
    resetToDefaults,
  } = useSettingsStore();

  if (!isSettingsOpen) {
    return null;
  }

  return (
    <div className="settings-overlay" onClick={closeSettings}>
      <div className="settings-panel" onClick={(e) => e.stopPropagation()}>
        <div className="settings-header">
          <h2>Settings</h2>
          <button className="close-button" onClick={closeSettings}>
            ×
          </button>
        </div>

        <div className="settings-content">
          {/* Memory Management Section */}
          <section className="settings-section">
            <h3>Memory Management</h3>
            
            <div className="setting-item">
              <div className="setting-label">
                <span>Memory Limit</span>
                <span className="setting-value">{formatMemory(settings.memoryLimitMb)}</span>
              </div>
              <input
                type="range"
                min="100"
                max="8192"
                step="100"
                value={settings.memoryLimitMb}
                onChange={(e) => setMemoryLimit(parseInt(e.target.value))}
                className="memory-slider"
              />
              <div className="slider-labels">
                <span>100 MB</span>
                <span>8 GB</span>
              </div>
              <p className="setting-description">
                Maximum memory to use for image processing. Larger values allow processing 
                bigger images in memory, while smaller values use tile-based processing.
              </p>
            </div>

            <div className="setting-item">
              <label className="checkbox-label">
                <input
                  type="checkbox"
                  checked={settings.autoChunk}
                  onChange={(e) => setAutoChunk(e.target.checked)}
                />
                <span>Auto-chunk large images</span>
              </label>
              <p className="setting-description">
                Automatically process images larger than half the memory limit in tiles.
              </p>
            </div>

            <div className="setting-item">
              <div className="setting-label">
                <span>Tile Size</span>
                <span className="setting-value">{settings.tileSize}px</span>
              </div>
              <input
                type="range"
                min="64"
                max="4096"
                step="64"
                value={settings.tileSize}
                onChange={(e) => setTileSize(parseInt(e.target.value))}
                className="tile-slider"
                disabled={!settings.autoChunk}
              />
              <div className="slider-labels">
                <span>64px</span>
                <span>4096px</span>
              </div>
              <p className="setting-description">
                Size of tiles for chunked processing. Larger tiles are faster but use more memory.
              </p>
            </div>
          </section>

          {/* Processing Options Section */}
          <section className="settings-section">
            <h3>Processing Options</h3>

            <div className="setting-item">
              <label className="checkbox-label">
                <input
                  type="checkbox"
                  checked={settings.parallel}
                  onChange={(e) => setParallel(e.target.checked)}
                />
                <span>Enable parallel execution</span>
              </label>
              <p className="setting-description">
                Process independent nodes in parallel. May cause issues with some filters.
              </p>
            </div>

            <div className="setting-item">
              <label className="checkbox-label">
                <input
                  type="checkbox"
                  checked={settings.useCache}
                  onChange={(e) => setUseCache(e.target.checked)}
                />
                <span>Enable result caching</span>
              </label>
              <p className="setting-description">
                Cache intermediate results to speed up re-execution.
              </p>
            </div>
          </section>
        </div>

        <div className="settings-footer">
          <button className="reset-button" onClick={resetToDefaults}>
            Reset to Defaults
          </button>
          <button className="done-button" onClick={closeSettings}>
            Done
          </button>
        </div>
      </div>
    </div>
  );
};
