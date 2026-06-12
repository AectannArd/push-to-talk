import { translations, type Lang, type Strings } from './translations';

/**
 * Returns translated strings for the given language.
 * `lang` should come from config (persisted to backend).
 * Falls back to 'en' for unknown languages.
 */
export function useTranslation(lang: string | null | undefined): Strings {
  const key: Lang = (lang === 'ru' || lang === 'ru-RU') ? 'ru-RU' : 'en-US';
  return translations[key];
}

/**
 * Format a string with positional replacements: format("Hello {0}", "world")
 */
export function fmt(template: string, ...args: (string | number)[]): string {
  return template.replace(/\{(\d+)\}/g, (_, i) => String(args[Number(i)] ?? ''));
}
