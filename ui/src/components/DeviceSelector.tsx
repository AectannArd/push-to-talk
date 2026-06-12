import type { Device } from '../types';

interface Props {
  devices: Device[];
  selectedDeviceId: string;
  onChange: (id: string) => void;
}

function formatLabel(d: Device): string {
  let label = d.name;
  if (d.is_default) label += ' (default)';
  if (d.config) label += ' - ' + d.config;
  return label;
}

export default function DeviceSelector({ devices, selectedDeviceId, onChange }: Props) {
  return (
    <div className="form-group">
      <label htmlFor="deviceSelect">Audio Input Device</label>
      <select
        id="deviceSelect"
        name="deviceSelect"
        value={selectedDeviceId}
        onChange={(e) => onChange(e.target.value)}
      >
        <option value="">Use default device</option>
        {devices.map((d) => (
          <option key={d.id} value={d.id}>
            {formatLabel(d)}
          </option>
        ))}
      </select>
    </div>
  );
}
