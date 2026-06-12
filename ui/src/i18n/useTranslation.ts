import { translations, type Strings } from './translations';

const ALIASES: Record<string, string> = {
  en: 'en-US', ru: 'ru-RU', de: 'de-DE', fr: 'fr-FR',
  es: 'es-ES', it: 'it-IT', pt: 'pt-PT', pl: 'pl-PL',
  uk: 'uk-UA', nl: 'nl-NL', cs: 'cs-CZ', sv: 'sv-SE',
  fi: 'fi-FI', ro: 'ro-RO', hu: 'hu-HU', el: 'el-GR',
  bg: 'bg-BG', da: 'da-DK', sk: 'sk-SK', lt: 'lt-LT',
  lv: 'lv-LV', et: 'et-EE', sl: 'sl-SI', hr: 'hr-HR',
  no: 'no-NO', tr: 'tr-TR', be: 'be-BY',
  zh: 'zh-CN', ja: 'ja-JP', ko: 'ko-KR', hi: 'hi-IN',
  ar: 'ar-SA', th: 'th-TH', vi: 'vi-VN', id: 'id-ID',
  ms: 'ms-MY', fa: 'fa-IR', he: 'he-IL', bn: 'bn-BD',
  ur: 'ur-PK', ta: 'ta-IN', te: 'te-IN',
  sw: 'sw-KE', am: 'am-ET', zu: 'zu-ZA', af: 'af-ZA',
  ha: 'ha-NG', yo: 'yo-NG',
  'pt-BR': 'pt-BR',
};

function resolve(lang: string | null | undefined): keyof typeof translations {
  if (!lang) return 'en-US';
  if (lang in translations) return lang as keyof typeof translations;
  const mapped = ALIASES[lang];
  if (mapped && mapped in translations) return mapped as keyof typeof translations;
  return 'en-US';
}

export function useTranslation(lang: string | null | undefined): Strings {
  return translations[resolve(lang)];
}

export function fmt(template: string, ...args: (string | number)[]): string {
  return template.replace(/\{(\d+)\}/g, (_, i) => String(args[Number(i)] ?? ''));
}
