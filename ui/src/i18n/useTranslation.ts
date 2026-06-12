import { translations, type Strings } from './translations';

/**
 * Returns translated strings for the given language.
 * `lang` should come from config (persisted to backend).
 * Falls back to 'en' for unknown languages.
 */
export function useTranslation(lang: string | null | undefined): Strings {
  const t = (lang === 'ru' ? translations.ru : translations.en);
  return t;
}

/**
 * Format a string with positional replacements: format("Hello {0}", "world")
 */
export function fmt(template: string, ...args: (string | number)[]): string {
  return template.replace(/\{(\d+)\}/g, (_, i) => String(args[Number(i)] ?? ''));
}
