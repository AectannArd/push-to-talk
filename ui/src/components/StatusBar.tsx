import type { Status } from '../types';
import type { Strings } from '../i18n/translations';

export default function StatusBar({ status, s }: { status: Status; s: Strings }) {
  const bg = status.is_recording ? 'danger' : status.is_service_running ? 'success' : 'secondary';
  const text = status.is_recording ? s.recording : status.is_service_running ? s.ready : s.serviceStopped;

  return (
    <div className="card mb-3">
      <div className="card-body py-2">
        <div className="d-flex align-items-center gap-2 mb-2">
          <span style={{ width: 12, height: 12, borderRadius: '50%', display: 'inline-block', backgroundColor: `var(--bs-${bg})`, flexShrink: 0 }} />
          <span className="fw-semibold">{text}</span>
        </div>
        <div className="row g-2 small">
          <div className="col-6">
            <div className="bg-body-tertiary rounded p-2">
              <span className="text-muted">{s.serviceLabel}</span>
              <span className="float-end fw-semibold">{status.is_service_running ? s.running : s.stopped}</span>
            </div>
          </div>
          <div className="col-6">
            <div className="bg-body-tertiary rounded p-2">
              <span className="text-muted">{s.recordingLabel}</span>
              <span className="float-end fw-semibold">{status.is_recording ? s.yes : s.no}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
