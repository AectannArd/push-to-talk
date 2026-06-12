import { useState, useEffect, useCallback, useRef } from 'react';
import { useTauriReady } from './hooks/useTauriReady';
import { useConfig } from './hooks/useConfig';
import { useStatus } from './hooks/useStatus';
import { useModels } from './hooks/useModels';
import { useDevices } from './hooks/useDevices';
import { useTranslation } from './i18n/useTranslation';
import { forwardLog } from './services/tauri';
import ConfigForm from './components/ConfigForm';
import StatusBar from './components/StatusBar';
import LangSelect from './components/LangSelect';
import './styles/morph.css';
import './App.css';

const LANG_OPTIONS: { code: string; label: string }[] = [
  { code: 'en-US', label: 'EN' }, { code: 'ru-RU', label: 'RU' },
  { code: 'de-DE', label: 'DE' }, { code: 'fr-FR', label: 'FR' },
  { code: 'es-ES', label: 'ES' }, { code: 'it-IT', label: 'IT' },
  { code: 'pt-PT', label: 'PT' }, { code: 'pl-PL', label: 'PL' },
  { code: 'uk-UA', label: 'UA' }, { code: 'nl-NL', label: 'NL' },
  { code: 'cs-CZ', label: 'CZ' }, { code: 'sv-SE', label: 'SE' },
  { code: 'fi-FI', label: 'FI' }, { code: 'ro-RO', label: 'RO' },
  { code: 'hu-HU', label: 'HU' }, { code: 'el-GR', label: 'EL' },
  { code: 'bg-BG', label: 'BG' }, { code: 'da-DK', label: 'DA' },
  { code: 'sk-SK', label: 'SK' }, { code: 'lt-LT', label: 'LT' },
  { code: 'lv-LV', label: 'LV' }, { code: 'et-EE', label: 'EE' },
  { code: 'sl-SI', label: 'SI' }, { code: 'hr-HR', label: 'HR' },
  { code: 'no-NO', label: 'NO' }, { code: 'tr-TR', label: 'TR' },
  { code: 'be-BY', label: 'BE' },
  { code: 'zh-CN', label: 'ZH' }, { code: 'ja-JP', label: 'JA' },
  { code: 'ko-KR', label: 'KO' }, { code: 'hi-IN', label: 'HI' },
  { code: 'ar-SA', label: 'AR' }, { code: 'th-TH', label: 'TH' },
  { code: 'vi-VN', label: 'VI' }, { code: 'id-ID', label: 'ID' },
  { code: 'ms-MY', label: 'MS' }, { code: 'fa-IR', label: 'FA' },
  { code: 'he-IL', label: 'HE' }, { code: 'bn-BD', label: 'BN' },
  { code: 'ur-PK', label: 'UR' }, { code: 'ta-IN', label: 'TA' },
  { code: 'te-IN', label: 'TE' },
  { code: 'sw-KE', label: 'SW' }, { code: 'am-ET', label: 'AM' },
  { code: 'zu-ZA', label: 'ZU' }, { code: 'af-ZA', label: 'AF' },
  { code: 'ha-NG', label: 'HA' }, { code: 'yo-NG', label: 'YO' },
  { code: 'pt-BR', label: 'BR' },
];

export default function App() {
  const ready = useTauriReady();
  const { config, updateConfig, loaded } = useConfig(ready);
  const s = useTranslation(config.ui_language);
  const { status, uiIsRecording, toggleRecording } = useStatus(ready);
  const { models, availableForDownload, selectedModel, selectModel, downloadModel } =
    useModels(ready, config.model_search_dirs, config.model);
  const { devices, selectedDeviceId, onDeviceChange } = useDevices(ready, config.device_id, updateConfig);

  const [statusMsg, setStatusMsg] = useState<{ text: string; type: string }>({ text: '', type: '' });
  const [keyboardTick, setKeyboardTick] = useState(0);
  const msgTimer = useRef<ReturnType<typeof setTimeout>>(null);

  useEffect(() => {
    if (!ready) return;
    const levels: Array<[keyof Console, string]> = [
      ['log', 'trace'], ['debug', 'debug'], ['info', 'info'], ['warn', 'warn'], ['error', 'error'],
    ];
    levels.forEach(([fn, level]) => {
      const orig = console[fn] as (...a: unknown[]) => void;
      (console[fn] as unknown) = (...args: unknown[]) => {
        orig(...args);
        forwardLog(level, args.map((a) => (typeof a === 'object' ? JSON.stringify(a) : String(a))).join(' '));
      };
    });
  }, [ready]);

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
    if (keyboardTick > 0) toggleRecording().catch((e: Error) => showStatus(s.failed + e.message, 'danger'));
  }, [keyboardTick]); // eslint-disable-line react-hooks/exhaustive-deps

  const showStatus = useCallback((text: string, type: string) => {
    setStatusMsg({ text, type });
    if (msgTimer.current) clearTimeout(msgTimer.current);
    msgTimer.current = setTimeout(() => setStatusMsg({ text: '', type: '' }), 5000);
  }, []);

  const handleToggle = useCallback(async () => {
    try { await toggleRecording(); }
    catch (e: unknown) { showStatus(s.failed + ((e as Error)?.message || String(e)), 'danger'); }
  }, [toggleRecording, showStatus, s.failed]);

  if (!ready || !loaded) {
    return (
      <div className="d-flex justify-content-center align-items-center" style={{ minHeight: '100vh' }}>
        <div className="text-white text-center">
          <h4>{s.appTitle}</h4>
          <div className="spinner-border text-light mt-2" role="status">
            <span className="visually-hidden">{s.initializing}</span>
          </div>
          <p className="mt-2 opacity-75">{s.initializing}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="container py-3 position-relative">
      <div className="position-absolute top-0 end-0 mt-3 me-3">
        <LangSelect value={config.ui_language || 'en-US'} onChange={(code) => updateConfig('ui_language', code)} options={LANG_OPTIONS} />
      </div>

      <h3 className="text-center mb-1">{s.appTitle}</h3>
      <p className="text-center text-muted small mb-3">{s.appSubtitle}</p>

      {statusMsg.text && (
        <div className={`alert alert-${statusMsg.type} alert-dismissible py-2 text-center`}>
          {statusMsg.text}
        </div>
      )}

      <StatusBar status={status} s={s} />

      <ConfigForm
        config={config} updateConfig={updateConfig}
        models={models} availableForDownload={availableForDownload}
        selectedModel={selectedModel} onSelectModel={selectModel} onDownloadModel={downloadModel}
        devices={devices} selectedDeviceId={selectedDeviceId} onDeviceChange={onDeviceChange}
        status={status} uiIsRecording={uiIsRecording} onToggleRecording={handleToggle}
        s={s}
      />
    </div>
  );
}
