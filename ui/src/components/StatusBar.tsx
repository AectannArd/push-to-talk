import type { Status } from '../types';
import type { Strings } from '../i18n/translations';

interface Props {
  status: Status;
  s: Strings;
}

export default function StatusBar({ status, s }: Props) {
  const indicatorClass =
    (status.is_recording && 'recording') ||
    (status.is_service_running && 'running') ||
    '';

  const statusText = status.is_recording
    ? s.recording
    : status.is_service_running
      ? s.ready
      : s.serviceStopped;

  return (
    <div className="status-section">
      <div className="status-bar">
        <div className={`status-indicator ${indicatorClass}`} />
        <span className="status-text">{statusText}</span>
      </div>
      <div className="session-info">
        <div className="session-item">
          <span className="session-label">{s.serviceLabel}</span>
          <span className="session-value">
            {status.is_service_running ? s.running : s.stopped}
          </span>
        </div>
        <div className="session-item">
          <span className="session-label">{s.recordingLabel}</span>
          <span className="session-value">
            {status.is_recording ? s.yes : s.no}
          </span>
        </div>
      </div>
    </div>
  );
}
