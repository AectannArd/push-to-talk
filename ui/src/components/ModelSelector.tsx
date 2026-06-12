import { useState } from 'react';
import type { Model, DownloadableModel } from '../types';

interface Props {
  models: Model[];
  availableForDownload: DownloadableModel[];
  selectedModel: string | null;
  onSelectModel: (path: string) => void;
  onDownload: (modelId: string) => Promise<void>;
}

export default function ModelSelector({
  models,
  availableForDownload,
  selectedModel,
  onSelectModel,
  onDownload,
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
      setSuccess(`Model ${downloadId} downloaded successfully!`);
      setDownloadId('');
    } catch (e: unknown) {
      setError('Download failed: ' + ((e as Error)?.message || String(e)));
    } finally {
      setDownloading(false);
    }
  };

  return (
    <>
      <div className="form-group">
        <label>Available Models</label>
        <div className="model-list">
          {models.length === 0 && (
            <p style={{ color: '#888' }}>Scanning model directories...</p>
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
        <label>Download Model</label>
        <div className="model-download-row">
          <select
            className="model-select"
            value={downloadId}
            onChange={(e) => setDownloadId(e.target.value)}
          >
            <option value="">Select a model to download...</option>
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
            {downloading ? '⏳ Downloading...' : '⬇ Download'}
          </button>
        </div>
        <div className="hint">
          Downloads from Hugging Face to the first directory in Model Search Directories
        </div>
        {success && <p style={{ color: 'green' }}>{success}</p>}
        {error && <p style={{ color: 'red' }}>{error}</p>}
      </div>
    </>
  );
}
