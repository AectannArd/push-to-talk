import { useState, useRef, useEffect, useMemo } from 'react';
import FlagIcon from './FlagIcon';

interface Props {
  value: string;
  onChange: (code: string) => void;
  options: { code: string; label: string }[];
}

export default function LangSelect({ value, onChange, options }: Props) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const ref = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
        setQuery('');
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, []);

  useEffect(() => {
    if (open) inputRef.current?.focus();
  }, [open]);

  const filtered = useMemo(
    () =>
      query
        ? options.filter(
            (o) =>
              o.code.toLowerCase().includes(query.toLowerCase()) ||
              o.label.toLowerCase().includes(query.toLowerCase()),
          )
        : options,
    [query, options],
  );

  const selected = options.find((o) => o.code === value) || options[0];

  return (
    <div className="lang-select" ref={ref}>
      <button
        type="button"
        className="lang-select-trigger"
        onClick={() => setOpen(!open)}
      >
        <FlagIcon code={selected.code} />
        <span>{selected.label}</span>
        <span className="lang-select-arrow">{open ? '▲' : '▼'}</span>
      </button>
      {open && (
        <div className="lang-select-dropdown">
          <input
            ref={inputRef}
            className="lang-select-filter"
            type="text"
            placeholder="Filter…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Escape') {
                setOpen(false);
                setQuery('');
              }
              if (e.key === 'Enter' && filtered.length > 0) {
                onChange(filtered[0].code);
                setOpen(false);
                setQuery('');
              }
            }}
          />
          <div className="lang-select-list">
            {filtered.map((opt) => (
              <button
                key={opt.code}
                type="button"
                className={`lang-select-option${opt.code === value ? ' active' : ''}`}
                onClick={() => {
                  onChange(opt.code);
                  setOpen(false);
                  setQuery('');
                }}
              >
                <FlagIcon code={opt.code} />
                <span>{opt.label}</span>
                <span className="lang-select-code">{opt.code}</span>
              </button>
            ))}
            {filtered.length === 0 && (
              <div className="lang-select-empty">No matches</div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
