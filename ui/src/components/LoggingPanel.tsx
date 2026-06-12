import type { Config } from '../types';

interface Props {
  config: Config;
  updateConfig: (key: keyof Config, value: unknown) => void;
}

export default function LoggingPanel({ config, updateConfig }: Props) {
  return (
    <>
      <div className="section-title">Logging</div>
      <div className="form-row">
        <div className="form-group">
          <label htmlFor="logDir">Log Directory</label>
          <input
            type="text"
            id="logDir"
            value={config.log_dir}
            onChange={(e) => updateConfig('log_dir', e.target.value)}
          />
        </div>
        <div className="form-group">
          <label htmlFor="logLevel">Log Level</label>
          <select
            id="logLevel"
            value={config.log_level}
            onChange={(e) => updateConfig('log_level', e.target.value)}
          >
            <option value="trace">trace</option>
            <option value="debug">debug</option>
            <option value="info">info</option>
            <option value="warn">warn</option>
            <option value="error">error</option>
          </select>
        </div>
      </div>
      <div className="form-row">
        <div className="form-group">
          <label htmlFor="logFormat">Log Format</label>
          <select
            id="logFormat"
            value={config.log_format}
            onChange={(e) => updateConfig('log_format', e.target.value)}
          >
            <option value="text">Text</option>
            <option value="json">JSON</option>
          </select>
        </div>
        <div className="form-group">
          <label htmlFor="logRetention">Log Retention (hours)</label>
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
      <div className="hint">Changes are saved automatically</div>
    </>
  );
}
