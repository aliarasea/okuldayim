import { api, tokenManager } from '../api';
import { isApiError } from '../api/errors';
import type { Me, TokenPair } from '../api/types';
import i18n, { isLanguage, type Language } from '../i18n';
import { refreshOutboxCounts, resetOutboxCounts } from '../offline';
import { getSetting, setSetting, wipeDatabase } from '../offline/database';
import { logger } from '../privacy/logger';
import { queryClient } from '../query';
import { sessionActions, useSessionStore } from './sessionStore';

const ME_CACHE_KEY = 'session.me';
const LANGUAGE_KEY = 'settings.language';

export async function bootstrapSession(): Promise<void> {
  try {
    if (!(await tokenManager.hasStoredSession())) {
      sessionActions.signedOut();
      return;
    }
    await applyStoredLanguage();
    await loadMe();
    await refreshOutboxCounts();
  } catch (error) {
    if (isApiError(error) && error.retryable) {
      // Çevrimdışı açılış: öğretmen son bilinen sınıf bilgisiyle çalışmaya devam eder.
      const cached = await readCachedMe();
      if (cached) {
        sessionActions.setMe(cached, { offline: true });
        await refreshOutboxCounts().catch(() => undefined);
        return;
      }
    }
    if (!(isApiError(error) && error.code === 'unauthorized')) {
      logger.warn('Oturum başlatılamadı', error);
    }
    if (useSessionStore.getState().status === 'booting') {
      sessionActions.signedOut();
    }
  }
}

export async function loadMe(): Promise<Me> {
  const me = await api.me.get();
  sessionActions.setMe(me);
  await writeCachedMe(me);
  return me;
}

export async function completeSignIn(tokens: TokenPair): Promise<void> {
  await tokenManager.setTokens(tokens);
  await loadMe();
}

export async function signOut(): Promise<void> {
  const refreshToken = await tokenManager.getRefreshToken().catch(() => null);
  if (refreshToken) {
    try {
      await api.auth.logout(refreshToken);
    } catch (error) {
      // Çevrimdışıysa sunucudaki oturum, yenileme anahtarının süresi dolunca kapanır.
      logger.info('Sunucuda oturum kapatılamadı', error);
    }
  }
  await wipeLocalData();
  sessionActions.signedOut();
}

/** AUTH-07: anahtarlar, şifreli yerel veritabanı, kuyruk ve bellekteki önbellek silinir. */
export async function wipeLocalData(): Promise<void> {
  queryClient.clear();
  resetOutboxCounts();
  await Promise.allSettled([tokenManager.clear(), wipeDatabase()]);
}

// Sunucu yenileme anahtarını reddetti (süre dolumu veya uzaktan oturum kapatma).
tokenManager.onSessionEnded(() => {
  void wipeLocalData().finally(() => sessionActions.signedOut('expired'));
});

export async function changeLanguage(language: Language): Promise<void> {
  await i18n.changeLanguage(language);
  try {
    await setSetting(LANGUAGE_KEY, language);
  } catch (error) {
    logger.warn('Dil tercihi kaydedilemedi', error);
  }
}

async function applyStoredLanguage(): Promise<void> {
  const stored = await getSetting(LANGUAGE_KEY).catch(() => null);
  if (isLanguage(stored)) await i18n.changeLanguage(stored);
}

async function readCachedMe(): Promise<Me | null> {
  try {
    const raw = await getSetting(ME_CACHE_KEY);
    return raw ? (JSON.parse(raw) as Me) : null;
  } catch {
    return null;
  }
}

async function writeCachedMe(me: Me): Promise<void> {
  try {
    await setSetting(ME_CACHE_KEY, JSON.stringify(me));
  } catch (error) {
    logger.warn('Oturum önbelleği yazılamadı', error);
  }
}
