import ModelSelector from './ModelSelector';
import DeviceSelector from './DeviceSelector';
import RecordingControls from './RecordingControls';
import PunctuationPanel from './PunctuationPanel';
import LoggingPanel from './LoggingPanel';
import type { Config, Status, Model, DownloadableModel, Device } from '../types';

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
}: Props) {
  const searchDirsStr = (config.model_search_dirs || []).join(', ');

  const handleSearchDirsChange = (value: string) => {
    const dirs = value.split(',').map((s) => s.trim()).filter(Boolean);
    updateConfig('model_search_dirs', dirs);
  };

  const handleSelectModel = (path: string) => {
    onSelectModel(path);
    updateConfig('model', path);
  };

  return (
    <div className="config-section">
      <h2 className="config-title">Configuration</h2>
      <form onSubmit={(e) => e.preventDefault()}>
        {/* ── Common ─────────────────────────────────── */}
        <div className="section-title">Common</div>
        <div className="common-grid">
          <div className="common-left">
            <ModelSelector
              models={models}
              availableForDownload={availableForDownload}
              selectedModel={selectedModel}
              onSelectModel={handleSelectModel}
              onDownload={onDownloadModel}
            />
            <DeviceSelector
              devices={devices}
              selectedDeviceId={selectedDeviceId}
              onChange={onDeviceChange}
            />
          </div>
          <div className="common-right">
            <RecordingControls
              status={status}
              uiIsRecording={uiIsRecording}
              onToggle={onToggleRecording}
            />
          </div>
        </div>

        {/* ── Audio & Transcription ──────────────────── */}
        <div className="section-title">Audio &amp; Transcription</div>
        <div className="form-group">
          <label htmlFor="hotkey">Hotkey</label>
          <input
            type="text"
            id="hotkey"
            value={config.hotkey}
            onChange={(e) => updateConfig('hotkey', e.target.value)}
            placeholder="Ctrl+Shift+T"
          />
          <div className="hint">Format: Mod+Mod+Key (e.g., Ctrl+Shift+T, Alt+T)</div>
        </div>
        <div className="form-group">
          <label htmlFor="language">Language</label>
          <input
            type="text"
            id="language"
            value={config.language || ''}
            onChange={(e) => updateConfig('language', e.target.value)}
            placeholder="auto"
          />
          <div className="hint">Use &quot;auto&quot; for automatic detection, or specify: ru, en, de, etc.</div>
        </div>

        {/* ── Whisper Model ──────────────────────────── */}
        <div className="section-title">Whisper Model</div>
        <div className="form-group">
          <label htmlFor="modelSearchDirs">Model Search Directories</label>
          <input
            type="text"
            id="modelSearchDirs"
            value={searchDirsStr}
            onChange={(e) => handleSearchDirsChange(e.target.value)}
            placeholder="~/.push-to-talk/models"
          />
          <div className="hint">Comma-separated list of directories</div>
        </div>

        {/* ── Punctuation ────────────────────────────── */}
        <PunctuationPanel config={config} updateConfig={updateConfig} status={status} />

        {/* ── Logging ────────────────────────────────── */}
        <LoggingPanel config={config} updateConfig={updateConfig} />
      </form>
    </div>
  );
}
