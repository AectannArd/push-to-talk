import type { Status } from '../types';
import type { Strings } from '../i18n/translations';

interface Props { status: Status; uiIsRecording: boolean; onToggle: () => void; s: Strings }

export default function RecordingControls({ status, uiIsRecording, onToggle, s }: Props) {
  const variant = uiIsRecording ? 'danger' : status.is_service_running ? 'primary' : 'secondary';
  const label = uiIsRecording ? s.stop : status.is_service_running ? s.startRecording : s.startService;
  return (
    <div className="d-flex flex-column gap-2 h-100">
      <button className={`btn btn-${variant} w-100`} onClick={onToggle}>{label}</button>
      <div className="form-control bg-body-tertiary flex-grow-1 d-flex align-items-center" style={{ minHeight: 80 }}>
        <span className="text-break small">{status.last_transcription || '---'}</span>
      </div>
    </div>
  );
}
