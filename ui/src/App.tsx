import { useState, useEffect, useCallback, useRef } from 'react';
import { useTauriReady } from './hooks/useTauriReady';
import { useConfig } from './hooks/useConfig';
import { useStatus } from './hooks/useStatus';
import { useModels } from './hooks/useModels';
import { useDevices } from './hooks/useDevices';
import { forwardLog } from './services/tauri';
import ConfigForm from './components/ConfigForm';
import StatusBar from './components/StatusBar';
import './App.css';

export default function App() {
  const ready = useTauriReady();
  const { config, updateConfig, loaded } = useConfig(ready);
  const { status, uiIsRecording, toggleRecording } = useStatus(ready);
  const { models, availableForDownload, selectedModel, selectModel, downloadModel } =
    useModels(ready, config.model_search_dirs, config.model);
  const { devices, selectedDeviceId, onDeviceChange } = useDevices(
    ready,
    config.device_id,
    updateConfig,
  );

  const [statusMsg, setStatusMsg] = useState({ text: '', type: '' });
  const [keyboardTick, setKeyboardTick] = useState(0);
  const msgTimer = useRef<ReturnType<typeof setTimeout>>(null);

  // Forward console logs to Rust backend
  useEffect(() => {
    if (!ready) return;
    const levels: Array<[keyof Console, string]> = [
      ['log', 'trace'],
      ['debug', 'debug'],
      ['info', 'info'],
      ['warn', 'warn'],
      ['error', 'error'],
    ];
    levels.forEach(([fn, level]) => {
      const orig = console[fn] as (...a: unknown[]) => void;
      (console[fn] as unknown) = (...args: unknown[]) => {
        orig(...args);
        forwardLog(
          level,
          args.map((a) => (typeof a === 'object' ? JSON.stringify(a) : String(a))).join(' '),
        );
      };
    });
  }, [ready]);

  // Keyboard shortcut: Ctrl+R to toggle recording
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'r') {
        e.preventDefault();
        if (status.is_service_running) setKeyboardTick((n) => n + 1);
      }
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [status.is_service_running]);

  useEffect(() => {
    if (keyboardTick > 0) {
      toggleRecording().catch((e: Error) => showStatus('Failed: ' + e.message, 'error'));
    }
  }, [keyboardTick]); // eslint-disable-line react-hooks/exhaustive-deps

  const showStatus = useCallback((text: string, type: string) => {
    setStatusMsg({ text, type });
    if (msgTimer.current) clearTimeout(msgTimer.current);
    msgTimer.current = setTimeout(() => setStatusMsg({ text: '', type: '' }), 5000);
  }, []);

  const handleToggle = useCallback(async () => {
    try {
      await toggleRecording();
    } catch (e: unknown) {
      showStatus('Failed: ' + ((e as Error)?.message || String(e)), 'error');
    }
  }, [toggleRecording, showStatus]);

  // ── Render ──────────────────────────────────────────

  if (!ready || !loaded) {
    return (
      <div style={{ textAlign: 'center', padding: 40, color: '#fff' }}>
        <p style={{ fontSize: 18, fontWeight: 600 }}>Push-to-Talk</p>
        <p style={{ marginTop: 8, opacity: 0.8 }}>Initializing…</p>
      </div>
    );
  }

  return (
    <div className="container">
      <h1>Push-to-Talk</h1>
      <p className="subtitle">Voice transcription at your fingertips</p>

      {statusMsg.text && (
        <div className={`status-message status-${statusMsg.type}`}>{statusMsg.text}</div>
      )}

      <StatusBar status={status} />

      <ConfigForm
        config={config}
        updateConfig={updateConfig}
        models={models}
        availableForDownload={availableForDownload}
        selectedModel={selectedModel}
        onSelectModel={selectModel}
        onDownloadModel={downloadModel}
        devices={devices}
        selectedDeviceId={selectedDeviceId}
        onDeviceChange={onDeviceChange}
        status={status}
        uiIsRecording={uiIsRecording}
        onToggleRecording={handleToggle}
      />
    </div>
  );
}
