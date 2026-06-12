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
import './App.css';

const LANG_OPTIONS: { code: string; label: string }[] = [
  { code: 'en-US', label: 'EN' },
  { code: 'ru-RU', label: 'RU' },
  { code: 'de-DE', label: 'DE' },
  { code: 'fr-FR', label: 'FR' },
  { code: 'es-ES', label: 'ES' },
  { code: 'it-IT', label: 'IT' },
  { code: 'pt-PT', label: 'PT' },
  { code: 'pl-PL', label: 'PL' },
  { code: 'uk-UA', label: 'UA' },
  { code: 'nl-NL', label: 'NL' },
  { code: 'cs-CZ', label: 'CZ' },
  { code: 'sv-SE', label: 'SE' },
  { code: 'fi-FI', label: 'FI' },
  { code: 'ro-RO', label: 'RO' },
  { code: 'hu-HU', label: 'HU' },
  { code: 'el-GR', label: 'EL' },
  { code: 'bg-BG', label: 'BG' },
  { code: 'da-DK', label: 'DA' },
  { code: 'sk-SK', label: 'SK' },
  { code: 'lt-LT', label: 'LT' },
  { code: 'lv-LV', label: 'LV' },
  { code: 'et-EE', label: 'EE' },
  { code: 'sl-SI', label: 'SI' },
  { code: 'hr-HR', label: 'HR' },
  { code: 'no-NO', label: 'NO' },
  { code: 'tr-TR', label: 'TR' },
  { code: 'be-BY', label: 'BE' },
  // Asia
  { code: 'zh-CN', label: 'ZH' },
  { code: 'ja-JP', label: 'JA' },
  { code: 'ko-KR', label: 'KO' },
  { code: 'hi-IN', label: 'HI' },
  { code: 'ar-SA', label: 'AR' },
  { code: 'th-TH', label: 'TH' },
  { code: 'vi-VN', label: 'VI' },
  { code: 'id-ID', label: 'ID' },
  { code: 'ms-MY', label: 'MS' },
  { code: 'fa-IR', label: 'FA' },
  { code: 'he-IL', label: 'HE' },
  { code: 'bn-BD', label: 'BN' },
  { code: 'ur-PK', label: 'UR' },
  { code: 'ta-IN', label: 'TA' },
  { code: 'te-IN', label: 'TE' },
  // Africa
  { code: 'sw-KE', label: 'SW' },
  { code: 'am-ET', label: 'AM' },
  { code: 'zu-ZA', label: 'ZU' },
  { code: 'af-ZA', label: 'AF' },
  { code: 'ha-NG', label: 'HA' },
  { code: 'yo-NG', label: 'YO' },
  // Americas
  { code: 'pt-BR', label: 'BR' },
];

export default function App() {
  const ready = useTauriReady();
  const { config, updateConfig, loaded } = useConfig(ready);
  const s = useTranslation(config.ui_language);
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
      toggleRecording().catch((e: Error) => showStatus(s.failed + e.message, 'error'));
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
      showStatus(s.failed + ((e as Error)?.message || String(e)), 'error');
    }
  }, [toggleRecording, showStatus, s.failed]);

  if (!ready || !loaded) {
    return (
      <div style={{ textAlign: 'center', padding: 40, color: '#fff' }}>
        <p style={{ fontSize: 18, fontWeight: 600 }}>{s.appTitle}</p>
        <p style={{ marginTop: 8, opacity: 0.8 }}>{s.initializing}</p>
      </div>
    );
  }

  return (
    <div className="container">
      <div className="lang-switcher">
        <LangSelect
          value={config.ui_language || 'en-US'}
          onChange={(code) => updateConfig('ui_language', code)}
          options={LANG_OPTIONS}
        />
      </div>
      <h1>{s.appTitle}</h1>
      <p className="subtitle">{s.appSubtitle}</p>

      {statusMsg.text && (
        <div className={`status-message status-${statusMsg.type}`}>{statusMsg.text}</div>
      )}

      <StatusBar status={status} s={s} />

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
        s={s}
      />
    </div>
  );
}
