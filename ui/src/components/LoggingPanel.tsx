import type { Config } from '../types';
import type { Strings } from '../i18n/translations';

export default function LoggingPanel({ config, updateConfig, s }: {
  config: Config; updateConfig: (key: keyof Config, value: unknown) => void; s: Strings;
}) {
  return (
    <div className="card mb-3">
      <div className="card-header py-2"><strong>{s.logging}</strong></div>
      <div className="card-body">
        <div className="row g-2">
          <div className="col-md-6">
            <label className="form-label">{s.logDirectory}</label>
            <input type="text" className="form-control" value={config.log_dir} onChange={(e) => updateConfig('log_dir', e.target.value)} />
          </div>
          <div className="col-md-3">
            <label className="form-label">{s.logLevel}</label>
            <select className="form-select" value={config.log_level} onChange={(e) => updateConfig('log_level', e.target.value)}>
              <option value="trace">{s.trace}</option>
              <option value="debug">{s.debug}</option>
              <option value="info">{s.info}</option>
              <option value="warn">{s.warn}</option>
              <option value="error">{s.error}</option>
            </select>
          </div>
          <div className="col-md-3">
            <label className="form-label">{s.logFormat}</label>
            <select className="form-select" value={config.log_format} onChange={(e) => updateConfig('log_format', e.target.value)}>
              <option value="text">{s.text}</option>
              <option value="json">{s.json}</option>
            </select>
          </div>
        </div>
        <div className="row g-2 mt-2">
          <div className="col-md-3">
            <label className="form-label">{s.logRetention}</label>
            <input type="number" className="form-control" value={config.log_retention_hours} min={1} max={720}
              onChange={(e) => updateConfig('log_retention_hours', parseInt(e.target.value, 10) || 1)} />
          </div>
        </div>
        <div className="form-text">{s.changesAutoSaved}</div>
      </div>
    </div>
  );
}
