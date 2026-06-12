import ModelSelector from './ModelSelector';
import DeviceSelector from './DeviceSelector';
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
  status: Status;
  activeTab: string; onTabChange: (tab: string) => void;
  s: Strings;
}

type Tab = 'audio' | 'models' | 'logging';

const TAB_ITEMS: { key: Tab; label: (s: Strings) => string }[] = [
  { key: 'audio', label: (s) => s.audioTranscription },
  { key: 'models', label: (s) => s.whisperModel },
  { key: 'logging', label: (s) => s.logging },
];

export default function ConfigForm(props: Props) {
  const { config, updateConfig, s } = props;
  const active = props.activeTab as Tab;
  const setActive = (t: string) => props.onTabChange(t);
  const searchDirsStr = (config.model_search_dirs || []).join(', ');

  const handleSearchDirsChange = (value: string) => {
    updateConfig('model_search_dirs', value.split(',').map((d) => d.trim()).filter(Boolean));
  };

  return (
    <div className="card">
      <div className="card-header p-0">
        <ul className="nav nav-tabs nav-fill" role="tablist">
          {TAB_ITEMS.map(({ key, label }) => (
            <li className="nav-item" key={key} role="presentation">
              <button
                className={`nav-link${active === key ? ' active' : ''}`}
                type="button"
                onClick={() => setActive(key)}
              >
                {label(s)}
              </button>
            </li>
          ))}
        </ul>
      </div>
      <div className="card-body">
        <form onSubmit={(e) => e.preventDefault()}>
          {active === 'audio' && (
            <>
              <DeviceSelector devices={props.devices} selectedDeviceId={props.selectedDeviceId}
                onChange={props.onDeviceChange} s={s} />
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
            </>
          )}

          {active === 'models' && (
            <>
              <ModelSelector
                models={props.models} availableForDownload={props.availableForDownload}
                selectedModel={props.selectedModel} onSelectModel={(path) => { props.onSelectModel(path); updateConfig('model', path); }}
                onDownload={props.onDownloadModel} s={s} />
              <div className="mb-3">
                <label className="form-label">{s.modelSearchDirs}</label>
                <input type="text" className="form-control" value={searchDirsStr}
                  onChange={(e) => handleSearchDirsChange(e.target.value)}
                  placeholder="~/.push-to-talk/models" />
                <div className="form-text">{s.modelSearchDirsHint}</div>
              </div>
              <hr />
              <PunctuationPanel config={config} updateConfig={updateConfig} status={props.status} s={s} />
            </>
          )}

          {active === 'logging' && (
            <LoggingPanel config={config} updateConfig={updateConfig} s={s} />
          )}
        </form>
      </div>
    </div>
  );
}
