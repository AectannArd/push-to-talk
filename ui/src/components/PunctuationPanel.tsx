import { useState, useEffect, useRef, useCallback } from 'react';
import DownloadModal from './DownloadModal';
import { invoke } from '../services/tauri';
import type { Config, Status, PunctuationModelStatus } from '../types';
import type { Strings } from '../i18n/translations';

interface Props {
  config: Config;
  updateConfig: (key: keyof Config, value: unknown) => void;
  status: Status;
  s: Strings;
}

export default function PunctuationPanel({ config, updateConfig, status, s }: Props) {
  const [showModal, setShowModal] = useState(false);
  const [downloading, setDownloading] = useState(false);
  const [downloadError, setDownloadError] = useState(false);
  const [puncStatus, setPuncStatus] = useState({ found: false, text: '', klass: '' });
  const [progress, setProgress] = useState({ active: false, file: '', width: 0, percent: 0 });
  const progressInterval = useRef<ReturnType<typeof setInterval>>(null);

  const checkStatus = useCallback(async () => {
    const dirs = config.model_search_dirs || [];
    try {
      const st = await invoke<PunctuationModelStatus>('check_punctuation_model', {
        modelSearchDirs: dirs,
      });
      if (st.found) {
        setPuncStatus({
          found: true,
          text: status.is_service_running ? s.punctuationActive : s.punctuationModelFound,
          klass: 'loaded',
        });
      } else {
        setPuncStatus({ found: false, text: s.punctuationModelMissing, klass: 'missing' });
      }
    } catch {
      setPuncStatus({ found: false, text: '…', klass: '' });
    }
  }, [config.model_search_dirs, status.is_service_running, s]);

  useEffect(() => {
    if (config.punctuation_enabled) checkStatus();
  }, [status.is_service_running, checkStatus]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (config.punctuation_enabled) checkStatus();
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const handleToggle = async (enabled: boolean) => {
    updateConfig('punctuation_enabled', enabled);
    if (!enabled) {
      setShowModal(false);
      setPuncStatus({ found: false, text: '', klass: '' });
      return;
    }
    try {
      const dirs = config.model_search_dirs || [];
      const st = await invoke<PunctuationModelStatus>('check_punctuation_model', { modelSearchDirs: dirs });
      if (st.found) {
        setPuncStatus({
          found: true,
          text: status.is_service_running ? s.punctuationActive : s.punctuationModelFound,
          klass: 'loaded',
        });
      } else {
        setShowModal(true);
      }
    } catch { /* ignore */ }
  };

  const startDownload = async () => {
    const dirs = config.model_search_dirs || [];
    const targetDir = dirs[0];
    if (!targetDir) {
      alert(s.noModelDir);
      onModalCancel();
      return;
    }
    setProgress({ active: true, file: 'Downloading model.onnx + tokenizer.json...', width: 0, percent: 0 });
    setDownloading(true);
    setDownloadError(false);
    progressInterval.current = setInterval(() => {
      setProgress((p) => {
        if (p.width >= 90) return p;
        const w = Math.min(p.width + Math.random() * 3, 90);
        return { ...p, width: w, percent: Math.round(w) };
      });
    }, 500);
    try {
      await invoke('download_punctuation_model', { targetDir });
      clearInterval(progressInterval.current!);
      setProgress({ active: false, file: s.downloadCompleteRestart, width: 100, percent: 100 });
      setShowModal(false);
      setDownloading(false);
      await checkStatus();
    } catch {
      clearInterval(progressInterval.current!);
      setProgress((p) => ({ ...p, active: false, file: s.downloadFailedLabel }));
      setDownloading(false);
      setDownloadError(true);
    }
  };

  const onModalCancel = () => {
    setShowModal(false);
    updateConfig('punctuation_enabled', false);
  };

  return (
    <>
      <div className="section-title">{s.punctuationRestoration}</div>
      <div className="form-group">
        <div className="toggle-row">
          <label className="toggle-label" htmlFor="punctuationEnabled">
            {s.enablePunctuation}
          </label>
          <label className="toggle-switch">
            <input
              type="checkbox"
              id="punctuationEnabled"
              checked={config.punctuation_enabled}
              onChange={(e) => handleToggle(e.target.checked)}
            />
            <span className="toggle-slider" />
          </label>
        </div>
        <div className="hint">{s.punctuationHint}</div>
      </div>

      {config.punctuation_enabled && (
        <div className="form-group">
          <div className={`model-status ${puncStatus.klass}`}>{puncStatus.text}</div>
          <div style={{ marginTop: 8 }}>
            {puncStatus.klass === 'missing' && (
              <button type="button" className="btn btn-primary" onClick={startDownload} disabled={downloading}>
                {downloading ? s.downloading : s.downloadPunctuationModel}
              </button>
            )}
            <div className={`download-progress${progress.active ? ' active' : ''}`}>
              <div className="progress-file">{progress.file}</div>
              <div className="progress-bar">
                <div className="progress-bar-fill" style={{ width: `${progress.width}%` }} />
              </div>
              <p style={{ fontSize: 12, color: '#888' }}>{progress.percent}%</p>
            </div>
          </div>
        </div>
      )}

      {showModal && (
        <DownloadModal
          onConfirm={startDownload}
          onCancel={onModalCancel}
          downloading={downloading}
          downloadError={downloadError}
          s={s}
        />
      )}
    </>
  );
}
