import ModelSelector from './ModelSelector';
import DeviceSelector from './DeviceSelector';
import RecordingControls from './RecordingControls';
import PunctuationPanel from './PunctuationPanel';
import LoggingPanel from './LoggingPanel';
import type { Config, Status, Model, DownloadableModel, Device } from '../types';
import type { Strings } from '../i18n/translations';

interface Props {
  config: Config;
  updateConfig: (key: keyof Config, value: unknown) => void;
  models: Model[];
  availableForDownload: DownloadableModel[];
  selectedModel: string | null;
  onSelectModel: (path: string) => void;
  onDownloadModel: (modelId: string) => Promise<void>;
  devices: Device[];
  selectedDeviceId: string;
  onDeviceChange: (id: string) => void;
  status: Status;
  uiIsRecording: boolean;
  onToggleRecording: () => void;
  s: Strings;
}

export default function ConfigForm({
  config,
  updateConfig,
  models,
  availableForDownload,
  selectedModel,
  onSelectModel,
  onDownloadModel,
  devices,
  selectedDeviceId,
  onDeviceChange,
  status,
  uiIsRecording,
  onToggleRecording,
  s,
}: Props) {
  const searchDirsStr = (config.model_search_dirs || []).join(', ');

  const handleSearchDirsChange = (value: string) => {
    const dirs = value.split(',').map((d) => d.trim()).filter(Boolean);
    updateConfig('model_search_dirs', dirs);
  };

  const handleSelectModel = (path: string) => {
    onSelectModel(path);
    updateConfig('model', path);
  };

  return (
    <div className="config-section">
      <h2 className="config-title">{s.configuration}</h2>
      <form onSubmit={(e) => e.preventDefault()}>
        {/* ── Common ─────────────────────────────────── */}
        <div className="section-title">{s.common}</div>
        <div className="common-grid">
          <div className="common-left">
            <ModelSelector
              models={models}
              availableForDownload={availableForDownload}
              selectedModel={selectedModel}
              onSelectModel={handleSelectModel}
              onDownload={onDownloadModel}
              s={s}
            />
            <DeviceSelector
              devices={devices}
              selectedDeviceId={selectedDeviceId}
              onChange={onDeviceChange}
              s={s}
            />
          </div>
          <div className="common-right">
            <RecordingControls
              status={status}
              uiIsRecording={uiIsRecording}
              onToggle={onToggleRecording}
              s={s}
            />
          </div>
        </div>

        {/* ── Audio & Transcription ──────────────────── */}
        <div className="section-title">{s.audioTranscription}</div>
        <div className="form-group">
          <label htmlFor="hotkey">{s.hotkey}</label>
          <input
            type="text"
            id="hotkey"
            value={config.hotkey}
            onChange={(e) => updateConfig('hotkey', e.target.value)}
            placeholder="Ctrl+Shift+T"
          />
          <div className="hint">{s.hotkeyHint}</div>
        </div>
        <div className="form-group">
          <label htmlFor="language">{s.language}</label>
          <input
            type="text"
            id="language"
            value={config.language || ''}
            onChange={(e) => updateConfig('language', e.target.value)}
            placeholder="auto"
          />
          <div className="hint">{s.languageHint}</div>
        </div>

        {/* ── Whisper Model ──────────────────────────── */}
        <div className="section-title">{s.whisperModel}</div>
        <div className="form-group">
          <label htmlFor="modelSearchDirs">{s.modelSearchDirs}</label>
          <input
            type="text"
            id="modelSearchDirs"
            value={searchDirsStr}
            onChange={(e) => handleSearchDirsChange(e.target.value)}
            placeholder="~/.push-to-talk/models"
          />
          <div className="hint">{s.modelSearchDirsHint}</div>
        </div>

        {/* ── Punctuation ────────────────────────────── */}
        <PunctuationPanel config={config} updateConfig={updateConfig} status={status} s={s} />

        {/* ── Logging ────────────────────────────────── */}
        <LoggingPanel config={config} updateConfig={updateConfig} s={s} />
      </form>
    </div>
  );
}
