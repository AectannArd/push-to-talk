import { useState, useRef, useEffect } from 'react';
import FlagIcon from './FlagIcon';

interface Props {
  value: string;
  onChange: (code: string) => void;
  options: { code: string; label: string }[];
}

export default function LangSelect({ value, onChange, options }: Props) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, []);

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
          {options.map((opt) => (
            <button
              key={opt.code}
              type="button"
              className={`lang-select-option${opt.code === value ? ' active' : ''}`}
              onClick={() => {
                onChange(opt.code);
                setOpen(false);
              }}
            >
              <FlagIcon code={opt.code} />
              <span>{opt.label}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
