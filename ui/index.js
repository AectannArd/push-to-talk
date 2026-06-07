let invoke;

// Forward console logs to backend
function forwardConsole(fnName, level) {
    const original = console[fnName];
    console[fnName] = (message) => {
        original(message);
        // Send to backend asynchronously
        invoke('frontend_log', { level, message: String(message) }).catch(() => {});
    }
}

// Wait for Tauri API to be ready before initializing
function waitForTauri() {
    return new Promise((resolve, reject) => {
        if (window.__TAURI__?.core?.invoke) {
            resolve(window.__TAURI__.core.invoke);
        } else {
            let attempts = 0;
            const check = setInterval(() => {
                attempts++;
                if (window.__TAURI__?.core?.invoke) {
                    clearInterval(check);
                    resolve(window.__TAURI__.core.invoke);
                } else if (attempts > 50) {
                    clearInterval(check);
                    reject(new Error('Tauri API not available after 5 seconds'));
                }
            }, 100);
        }
    });
}

let devicePollInterval = null;
let modelsScanned = false;
let devicesLoaded = false;
let isInitialLoad = true;
let isRecording = false;
let isServiceRunning = false;
let uiIsRecording = false;        // local UI button state — decoupled from global hotkey state
let uiTranscriptionPending = false; // true after UI stop — expect a new transcription
let lastDisplayedTranscription = null;
let selectedModel = null;
let lastModelScan = '';

// Model catalog loaded from backend at startup
let downloadableModels = [];

waitForTauri()
    .then(inv => {
        invoke = inv;

        // Set up console forwarding after invoke is available
        forwardConsole('log', 'trace');
        forwardConsole('debug', 'debug');
        forwardConsole('info', 'info');
        forwardConsole('warn', 'warn');
        forwardConsole('error', 'error');

        // Load catalog from backend (model IDs → names, descs, URLs)
        loadCatalog();
        // Load config once on startup
        loadConfig();
        // Poll only for status updates (recording state)
        setInterval(pollStatus, 2000);
    })
    .catch(err => {
        console.error('Failed to initialize Tauri API:', err);
        document.body.innerHTML = '<h1>Error: Tauri API not available</h1><p>Please restart the application.</p>';
    });

// Load and populate device dropdown
async function loadDevices() {
    if (devicesLoaded) return; // Only load once
    try {
        const devices = await invoke('list_audio_devices');
        const select = document.getElementById('deviceSelect');
        const currentValue = select.value;

        // Clear existing options except the default
        select.innerHTML = '<option value="">Use default device</option>';

        devices.forEach(device => {
            const option = document.createElement('option');
            option.value = device.id;
            option.textContent = device.name + (device.is_default ? ' (default)' : '');
            if (device.config) {
                option.textContent += ' - ' + device.config;
            }
            select.appendChild(option);
        });

        // Restore selection if still available
        if (currentValue && devices.some(d => d.id === currentValue)) {
            select.value = currentValue;
        }
        devicesLoaded = true;
    } catch (error) {
        console.error('Failed to load devices:', error);
    }
}

// Start polling for device changes
function startDeviceMonitoring() {
    // Check every 3 seconds for device changes
    devicePollInterval = setInterval(async () => {
        const currentDeviceEl = document.getElementById('deviceSelect');
        const currentDeviceId = currentDeviceEl.value;

        if (currentDeviceId) {
            // Check if current device is still available
            try {
                const devices = await invoke('list_audio_devices');
                const deviceStillExists = devices.some(d => d.id === currentDeviceId);

                if (!deviceStillExists) {
                    // Current device was disconnected
                    showStatus('Current device disconnected, switching to default', 'error');
                    currentDeviceEl.value = '';
                    // Auto-save the config with default device
                    await saveConfigSilently();
                }
            } catch (error) {
                console.error('Failed to check devices:', error);
            }
        }
    }, 3000);
}

// Save config without showing success message (for auto-switch)
async function saveConfigSilently() {
    const config = buildConfigFromForm();
    try {
        await invoke('save_config', { cfg: config });
    } catch (error) {
        console.error('Failed to save config:', error);
    }
}

// Build config object from form
function buildConfigFromForm() {
    const deviceSelect = document.getElementById('deviceSelect');
    return {
        hotkey: document.getElementById('hotkey').value.trim(),
        language: document.getElementById('language').value.trim() || null,
        device_id: deviceSelect.value || null,
        device_name: deviceSelect.selectedOptions[0]?.textContent?.replace(' (default)', '').split(' - ')[0] || null,
        model: selectedModel || null,
        model_search_dirs: document.getElementById('modelSearchDirs').value.split(',').map(s => s.trim()).filter(s => s.length > 0),
        log_dir: document.getElementById('logDir').value.trim() || 'logs',
        log_level: document.getElementById('logLevel').value,
        log_format: document.getElementById('logFormat').value,
        log_retention_hours: parseInt(document.getElementById('logRetention').value) ?? 24
    };
}

// Auto-save config on field changes (debounced, silent)
let autoSaveTimeout = null;
function autoSaveConfig() {
    if (isInitialLoad) return; // Skip auto-save during initial load
    if (autoSaveTimeout) clearTimeout(autoSaveTimeout);
    autoSaveTimeout = setTimeout(async () => {
        try {
            const config = buildConfigFromForm();
            await invoke('save_config', { cfg: config });
        } catch (error) {
            // Serialize error properly for Tauri errors
            const errorMsg = typeof error === 'string' ? error :
                            error?.message ||
                            error?.toString() ||
                            JSON.stringify(error) ||
                            'Unknown error';
            console.error('Auto-save failed: ' + errorMsg);
        }
    }, 500);
}

// Auto-save on form field changes
const autoSaveFields = ['hotkey', 'language', 'deviceSelect', 'modelSearchDirs', 'logDir', 'logLevel', 'logFormat', 'logRetention'];
autoSaveFields.forEach(id => {
    const el = document.getElementById(id);
    if (el) {
        el.addEventListener(id === 'deviceSelect' ? 'change' : 'input', autoSaveConfig);
    }
});

async function loadCatalog() {
    try {
        downloadableModels = await invoke('get_downloadable_models');
    } catch (error) {   
        console.error('Failed to load model catalog:', error);
    }
}

async function loadConfig() {
    try {
        const status = await invoke('get_status');
        updateStatusUI(status);
        const config = await invoke('get_config');
        fillConfigForm(config);
        await loadDevices();
        if (!devicePollInterval) {
            startDeviceMonitoring();
            // Auto-scan models on first load
            if (!modelsScanned) {
                modelsScanned = true;
                setTimeout(() => scanModels(), 500);
            }
        }
        isInitialLoad = false; // Initial load complete, enable auto-save
    } catch (error) {
        showStatus('Failed to load state: ' + error, 'error');
    }
}

async function pollStatus() {
    try {
        const status = await invoke('get_status');
        updateStatusUI(status);
    } catch (error) {
        // Silently ignore polling errors
    }
}

function updateStatusUI(status) {
    isRecording = status.is_recording;
    isServiceRunning = status.is_service_running;

    // Indicator light and status text — always reflect real server state
    const indicator = document.getElementById('statusIndicator');
    const statusText = document.getElementById('statusText');
    indicator.className = 'status-indicator';
    if (status.is_recording) {
        indicator.classList.add('recording');
        statusText.textContent = '🔴 Recording...';
    } else if (status.is_service_running) {
        indicator.classList.add('running');
        statusText.textContent = 'Ready (press button or hotkey to record)';
    } else {
        statusText.textContent = 'Service stopped';
    }

    // Button appearance — tracked locally, decoupled from global hotkey
    updateButtonAppearance();

    // Session info cards
    document.getElementById('serviceStatus').textContent = status.is_service_running ? 'Running' : 'Stopped';
    document.getElementById('recordingStatus').textContent = status.is_recording ? 'Yes' : 'No';

    // Update transcription ONLY after UI-initiated stop
    if (uiTranscriptionPending && status.last_transcription &&
        status.last_transcription !== lastDisplayedTranscription) {
        document.getElementById('lastTranscription').textContent = status.last_transcription;
        lastDisplayedTranscription = status.last_transcription;
        uiTranscriptionPending = false;
    }
}

// Select a model for transcription and auto-save
function selectModel(path) {
    selectedModel = path;
    // Reset snapshot so scanModels re-renders the radio-button selection
    lastModelScan = '';
    // Re-render to update selection markers
    scanModels();
    // Auto-save the new model selection
    autoSaveConfig();
}

function fillConfigForm(config) {
    // Set selected model from config (full path)
    selectedModel = config.model || null;

    document.getElementById('hotkey').value = config.hotkey || '';
    document.getElementById('language').value = config.language || '';
    if (config.device_id) {
        setTimeout(() => {
            const select = document.getElementById('deviceSelect');
            if (select.querySelector(`option[value="${config.device_id}"]`)) {
                select.value = config.device_id;
            }
        }, 100);
    }
    document.getElementById('modelSearchDirs').value = (config.model_search_dirs || []).join(', ');
    document.getElementById('logDir').value = config.log_dir || '';
    document.getElementById('logLevel').value = config.log_level || 'info';
    document.getElementById('logFormat').value = config.log_format || 'text';
    document.getElementById('logRetention').value = config.log_retention_hours ?? 24;
}

function updateButtonAppearance() {
    const toggleBtn = document.getElementById('toggleBtn');
    toggleBtn.classList.remove('btn-primary', 'btn-danger', 'btn-secondary');
    if (uiIsRecording) {
        toggleBtn.textContent = '⏹ Stop';
        toggleBtn.classList.add('btn-danger');
    } else if (isServiceRunning) {
        toggleBtn.textContent = '🎤 Start Recording';
        toggleBtn.classList.add('btn-primary');
    } else {
        toggleBtn.textContent = '▶ Start Service';
        toggleBtn.classList.add('btn-secondary');
    }
}

document.getElementById('toggleBtn').addEventListener('click', async () => {
    try {
        if (!isServiceRunning) {
            await invoke('start_service');
            showStatus('Service started', 'success');
            pollStatus();
        } else {
            await invoke('trigger_recording');
            uiIsRecording = !uiIsRecording;
            if (!uiIsRecording) {
                // UI just stopped recording — expect a new transcription soon
                uiTranscriptionPending = true;
            }
            updateButtonAppearance();
            pollStatus();
        }
    } catch (error) {
        showStatus('Failed: ' + error, 'error');
    }
});

// No save button needed - auto-save handles everything

function showStatus(message, type) {
    const el = document.getElementById('statusMessage');
    el.textContent = message;
    el.className = 'status-message status-' + type;
    setTimeout(() => { el.className = 'status-message'; }, 5000);
}

// Keyboard shortcut: Cmd+R or Ctrl+R to toggle recording
document.addEventListener('keydown', async (e) => {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'r') {
        e.preventDefault();
        if (isServiceRunning) {
            try {
                await invoke('trigger_recording');
                uiIsRecording = !uiIsRecording;
                if (!uiIsRecording) {
                    uiTranscriptionPending = true;
                }
                updateButtonAppearance();
                pollStatus();
            } catch (error) {
                showStatus('Failed: ' + error, 'error');
            }
        }
    }
});

// Scan model directories and refresh the UI. Returns the found models.
// Skips DOM update if the result is identical to the last scan.
async function scanModels() {
    const modelListEl = document.getElementById('modelList');
    try {
        const config = buildConfigFromForm();
        const models = await invoke('scan_models', { modelSearchDirs: config.model_search_dirs });

        // Skip DOM rebuild if nothing changed since last scan
        const snapshot = JSON.stringify(models.map(m => m.filename).sort());
        if (snapshot === lastModelScan) {
            return models;
        }
        lastModelScan = snapshot;

        if (models.length === 0) {
            modelListEl.innerHTML = '<p style="color: #888;">No models found. Download a model or update search directories.</p>';
        } else {
            modelListEl.innerHTML = models.map(m => {
                const isSelected = selectedModel === m.path;
                const radio = isSelected ? '●' : '○';
                return `<div class="model-item${isSelected ? ' model-item-selected' : ''}"
                            data-path="${m.path}"
                            title="${m.path}">
                            <span class="model-radio">${radio}</span>
                            <span class="model-name">${m.filename}</span>
                            <span class="model-size">${m.size}</span>
                        </div>`;
            }).join('');

            // Add click handlers to model items
            modelListEl.querySelectorAll('.model-item').forEach(item => {
                item.addEventListener('click', function() {
                    selectModel(this.dataset.path);
                });
            });
        }

        // Rebuild download dropdown: only show models not yet on disk
        const foundFilenames = new Set(models.map(m => m.filename));
        const downloadSelect = document.getElementById('modelToDownload');
        const available = downloadableModels.filter(m => !foundFilenames.has(m.name));
        if (available.length === 0) {
            downloadSelect.innerHTML = '<option value="">All models downloaded ✓</option>';
        } else {
            downloadSelect.innerHTML = available.map(m =>
                `<option value="${m.id}">${m.desc}</option>`
            ).join('');
        }

        return models;
    } catch (error) {
        modelListEl.innerHTML = `<p style="color: #ff4444;">Error: ${error}</p>`;
        return [];
    }
}

// Background polling: check for model directory changes every 5 seconds
setInterval(() => {
    scanModels();
}, 5000);

// Download model button
document.getElementById('downloadModelBtn').addEventListener('click', async () => {
    const select = document.getElementById('modelToDownload');
    const modelId = select.value;
    const downloadBtn = document.getElementById('downloadModelBtn');

    // Guard: empty value = no model selected (placeholder "All models downloaded")
    if (!modelId) {
        showStatus('No model selected for download', 'error');
        return;
    }

    downloadBtn.disabled = true;
    downloadBtn.textContent = '⏳ Downloading...';

    try {
        const config = buildConfigFromForm();
        const targetDir = config.model_search_dirs[0] || '~/.push-to-talk/models';
        await invoke('download_model', { modelId, targetDir });
        showStatus(`Model ${modelId} downloaded successfully!`, 'success');
        // Immediate re-scan to show the new model and refresh the dropdown
        scanModels();
    } catch (error) {
        showStatus('Download failed: ' + error, 'error');
    } finally {
        downloadBtn.disabled = false;
        downloadBtn.textContent = '⬇️ Download';
    }
});
