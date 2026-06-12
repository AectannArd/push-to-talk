import ModelSelector from './ModelSelector';
import DeviceSelector from './DeviceSelector';
import RecordingControls from './RecordingControls';
import PunctuationPanel from './PunctuationPanel';
import LoggingPanel from './LoggingPanel';
import type { Config, Status, Model, DownloadableModel, Device } from '../types';
import type { Strings } from '../i18n/translations';

interface Props {
  config: Config; updateConfig: (key: keyof Config, value: unknown) => void;
  models: Model[]; availableForDownload: DownloadableModel[];
  selectedModel: string | null; onSelectModel: (path: string) => void;
  onDownloadModel: (modelId: string) => Promise<void>;
  devices: Device[]; selectedDeviceId: string; onDeviceChange: (id: string) => void;
  status: Status; uiIsRecording: boolean; onToggleRecording: () => void;
  s: Strings;
}

export default function ConfigForm(props: Props) {
  const { config, updateConfig, s } = props;
  const searchDirsStr = (config.model_search_dirs || []).join(', ');

  return (
    <div className="card">
      <div className="card-header py-2"><strong>{s.configuration}</strong></div>
      <div className="card-body">
        <form onSubmit={(e) => e.preventDefault()}>
          {/* Common section */}
          <h6 className="text-primary mb-3">{s.common}</h6>
          <div className="row g-3 mb-3">
            <div className="col-md-7">
              <ModelSelector models={props.models} availableForDownload={props.availableForDownload}
                selectedModel={props.selectedModel} onSelectModel={props.onSelectModel}
                onDownload={props.onDownloadModel} s={s} />
              <DeviceSelector devices={props.devices} selectedDeviceId={props.selectedDeviceId}
                onChange={props.onDeviceChange} s={s} />
            </div>
            <div className="col-md-5">
              <RecordingControls status={props.status} uiIsRecording={props.uiIsRecording}
                onToggle={props.onToggleRecording} s={s} />
            </div>
          </div>

          {/* Audio & Transcription */}
          <h6 className="text-primary border-top pt-3 mb-3">{s.audioTranscription}</h6>
          <div className="row g-2 mb-3">
            <div className="col-md-6">
              <label className="form-label">{s.hotkey}</label>
              <input type="text" className="form-control" value={config.hotkey}
                onChange={(e) => updateConfig('hotkey', e.target.value)} placeholder="Ctrl+Shift+T" />
              <div className="form-text">{s.hotkeyHint}</div>
            </div>
            <div className="col-md-6">
              <label className="form-label">{s.language}</label>
              <input type="text" className="form-control" value={config.language || ''}
                onChange={(e) => updateConfig('language', e.target.value)} placeholder="auto" />
              <div className="form-text">{s.languageHint}</div>
            </div>
          </div>

          {/* Whisper Model */}
          <h6 className="text-primary border-top pt-3 mb-3">{s.whisperModel}</h6>
          <div className="mb-3">
            <label className="form-label">{s.modelSearchDirs}</label>
            <input type="text" className="form-control" value={searchDirsStr}
              onChange={(e) => updateConfig('model_search_dirs', e.target.value.split(',').map((d) => d.trim()).filter(Boolean))}
              placeholder="~/.push-to-talk/models" />
            <div className="form-text">{s.modelSearchDirsHint}</div>
          </div>

          <PunctuationPanel config={config} updateConfig={updateConfig} status={props.status} s={s} />
          <LoggingPanel config={config} updateConfig={updateConfig} s={s} />
        </form>
      </div>
    </div>
  );
}
