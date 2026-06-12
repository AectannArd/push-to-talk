import type { Config } from '../types';
import type { Strings } from '../i18n/translations';

interface Props {
  config: Config;
  updateConfig: (key: keyof Config, value: unknown) => void;
  s: Strings;
}

export default function LoggingPanel({ config, updateConfig, s }: Props) {
  return (
    <>
      <div className="section-title">{s.logging}</div>
      <div className="form-row">
        <div className="form-group">
          <label htmlFor="logDir">{s.logDirectory}</label>
          <input
            type="text"
            id="logDir"
            value={config.log_dir}
            onChange={(e) => updateConfig('log_dir', e.target.value)}
          />
        </div>
        <div className="form-group">
          <label htmlFor="logLevel">{s.logLevel}</label>
          <select
            id="logLevel"
            value={config.log_level}
            onChange={(e) => updateConfig('log_level', e.target.value)}
          >
            <option value="trace">{s.trace}</option>
            <option value="debug">{s.debug}</option>
            <option value="info">{s.info}</option>
            <option value="warn">{s.warn}</option>
            <option value="error">{s.error}</option>
          </select>
        </div>
      </div>
      <div className="form-row">
        <div className="form-group">
          <label htmlFor="logFormat">{s.logFormat}</label>
          <select
            id="logFormat"
            value={config.log_format}
            onChange={(e) => updateConfig('log_format', e.target.value)}
          >
            <option value="text">{s.text}</option>
            <option value="json">{s.json}</option>
          </select>
        </div>
        <div className="form-group">
          <label htmlFor="logRetention">{s.logRetention}</label>
          <input
            type="number"
            id="logRetention"
            value={config.log_retention_hours}
            onChange={(e) => updateConfig('log_retention_hours', parseInt(e.target.value, 10) || 1)}
            min={1}
            max={720}
          />
        </div>
      </div>
      <div className="hint">{s.changesAutoSaved}</div>
    </>
  );
}
