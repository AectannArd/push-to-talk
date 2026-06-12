import { useState } from 'react';
import type { Model, DownloadableModel } from '../types';
import type { Strings } from '../i18n/translations';
import { fmt } from '../i18n/useTranslation';

interface Props {
  models: Model[];
  availableForDownload: DownloadableModel[];
  selectedModel: string | null;
  onSelectModel: (path: string) => void;
  onDownload: (modelId: string) => Promise<void>;
  s: Strings;
}

export default function ModelSelector({
  models,
  availableForDownload,
  selectedModel,
  onSelectModel,
  onDownload,
  s,
}: Props) {
  const [downloadId, setDownloadId] = useState('');
  const [downloading, setDownloading] = useState(false);
  const [success, setSuccess] = useState('');
  const [error, setError] = useState('');

  const handleDownload = async () => {
    if (!downloadId) return;
    setDownloading(true);
    setSuccess('');
    setError('');
    try {
      await onDownload(downloadId);
      setSuccess(fmt(s.downloadSuccess, downloadId));
      setDownloadId('');
    } catch (e: unknown) {
      setError(s.downloadFailed + ((e as Error)?.message || String(e)));
    } finally {
      setDownloading(false);
    }
  };

  return (
    <>
      <div className="form-group">
        <label>{s.availableModels}</label>
        <div className="model-list">
          {models.length === 0 && (
            <p style={{ color: '#888' }}>{s.scanningModels}</p>
          )}
          {models.map((m) => (
            <div
              key={m.path}
              className={`model-item${selectedModel === m.path ? ' model-item-selected' : ''}`}
              onClick={() => onSelectModel(m.path)}
              title={m.path}
            >
              <span className="model-radio">
                {selectedModel === m.path ? '●' : '○'}
              </span>
              <span className="model-name">{m.filename}</span>
              <span className="model-size">{m.size}</span>
            </div>
          ))}
        </div>
      </div>

      <div className="form-group">
        <label>{s.downloadModelLabel}</label>
        <div className="model-download-row">
          <select
            className="model-select"
            value={downloadId}
            onChange={(e) => setDownloadId(e.target.value)}
          >
            <option value="">{s.selectModelToDownload}</option>
            {availableForDownload.map((m) => (
              <option key={m.id} value={m.id}>
                {m.desc}
              </option>
            ))}
          </select>
          <button
            type="button"
            className="btn btn-primary"
            onClick={handleDownload}
            disabled={downloading || !downloadId}
          >
            {downloading ? s.downloading : s.download}
          </button>
        </div>
        <div className="hint">{s.downloadHint}</div>
        {success && <p style={{ color: 'green' }}>{success}</p>}
        {error && <p style={{ color: 'red' }}>{error}</p>}
      </div>
    </>
  );
}
