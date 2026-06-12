import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '../services/tauri';
import type { Status } from '../types';

const POLL_MS = 2000;

export function useStatus(ready: boolean) {
  const [status, setStatus] = useState<Status>({
    is_recording: false,
    is_service_running: false,
    hotkey: '',
    language: null,
    last_transcription: null,
  });
  const [uiIsRecording, setUiIsRecording] = useState(false);
  const uiTranscriptionPending = useRef(false);
  const lastDisplayed = useRef<string | null>(null);

  useEffect(() => {
    if (!ready) return;
    const poll = () => {
      invoke<Status>('get_status')
        .then((s) => {
          setStatus((prev) => {
            const next = { ...s };
            if (
              uiTranscriptionPending.current &&
              s.last_transcription &&
              s.last_transcription !== lastDisplayed.current
            ) {
              next.last_transcription = s.last_transcription;
              lastDisplayed.current = s.last_transcription;
              uiTranscriptionPending.current = false;
            } else {
              next.last_transcription = prev.last_transcription;
            }
            return next;
          });
        })
        .catch(() => {});
    };
    poll();
    const id = setInterval(poll, POLL_MS);
    return () => clearInterval(id);
  }, [ready]);

  const toggleRecording = useCallback(async () => {
    if (!status.is_service_running) {
      await invoke('start_service');
      setUiIsRecording(false);
      return;
    }
    await invoke('trigger_recording');
    setUiIsRecording((prev) => {
      if (prev) uiTranscriptionPending.current = true;
      return !prev;
    });
  }, [status.is_service_running]);

  return { status, uiIsRecording, toggleRecording };
}
