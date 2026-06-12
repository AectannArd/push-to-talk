export type Lang = 'en' | 'ru';

export interface Strings {
  // App shell
  appTitle: string;
  appSubtitle: string;
  initializing: string;
  // Status bar
  recording: string;
  ready: string;
  serviceStopped: string;
  serviceLabel: string;
  recordingLabel: string;
  running: string;
  stopped: string;
  yes: string;
  no: string;
  // Recording controls
  startRecording: string;
  stop: string;
  startService: string;
  // Device selector
  audioDevice: string;
  useDefaultDevice: string;
  // Model selector
  availableModels: string;
  scanningModels: string;
  noModelsFound: string;
  downloadModel: string;
  downloadModelLabel: string;
  selectModelToDownload: string;
  allModelsDownloaded: string;
  downloading: string;
  download: string;
  downloadHint: string;
  downloadSuccess: string;
  downloadFailed: string;
  // Config form
  configuration: string;
  common: string;
  audioTranscription: string;
  hotkey: string;
  hotkeyHint: string;
  language: string;
  languageHint: string;
  whisperModel: string;
  modelSearchDirs: string;
  modelSearchDirsHint: string;
  // Punctuation panel
  punctuationRestoration: string;
  enablePunctuation: string;
  punctuationHint: string;
  punctuationActive: string;
  punctuationModelFound: string;
  punctuationModelMissing: string;
  downloadPunctuationModel: string;
  downloadCompleteRestart: string;
  downloadFailedLabel: string;
  noModelDir: string;
  // Logging panel
  logging: string;
  logDirectory: string;
  logLevel: string;
  logFormat: string;
  logRetention: string;
  text: string;
  json: string;
  changesAutoSaved: string;
  // Modal
  punctuationModelNotFound: string;
  modalDescription: string;
  modalSize: string;
  yesDownload: string;
  noDisable: string;
  retryDownload: string;
  // Status messages
  serviceStarted: string;
  failed: string;
  // General
  trace: string;
  debug: string;
  info: string;
  warn: string;
  error: string;
}

const en: Strings = {
  appTitle: 'Push-to-Talk',
  appSubtitle: 'Voice transcription at your fingertips',
  initializing: 'Initializing…',
  recording: 'Recording...',
  ready: 'Ready (press button or hotkey to record)',
  serviceStopped: 'Service stopped',
  serviceLabel: 'Service:',
  recordingLabel: 'Recording:',
  running: 'Running',
  stopped: 'Stopped',
  yes: 'Yes',
  no: 'No',
  startRecording: '\u{1F3A4} Start Recording',
  stop: '\u{23F9} Stop',
  startService: '\u{25B6} Start Service',
  audioDevice: 'Audio Input Device',
  useDefaultDevice: 'Use default device',
  availableModels: 'Available Models',
  scanningModels: 'Scanning model directories...',
  noModelsFound: 'No models found. Download a model or update search directories.',
  downloadModel: 'Download Model',
  downloadModelLabel: 'Download Model',
  selectModelToDownload: 'Select a model to download...',
  allModelsDownloaded: 'All models downloaded \u{2713}',
  downloading: '\u{23F3} Downloading...',
  download: '\u{2B07} Download',
  downloadHint: 'Downloads from Hugging Face to the first directory in Model Search Directories',
  downloadSuccess: 'Model {id} downloaded successfully!',
  downloadFailed: 'Download failed: ',
  configuration: 'Configuration',
  common: 'Common',
  audioTranscription: 'Audio & Transcription',
  hotkey: 'Hotkey',
  hotkeyHint: 'Format: Mod+Mod+Key (e.g., Ctrl+Shift+T, Alt+T)',
  language: 'Language',
  languageHint: 'Use "auto" for automatic detection, or specify: ru, en, de, etc.',
  whisperModel: 'Whisper Model',
  modelSearchDirs: 'Model Search Directories',
  modelSearchDirsHint: 'Comma-separated list of directories',
  punctuationRestoration: 'Punctuation Restoration',
  enablePunctuation: 'Enable punctuation & case restoration',
  punctuationHint: 'Uses BERT model to restore punctuation and correct casing in transcribed text. Model: kontur-ai/sbert_punc_case_ru (ONNX, ~1.7 GB).',
  punctuationActive: '\u{2713} Active — service running with punctuation',
  punctuationModelFound: '\u{2713} Model found — restart service to apply',
  punctuationModelMissing: '\u{26A0} Model not found in search directories',
  downloadPunctuationModel: '\u{2B07} Download Punctuation Model',
  downloadCompleteRestart: 'Download complete! Restart the service to apply.',
  downloadFailedLabel: 'Download failed.',
  noModelDir: 'No model search directory configured. Set one in the Whisper Model section.',
  logging: 'Logging',
  logDirectory: 'Log Directory',
  logLevel: 'Log Level',
  logFormat: 'Log Format',
  logRetention: 'Log Retention (hours)',
  text: 'Text',
  json: 'JSON',
  changesAutoSaved: 'Changes are saved automatically',
  punctuationModelNotFound: 'Punctuation Model Not Found',
  modalDescription: 'The punctuation model (model.onnx) is not present in your model directories. Would you like to download it from HuggingFace?',
  modalSize: '~1.7 GB download. Model is only needed once.',
  yesDownload: 'Yes, Download',
  noDisable: 'No, Disable',
  retryDownload: 'Retry Download',
  serviceStarted: 'Service started',
  failed: 'Failed: ',
  trace: 'trace',
  debug: 'debug',
  info: 'info',
  warn: 'warn',
  error: 'error',
};

const ru: Strings = {
  appTitle: 'Push-to-Talk',
  appSubtitle: 'Голосовой ввод на кончиках пальцев',
  initializing: 'Инициализация…',
  recording: 'Запись...',
  ready: 'Готово (нажмите кнопку или горячую клавишу)',
  serviceStopped: 'Сервис остановлен',
  serviceLabel: 'Сервис:',
  recordingLabel: 'Запись:',
  running: 'Запущен',
  stopped: 'Остановлен',
  yes: 'Да',
  no: 'Нет',
  startRecording: '\u{1F3A4} Начать запись',
  stop: '\u{23F9} Стоп',
  startService: '\u{25B6} Запустить сервис',
  audioDevice: 'Аудиоустройство',
  useDefaultDevice: 'Использовать по умолчанию',
  availableModels: 'Доступные модели',
  scanningModels: 'Сканирование папок с моделями...',
  noModelsFound: 'Модели не найдены. Скачайте модель или обновите папки поиска.',
  downloadModel: 'Скачать модель',
  downloadModelLabel: 'Скачать модель',
  selectModelToDownload: 'Выберите модель для скачивания...',
  allModelsDownloaded: 'Все модели скачаны \u{2713}',
  downloading: '\u{23F3} Скачивание...',
  download: '\u{2B07} Скачать',
  downloadHint: 'Скачивание с Hugging Face в первую папку из списка поиска',
  downloadSuccess: 'Модель {id} успешно скачана!',
  downloadFailed: 'Ошибка скачивания: ',
  configuration: 'Конфигурация',
  common: 'Основное',
  audioTranscription: 'Аудио и транскрипция',
  hotkey: 'Горячая клавиша',
  hotkeyHint: 'Формат: Mod+Mod+Клавиша (например, Ctrl+Shift+T, Alt+T)',
  language: 'Язык',
  languageHint: 'Используйте "auto" для автоопределения или укажите: ru, en, de и т.д.',
  whisperModel: 'Модель Whisper',
  modelSearchDirs: 'Папки поиска моделей',
  modelSearchDirsHint: 'Список папок через запятую',
  punctuationRestoration: 'Восстановление пунктуации',
  enablePunctuation: 'Включить пунктуацию и заглавные буквы',
  punctuationHint: 'Использует BERT-модель для восстановления пунктуации и регистра. Модель: kontur-ai/sbert_punc_case_ru (ONNX, ~1.7 ГБ).',
  punctuationActive: '\u{2713} Активно — сервис работает с пунктуацией',
  punctuationModelFound: '\u{2713} Модель найдена — перезапустите сервис',
  punctuationModelMissing: '\u{26A0} Модель не найдена в папках поиска',
  downloadPunctuationModel: '\u{2B07} Скачать модель пунктуации',
  downloadCompleteRestart: 'Скачивание завершено! Перезапустите сервис.',
  downloadFailedLabel: 'Ошибка скачивания.',
  noModelDir: 'Папка поиска моделей не настроена. Укажите её в разделе «Модель Whisper».',
  logging: 'Логирование',
  logDirectory: 'Папка логов',
  logLevel: 'Уровень логирования',
  logFormat: 'Формат логов',
  logRetention: 'Хранение логов (часов)',
  text: 'Текст',
  json: 'JSON',
  changesAutoSaved: 'Изменения сохраняются автоматически',
  punctuationModelNotFound: 'Модель пунктуации не найдена',
  modalDescription: 'Модель пунктуации (model.onnx) отсутствует в папках поиска. Скачать её с HuggingFace?',
  modalSize: '~1.7 ГБ для скачивания. Модель нужна только один раз.',
  yesDownload: 'Да, скачать',
  noDisable: 'Нет, отключить',
  retryDownload: 'Повторить скачивание',
  serviceStarted: 'Сервис запущен',
  failed: 'Ошибка: ',
  trace: 'trace',
  debug: 'debug',
  info: 'info',
  warn: 'warn',
  error: 'error',
};

export const translations: Record<Lang, Strings> = { en, ru };
