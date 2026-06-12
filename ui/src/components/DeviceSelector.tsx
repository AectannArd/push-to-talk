import type { Device } from '../types';
import type { Strings } from '../i18n/translations';

export default function DeviceSelector({ devices, selectedDeviceId, onChange, s }: {
  devices: Device[]; selectedDeviceId: string; onChange: (id: string) => void; s: Strings;
}) {
  return (
    <div className="mb-3">
      <label className="form-label">{s.audioDevice}</label>
      <select className="form-select" value={selectedDeviceId} onChange={(e) => onChange(e.target.value)}>
        <option value="">{s.useDefaultDevice}</option>
        {devices.map((d) => (
          <option key={d.id} value={d.id}>
            {d.name + (d.is_default ? ' (default)' : '') + (d.config ? ' - ' + d.config : '')}
          </option>
        ))}
      </select>
    </div>
  );
}
