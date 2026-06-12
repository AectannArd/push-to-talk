import { useState } from 'react';
import type { Model, DownloadableModel } from '../types';
import type { Strings } from '../i18n/translations';
import { fmt } from '../i18n/useTranslation';

export default function ModelSelector({ models, availableForDownload, selectedModel, onSelectModel, onDownload, s }: {
  models: Model[]; availableForDownload: DownloadableModel[];
  selectedModel: string | null; onSelectModel: (path: string) => void;
  onDownload: (modelId: string) => Promise<void>; s: Strings;
}) {
  const [downloadId, setDownloadId] = useState('');
  const [downloading, setDownloading] = useState(false);
  const [msg, setMsg] = useState<{ text: string; ok: boolean } | null>(null);

  const handleDownload = async () => {
    if (!downloadId) return;
    setDownloading(true); setMsg(null);
    try {
      await onDownload(downloadId);
      setMsg({ text: fmt(s.downloadSuccess, downloadId), ok: true });
      setDownloadId('');
    } catch (e: unknown) {
      setMsg({ text: s.downloadFailed + ((e as Error)?.message || ''), ok: false });
    } finally { setDownloading(false); }
  };

  return (
    <>
      <div className="mb-3">
        <label className="form-label">{s.availableModels}</label>
        <div className="list-group" style={{ maxHeight: 200, overflowY: 'auto' }}>
          {models.length === 0 && <span className="list-group-item text-muted small">{s.scanningModels}</span>}
          {models.map((m) => (
            <button key={m.path} type="button"
              className={`list-group-item list-group-item-action d-flex align-items-center gap-2 py-1 px-2 small ${selectedModel === m.path ? 'active' : ''}`}
              onClick={() => onSelectModel(m.path)} title={m.path}>
              <span className="fw-bold">{selectedModel === m.path ? '●' : '○'}</span>
              <span className="font-monospace text-truncate flex-grow-1">{m.filename}</span>
              <span className={`${selectedModel === m.path ? '' : 'text-muted'}`}>{m.size}</span>
            </button>
          ))}
        </div>
      </div>

      <div className="mb-3">
        <label className="form-label">{s.downloadModelLabel}</label>
        <div className="input-group">
          <select className="form-select" value={downloadId} onChange={(e) => setDownloadId(e.target.value)}>
            <option value="">{s.selectModelToDownload}</option>
            {availableForDownload.map((m) => <option key={m.id} value={m.id}>{m.desc}</option>)}
          </select>
          <button className="btn btn-primary" onClick={handleDownload} disabled={downloading || !downloadId}>
            {downloading ? <><span className="spinner-border spinner-border-sm me-1" />{s.downloading}</> : s.download}
          </button>
        </div>
        <div className="form-text">{s.downloadHint}</div>
        {msg && <div className={`alert alert-${msg.ok ? 'success' : 'danger'} py-1 px-2 mt-1 small mb-0`}>{msg.text}</div>}
      </div>
    </>
  );
}
