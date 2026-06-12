import { useState, useEffect, useRef, useCallback } from 'react';
import DownloadModal from './DownloadModal';
import { invoke } from '../services/tauri';
import type { Config, Status, PunctuationModelStatus } from '../types';
import type { Strings } from '../i18n/translations';

export default function PunctuationPanel({ config, updateConfig, status, s }: {
  config: Config; updateConfig: (key: keyof Config, value: unknown) => void; status: Status; s: Strings;
}) {
  const [showModal, setShowModal] = useState(false);
  const [downloading, setDownloading] = useState(false);
  const [downloadError, setDownloadError] = useState(false);
  const [puncStatus, setPuncStatus] = useState<{ found: boolean; text: string; klass: string }>({ found: false, text: '', klass: '' });
  const [progress, setProgress] = useState({ active: false, file: '', width: 0, percent: 0 });
  const progressInterval = useRef<ReturnType<typeof setInterval>>(null);

  const checkStatus = useCallback(async () => {
    try {
      const st = await invoke<PunctuationModelStatus>('check_punctuation_model', { modelSearchDirs: config.model_search_dirs || [] });
      if (st.found) {
        setPuncStatus({ found: true, text: status.is_service_running ? s.punctuationActive : s.punctuationModelFound, klass: 'success' });
      } else {
        setPuncStatus({ found: false, text: s.punctuationModelMissing, klass: 'warning' });
      }
    } catch { setPuncStatus({ found: false, text: '…', klass: '' }); }
  }, [config.model_search_dirs, status.is_service_running, s]);

  useEffect(() => { if (config.punctuation_enabled) checkStatus(); }, [status.is_service_running, checkStatus]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => { if (config.punctuation_enabled) checkStatus(); }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const handleToggle = async (enabled: boolean) => {
    updateConfig('punctuation_enabled', enabled);
    if (!enabled) { setShowModal(false); setPuncStatus({ found: false, text: '', klass: '' }); return; }
    try {
      const st = await invoke<PunctuationModelStatus>('check_punctuation_model', { modelSearchDirs: config.model_search_dirs || [] });
      if (st.found) {
        setPuncStatus({ found: true, text: status.is_service_running ? s.punctuationActive : s.punctuationModelFound, klass: 'success' });
      } else { setShowModal(true); }
    } catch { /* ignore */ }
  };

  const startDownload = async () => {
    const dirs = config.model_search_dirs || [];
    if (!dirs[0]) { alert(s.noModelDir); onModalCancel(); return; }
    setProgress({ active: true, file: 'Downloading...', width: 0, percent: 0 });
    setDownloading(true); setDownloadError(false);
    progressInterval.current = setInterval(() => {
      setProgress((p) => { if (p.width >= 90) return p; const w = Math.min(p.width + Math.random() * 3, 90); return { ...p, width: w, percent: Math.round(w) }; });
    }, 500);
    try {
      await invoke('download_punctuation_model', { targetDir: dirs[0] });
      clearInterval(progressInterval.current!);
      setProgress({ active: false, file: s.downloadCompleteRestart, width: 100, percent: 100 });
      setShowModal(false); setDownloading(false); await checkStatus();
    } catch {
      clearInterval(progressInterval.current!);
      setProgress((p) => ({ ...p, active: false, file: s.downloadFailedLabel }));
      setDownloading(false); setDownloadError(true);
    }
  };

  const onModalCancel = () => { setShowModal(false); updateConfig('punctuation_enabled', false); };

  return (
    <div className="card mb-3">
      <div className="card-header py-2"><strong>{s.punctuationRestoration}</strong></div>
      <div className="card-body">
        <div className="form-check form-switch mb-2">
          <input className="form-check-input" type="checkbox" id="punctuationEnabled"
            checked={config.punctuation_enabled} onChange={(e) => handleToggle(e.target.checked)} />
          <label className="form-check-label" htmlFor="punctuationEnabled">{s.enablePunctuation}</label>
        </div>
        <div className="form-text">{s.punctuationHint}</div>
        {config.punctuation_enabled && (
          <div className="mt-2">
            {puncStatus.text && <div className={`alert alert-${puncStatus.klass} py-1 px-2 small mb-2`}>{puncStatus.text}</div>}
            {puncStatus.klass === 'warning' && (
              <button className="btn btn-primary btn-sm" onClick={startDownload} disabled={downloading}>
                {downloading ? <><span className="spinner-border spinner-border-sm me-1" />{s.downloading}</> : s.downloadPunctuationModel}
              </button>
            )}
            {progress.active && (
              <div className="mt-2">
                <div className="small text-muted mb-1">{progress.file}</div>
                <div className="progress" style={{ height: 8 }}>
                  <div className="progress-bar" style={{ width: progress.width + '%' }} />
                </div>
                <small className="text-muted">{progress.percent}%</small>
              </div>
            )}
          </div>
        )}
      </div>
      {showModal && <DownloadModal onConfirm={startDownload} onCancel={onModalCancel} downloading={downloading} downloadError={downloadError} s={s} />}
    </div>
  );
}
