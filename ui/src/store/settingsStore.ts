import { create } from 'zustand';
import { persist } from 'zustand/middleware';

/**
 * Execution settings that control how the processing pipeline runs.
 */
export interface ExecutionSettings {
  /** Memory limit in megabytes (100-8192). */
  memoryLimitMb: number;
  /** Whether to enable automatic chunked processing for large images. */
  autoChunk: boolean;
  /** Preferred tile size for chunked processing (64-4096). */
  tileSize: number;
  /** Whether to enable parallel execution. */
  parallel: boolean;
  /** Whether to enable caching. */
  useCache: boolean;
}

interface SettingsStore {
  settings: ExecutionSettings;
  isSettingsOpen: boolean;
  
  // Actions
  updateSettings: (settings: Partial<ExecutionSettings>) => void;
  setMemoryLimit: (mb: number) => void;
  setAutoChunk: (enabled: boolean) => void;
  setTileSize: (size: number) => void;
  setParallel: (enabled: boolean) => void;
  setUseCache: (enabled: boolean) => void;
  toggleSettings: () => void;
  openSettings: () => void;
  closeSettings: () => void;
  resetToDefaults: () => void;
}

const defaultSettings: ExecutionSettings = {
  memoryLimitMb: 500,
  autoChunk: true,
  tileSize: 512,
  parallel: false,
  useCache: false,
};

/**
 * Settings store with persistence to localStorage.
 * Settings are automatically saved and restored across sessions.
 */
export const useSettingsStore = create<SettingsStore>()(
  persist(
    (set) => ({
      settings: defaultSettings,
      isSettingsOpen: false,

      updateSettings: (newSettings) => {
        set((state) => ({
          settings: { ...state.settings, ...newSettings },
        }));
      },

      setMemoryLimit: (mb) => {
        const clamped = Math.min(8192, Math.max(100, mb));
        set((state) => ({
          settings: { ...state.settings, memoryLimitMb: clamped },
        }));
      },

      setAutoChunk: (enabled) => {
        set((state) => ({
          settings: { ...state.settings, autoChunk: enabled },
        }));
      },

      setTileSize: (size) => {
        const clamped = Math.min(4096, Math.max(64, size));
        set((state) => ({
          settings: { ...state.settings, tileSize: clamped },
        }));
      },

      setParallel: (enabled) => {
        set((state) => ({
          settings: { ...state.settings, parallel: enabled },
        }));
      },

      setUseCache: (enabled) => {
        set((state) => ({
          settings: { ...state.settings, useCache: enabled },
        }));
      },

      toggleSettings: () => {
        set((state) => ({ isSettingsOpen: !state.isSettingsOpen }));
      },

      openSettings: () => {
        set({ isSettingsOpen: true });
      },

      closeSettings: () => {
        set({ isSettingsOpen: false });
      },

      resetToDefaults: () => {
        set({ settings: defaultSettings });
      },
    }),
    {
      name: 'ambara-settings',
      partialize: (state) => ({ settings: state.settings }),
    }
  )
);
