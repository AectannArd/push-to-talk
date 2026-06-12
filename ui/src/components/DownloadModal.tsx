import type { Strings } from '../i18n/translations';

interface Props {
  onConfirm: () => void;
  onCancel: () => void;
  downloading: boolean;
  downloadError: boolean;
  s: Strings;
}

export default function DownloadModal({ onConfirm, onCancel, downloading, downloadError, s }: Props) {
  return (
    <div className="modal-overlay active">
      <div className="modal-box">
        <h3>{s.punctuationModelNotFound}</h3>
        <p>
          {s.modalDescription}
          <br /><br />
          <small style={{ color: '#888' }}>{s.modalSize}</small>
        </p>
        {downloadError && (
          <p style={{ color: 'red', fontSize: 13, marginBottom: 12 }}>
            {s.downloadFailedLabel}
          </p>
        )}
        <div className="modal-buttons">
          <button
            className="modal-btn-secondary"
            onClick={onCancel}
            disabled={downloading}
          >
            {s.noDisable}
          </button>
          <button
            className="modal-btn-primary"
            onClick={onConfirm}
            disabled={downloading}
          >
            {downloading ? s.downloading : s.yesDownload}
          </button>
        </div>
      </div>
    </div>
  );
}
