import type { Status } from '../types';

interface Props {
  status: Status;
}

export default function StatusBar({ status }: Props) {
  const indicatorClass =
    (status.is_recording && 'recording') ||
    (status.is_service_running && 'running') ||
    '';

  const statusText = status.is_recording
    ? 'Recording...'
    : status.is_service_running
      ? 'Ready (press button or hotkey to record)'
      : 'Service stopped';

  return (
    <div className="status-section">
      <div className="status-bar">
        <div className={`status-indicator ${indicatorClass}`} />
        <span className="status-text">{statusText}</span>
      </div>
      <div className="session-info">
        <div className="session-item">
          <span className="session-label">Service:</span>
          <span className="session-value">
            {status.is_service_running ? 'Running' : 'Stopped'}
          </span>
        </div>
        <div className="session-item">
          <span className="session-label">Recording:</span>
          <span className="session-value">
            {status.is_recording ? 'Yes' : 'No'}
          </span>
        </div>
      </div>
    </div>
  );
}
