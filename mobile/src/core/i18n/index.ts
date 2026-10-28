import { getLocales } from 'expo-localization';
import { createInstance } from 'i18next';
import { initReactI18next } from 'react-i18next';

import { en } from './en';
import { tr } from './tr';

export const LANGUAGES = ['tr', 'en'] as const;
export type Language = (typeof LANGUAGES)[number];

export function isLanguage(value: unknown): value is Language {
  return LANGUAGES.includes(value as Language);
}

export function deviceLanguage(): Language {
  return getLocales()[0]?.languageCode === 'en' ? 'en' : 'tr';
}

/** Intl biçimlendirmesi için yerel ayar etiketi. */
export function localeFor(language: string): string {
  return language === 'en' ? 'en-GB' : 'tr-TR';
}

const i18n = createInstance();

void i18n.use(initReactI18next).init({
  resources: { tr: { translation: tr }, en: { translation: en } },
  lng: deviceLanguage(),
  fallbackLng: 'tr',
  interpolation: { escapeValue: false },
  initAsync: false,
});

export default i18n;
