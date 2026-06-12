/**
 * Tauri v2 IPC wrapper.
 *
 * Uses the raw `window.__TAURI__.core.invoke` with explicit .bind()
 * because ES modules run in strict mode where bare function calls
 * lose the `this` context that Tauri's IPC internals require.
 */

type InvokeFn = (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;

let _invoke: InvokeFn | null = null;

function resolveInvoke(): InvokeFn {
  if (_invoke) return _invoke;

  const raw = window.__TAURI__?.core?.invoke;
  if (raw) {
    // .bind() preserves `this` context — critical in ES module strict mode
    _invoke = raw.bind(window.__TAURI__!.core) as InvokeFn;
    return _invoke;
  }

  throw new Error('window.__TAURI__.core.invoke not available');
}

export async function invoke<T = unknown>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const fn = resolveInvoke();
  return (args !== undefined ? fn(cmd, args) : fn(cmd)) as Promise<T>;
}

export function forwardLog(level: string, message: string): void {
  invoke('frontend_log', { level, message }).catch(() => {});
}
