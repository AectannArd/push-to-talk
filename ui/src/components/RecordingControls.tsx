import type { Status } from '../types';
import type { Strings } from '../i18n/translations';

interface Props {
  status: Status;
  uiIsRecording: boolean;
  onToggle: () => void;
  s: Strings;
}

export default function RecordingControls({ status, uiIsRecording, onToggle, s }: Props) {
  const btnClass = uiIsRecording
    ? 'btn-danger'
    : status.is_service_running
      ? 'btn-primary'
      : 'btn-secondary';

  const btnText = uiIsRecording
    ? s.stop
    : status.is_service_running
      ? s.startRecording
      : s.startService;

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
