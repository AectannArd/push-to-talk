export default function DownloadModal({ onConfirm, onCancel, downloading, downloadError, s }: {
  onConfirm: () => void; onCancel: () => void; downloading: boolean; downloadError: boolean; s: { punctuationModelNotFound: string; modalDescription: string; modalSize: string; yesDownload: string; noDisable: string; downloading: string; downloadFailedLabel: string };
}) {
  return (
    <div className="modal d-block" tabIndex={-1} style={{ backgroundColor: 'rgba(0,0,0,0.5)' }}>
      <div className="modal-dialog modal-dialog-centered">
        <div className="modal-content">
          <div className="modal-header">
            <h6 className="modal-title">{s.punctuationModelNotFound}</h6>
          </div>
          <div className="modal-body">
            <p className="small">{s.modalDescription}</p>
            <small className="text-muted">{s.modalSize}</small>
            {downloadError && <div className="alert alert-danger py-1 px-2 mt-2 small mb-0">{s.downloadFailedLabel}</div>}
          </div>
          <div className="modal-footer">
            <button className="btn btn-secondary btn-sm" onClick={onCancel} disabled={downloading}>{s.noDisable}</button>
            <button className="btn btn-primary btn-sm" onClick={onConfirm} disabled={downloading}>
              {downloading ? <><span className="spinner-border spinner-border-sm me-1" />{s.downloading}</> : s.yesDownload}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
