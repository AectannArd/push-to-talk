import type { Device } from '../types';
import type { Strings } from '../i18n/translations';

interface Props {
  devices: Device[];
  selectedDeviceId: string;
  onChange: (id: string) => void;
  s: Strings;
}

function formatLabel(d: Device): string {
  let label = d.name;
  if (d.is_default) label += ' (default)';
  if (d.config) label += ' - ' + d.config;
  return label;
}

export default function DeviceSelector({ devices, selectedDeviceId, onChange, s }: Props) {
  return (
    <div className="form-group">
      <label htmlFor="deviceSelect">{s.audioDevice}</label>
      <select
        id="deviceSelect"
        name="deviceSelect"
        value={selectedDeviceId}
        onChange={(e) => onChange(e.target.value)}
      >
        <option value="">{s.useDefaultDevice}</option>
        {devices.map((d) => (
          <option key={d.id} value={d.id}>
            {formatLabel(d)}
          </option>
        ))}
      </select>
    </div>
  );
}
