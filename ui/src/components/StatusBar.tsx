import type { Status } from '../types';
import type { Strings } from '../i18n/translations';

interface Props {
  status: Status;
  uiIsRecording: boolean;
  onToggle: () => void;
  s: Strings;
}

export default function StatusBar({ status, uiIsRecording, onToggle, s }: Props) {
  const dotBg = status.is_recording ? 'var(--bs-danger)' : status.is_service_running ? 'var(--bs-success)' : 'var(--bs-secondary)';
  const text = status.is_recording ? s.recording : status.is_service_running ? s.ready : s.serviceStopped;

  const btnVariant = uiIsRecording ? 'danger' : status.is_service_running ? 'primary' : 'secondary';
  const btnLabel = uiIsRecording ? s.stop : status.is_service_running ? s.startRecording : s.startService;

  return (
    <div className="card mb-3 status-card">
      <div className="card-body py-2">
        <div className="row g-2 align-items-center">
          {/* Left: status indicator + text + service/recording info */}
          <div className="col-md-7">
            <div className="d-flex align-items-center gap-2 mb-2">
              <span style={{ width: 12, height: 12, borderRadius: '50%', display: 'inline-block', backgroundColor: dotBg, flexShrink: 0 }} />
              <span className="fw-semibold small">{text}</span>
            </div>
            <div className="row g-2">
              <div className="col-6">
                <div className="bg-body-tertiary rounded p-2">
                  <span className="text-muted small">{s.serviceLabel}</span>
                  <span className="float-end fw-semibold small">{status.is_service_running ? s.running : s.stopped}</span>
                </div>
              </div>
              <div className="col-6">
                <div className="bg-body-tertiary rounded p-2">
                  <span className="text-muted small">{s.recordingLabel}</span>
                  <span className="float-end fw-semibold small">{status.is_recording ? s.yes : s.no}</span>
                </div>
              </div>
            </div>
          </div>
          {/* Right: button + transcription */}
          <div className="col-md-5">
            <div className="d-flex flex-column gap-2">
              <button className={`btn btn-${btnVariant} btn-sm w-100`} onClick={onToggle}>{btnLabel}</button>
              <div className="form-control form-control-sm bg-body-tertiary" style={{ minHeight: 56, fontSize: '0.8rem' }}>
                {status.last_transcription || '---'}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
