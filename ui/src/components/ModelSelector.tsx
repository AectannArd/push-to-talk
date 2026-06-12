import { useState, useRef } from 'react';
import type { Model, DownloadableModel } from '../types';
import type { Strings } from '../i18n/translations';
import { fmt } from '../i18n/useTranslation';

export default function ModelSelector({ models, availableForDownload, selectedModel, onSelectModel, onDownload, s }: {
  models: Model[]; availableForDownload: DownloadableModel[];
  selectedModel: string | null; onSelectModel: (path: string) => void;
  onDownload: (modelId: string) => Promise<string | undefined>; s: Strings;
}) {
  const [downloadId, setDownloadId] = useState('');
  const [downloading, setDownloading] = useState(false);
  const [msg, setMsg] = useState<{ text: string; ok: boolean } | null>(null);
  const [progress, setProgress] = useState({ active: false, file: '', width: 0, percent: 0 });
  const progressInterval = useRef<ReturnType<typeof setInterval>>(null);

  const handleDownload = async () => {
    if (!downloadId) return;
    const entry = availableForDownload.find((m) => m.id === downloadId);
    setDownloading(true); setMsg(null);
    setProgress({ active: true, file: entry ? entry.name : '', width: 0, percent: 0 });

    progressInterval.current = setInterval(() => {
      setProgress((p) => { if (p.width >= 90) return p; const w = Math.min(p.width + Math.random() * 3, 90); return { ...p, width: w, percent: Math.round(w) }; });
    }, 500);

    try {
      const path = await onDownload(downloadId);
      clearInterval(progressInterval.current!);
      setProgress({ active: false, file: '', width: 100, percent: 100 });
      if (path) onSelectModel(path);
      setMsg({ text: fmt(s.downloadSuccess, downloadId), ok: true });
      setDownloadId('');
    } catch (e: unknown) {
      clearInterval(progressInterval.current!);
      setProgress((p) => ({ ...p, active: false }));
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
            {downloading ? <><span className="spinner-border spinner-border-sm me-1" />{s.downloading}</> : <><img src="/download-icon-sm.png" alt="" width={18} height={18} style={{ marginRight: 4, verticalAlign: 'middle' }} />{s.download}</>}
          </button>
        </div>
        <div className="form-text">{s.downloadHint}</div>

        {progress.active && (
          <div className="mt-2">
            <div className="small text-muted mb-1">{progress.file}</div>
            <div className="progress" style={{ height: 8 }}>
              <div className="progress-bar" style={{ width: progress.width + '%' }} />
            </div>
            <small className="text-muted">{progress.percent}%</small>
          </div>
        )}

        {msg && <div className={`alert alert-${msg.ok ? 'success' : 'danger'} py-1 px-2 mt-1 small mb-0`}>{msg.text}</div>}
      </div>
    </>
  );
}
