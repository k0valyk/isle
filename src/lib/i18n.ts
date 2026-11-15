import uk from "../i18n/uk.json";
import en from "../i18n/en.json";
import ru from "../i18n/ru.json";

export type Locale = "uk" | "en" | "ru";

export const locales: Locale[] = ["uk", "en", "ru"];

const dictionaries: Record<Locale, Record<string, string>> = {
  uk: uk as Record<string, string>,
  en: en as Record<string, string>,
  ru: ru as Record<string, string>,
};

export function resolveLocale(value: string | undefined | null): Locale {
  return value === "en" || value === "ru" ? value : "uk";
}

export function translate(
  locale: Locale,
  key: string,
  params: Record<string, string | number> = {},
): string {
  const dict = dictionaries[locale] ?? dictionaries.uk;
  const raw = dict[key] ?? dictionaries.uk[key] ?? key;
  return raw.replace(/\{\{(\w+)\}\}/g, (_match, name: string) => {
    const value = params[name];
    return value === undefined ? `{{${name}}}` : String(value);
  });
}

export function localeName(locale: Locale): string {
  return translate(locale, `settings.lang_${locale}`);
}
