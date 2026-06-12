import { useState, useEffect, useCallback } from 'react';
import { invoke } from '../services/tauri';
import type { Device } from '../types';

const MONITOR_MS = 3000;

export function useDevices(
  ready: boolean,
  configDeviceId: string | null,
  updateConfig: (key: 'device_id' | 'device_name', value: unknown) => void,
) {
  const [devices, setDevices] = useState<Device[]>([]);
  const [selectedDeviceId, setSelectedDeviceId] = useState('');

  // Initial load
  useEffect(() => {
    if (!ready) return;
    invoke<Device[]>('list_audio_devices')
      .then((devs) => {
        setDevices(devs);
        if (configDeviceId && devs.some((d) => d.id === configDeviceId)) {
          setSelectedDeviceId(configDeviceId);
        }
      })
      .catch(() => {});
  }, [ready]); // eslint-disable-line react-hooks/exhaustive-deps

  // Monitoring
  useEffect(() => {
    if (!ready) return;
    const check = () => {
      invoke<Device[]>('list_audio_devices')
        .then((devs) => {
          setDevices(devs);
          setSelectedDeviceId((sel) => {
            if (sel && !devs.some((d) => d.id === sel)) {
              updateConfig('device_id', null);
              updateConfig('device_name', null);
              return '';
            }
            return sel;
          });
        })
        .catch(() => {});
    };
    const id = setInterval(check, MONITOR_MS);
    return () => clearInterval(id);
  }, [ready, updateConfig]);

  const onDeviceChange = useCallback(
    (deviceId: string) => {
      setSelectedDeviceId(deviceId);
      if (deviceId) {
        const selected = devices.find((d) => d.id === deviceId);
        if (selected) {
          updateConfig('device_id', selected.id);
          updateConfig(
            'device_name',
            selected.name.replace(' (default)', '').split(' - ')[0],
          );
        }
      } else {
        updateConfig('device_id', null);
        updateConfig('device_name', null);
      }
    },
    [devices, updateConfig],
  );

  return { devices, selectedDeviceId, onDeviceChange };
}
