interface Props {
  onConfirm: () => void;
  onCancel: () => void;
  downloading: boolean;
  downloadError: boolean;
}

export default function DownloadModal({ onConfirm, onCancel, downloading, downloadError }: Props) {
  return (
    <div className="modal-overlay active">
      <div className="modal-box">
        <h3>Punctuation Model Not Found</h3>
        <p>
          The punctuation model (<code>model.onnx</code>) is not present in your model
          directories. Would you like to download it from HuggingFace?
          <br /><br />
          <small style={{ color: '#888' }}>~1.7 GB download. Model is only needed once.</small>
        </p>
        {downloadError && (
          <p style={{ color: 'red', fontSize: 13, marginBottom: 12 }}>
            Download failed. Please check your network connection and try again.
          </p>
        )}
        <div className="modal-buttons">
          <button
            className="modal-btn-secondary"
            onClick={onCancel}
            disabled={downloading}
          >
            No, Disable
          </button>
          <button
            className="modal-btn-primary"
            onClick={onConfirm}
            disabled={downloading}
          >
            {downloading ? 'Downloading...' : 'Yes, Download'}
          </button>
        </div>
      </div>
    </div>
  );
}
