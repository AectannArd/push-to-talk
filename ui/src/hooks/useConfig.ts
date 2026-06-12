import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '../services/tauri';
import type { Config } from '../types';

const DEFAULTS: Config = {
  hotkey: 'Ctrl+Shift+T',
  language: '',
  device_id: null,
  device_name: null,
  model: null,
  model_search_dirs: [],
  log_dir: 'logs',
  log_level: 'warn',
  log_format: 'text',
  log_retention_hours: 2,
  punctuation_enabled: false,
};

const DEBOUNCE_MS = 500;

export function useConfig(ready: boolean) {
  const [config, setConfig] = useState<Config>(DEFAULTS);
  const [loaded, setLoaded] = useState(false);
  const isInitialLoad = useRef(true);
  const timerRef = useRef<ReturnType<typeof setTimeout>>(null);

  // Load once when Tauri is ready
  useEffect(() => {
    if (!ready) return;
    invoke<Config>('get_config')
      .then((cfg) => {
        setConfig({
          hotkey: cfg.hotkey || DEFAULTS.hotkey,
          language: cfg.language || '',
          device_id: cfg.device_id ?? null,
          device_name: cfg.device_name ?? null,
          model: cfg.model ?? null,
          model_search_dirs: cfg.model_search_dirs ?? [],
          log_dir: cfg.log_dir || DEFAULTS.log_dir,
          log_level: cfg.log_level || DEFAULTS.log_level,
          log_format: cfg.log_format || DEFAULTS.log_format,
          log_retention_hours: cfg.log_retention_hours ?? DEFAULTS.log_retention_hours,
          punctuation_enabled: cfg.punctuation_enabled ?? false,
        });
        setLoaded(true);
        setTimeout(() => { isInitialLoad.current = false; }, 100);
      })
      .catch((err: Error) => {
        console.error('Failed to load config:', err);
        setLoaded(true);
      });
  }, [ready]);

  const toBackend = useCallback((c: Config): Record<string, unknown> => ({
    hotkey: c.hotkey,
    language: c.language || null,
    device_id: c.device_id || null,
    device_name: c.device_name || null,
    model: c.model || null,
    model_search_dirs: c.model_search_dirs,
    log_dir: c.log_dir,
    log_level: c.log_level,
    log_format: c.log_format,
    log_retention_hours: c.log_retention_hours,
    punctuation_enabled: c.punctuation_enabled,
  }), []);

  const updateConfig = useCallback((key: keyof Config, value: unknown) => {
    setConfig((prev) => {
      const next = { ...prev, [key]: value };
      if (isInitialLoad.current) return next;
      if (timerRef.current) clearTimeout(timerRef.current);
      timerRef.current = setTimeout(() => {
        invoke('save_config', { cfg: toBackend(next) }).catch((e: Error) =>
          console.error('Failed to save config:', e),
        );
      }, DEBOUNCE_MS);
      return next;
    });
  }, [toBackend]);

  return { config, updateConfig, loaded };
}
