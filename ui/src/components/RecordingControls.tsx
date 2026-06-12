import type { Status } from '../types';

interface Props {
  status: Status;
  uiIsRecording: boolean;
  onToggle: () => void;
}

export default function RecordingControls({ status, uiIsRecording, onToggle }: Props) {
  const btnClass = uiIsRecording
    ? 'btn-danger'
    : status.is_service_running
      ? 'btn-primary'
      : 'btn-secondary';

  const btnText = uiIsRecording
    ? '⏹ Stop'
    : status.is_service_running
      ? '🎤 Start Recording'
      : '▶ Start Service';

  return (
    <>
      <button type="button" className={`btn ${btnClass}`} onClick={onToggle}>
        {btnText}
      </button>
      <div className="transcription-box">
        <span className="transcription-value">
          {status.last_transcription || '---'}
        </span>
      </div>
    </>
  );
}
