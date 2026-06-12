import { useState, useEffect } from 'react';

/** Poll until window.__TAURI__ is available. */
export function useTauriReady(): boolean {
  const [ready, setReady] = useState(
    () => !!(window.__TAURI__?.core?.invoke),
  );

  useEffect(() => {
    if (ready) return;
    let attempts = 0;
    const id = setInterval(() => {
      attempts++;
      if (window.__TAURI__?.core?.invoke) {
        clearInterval(id);
        setReady(true);
      } else if (attempts > 50) {
        clearInterval(id);
      }
    }, 100);
    return () => clearInterval(id);
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  return ready;
}
