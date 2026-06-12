import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '../services/tauri';
import type { Model, DownloadableModel } from '../types';

const SCAN_MS = 5000;

export function useModels(ready: boolean, modelSearchDirs: string[], selectedPath: string | null) {
  const [models, setModels] = useState<Model[]>([]);
  const [downloadable, setDownloadable] = useState<DownloadableModel[]>([]);
  const [availableForDownload, setAvailable] = useState<DownloadableModel[]>([]);
  const [selectedModel, setSelectedModel] = useState<string | null>(selectedPath);
  const lastSnapshot = useRef('');

  // Load catalog once
  useEffect(() => {
    if (!ready) return;
    invoke<DownloadableModel[]>('get_downloadable_models')
      .then(setDownloadable)
      .catch(() => {});
  }, [ready]);

  // Recalc available
  useEffect(() => {
    const foundNames = new Set(models.map((m) => m.filename));
    setAvailable(downloadable.filter((m) => !foundNames.has(m.name)));
  }, [models, downloadable]);

  // Periodic scanning
  useEffect(() => {
    if (!ready || !modelSearchDirs.length) return;

    const scan = () => {
      invoke<Model[]>('scan_models', { modelSearchDirs: modelSearchDirs })
        .then((found) => {
          const snapshot = JSON.stringify(found.map((m) => m.filename).sort());
          if (snapshot === lastSnapshot.current) return;
          lastSnapshot.current = snapshot;
          setModels(found);

          setSelectedModel((prev) => {
            if (prev && found.some((m) => m.path === prev)) return prev;
            if (selectedPath && found.some((m) => m.path === selectedPath)) return selectedPath;
            return null;
          });
        })
        .catch(() => {});
    };

    scan();
    const id = setInterval(scan, SCAN_MS);
    return () => clearInterval(id);
  }, [ready, modelSearchDirs, selectedPath]);

  const selectModel = useCallback((path: string) => {
    setSelectedModel(path);
    lastSnapshot.current = '';
  }, []);

  const downloadModel = useCallback(
    async (modelId: string) => {
      const base = (modelSearchDirs[0] || '').replace(/[\\/]+$/, '');
      const targetDir = base ? base + '/transcriber' : '';
      await invoke('download_model', { modelId, targetDir });
    },
    [modelSearchDirs],
  );

  return { models, downloadable, availableForDownload, selectedModel, selectModel, downloadModel };
}
