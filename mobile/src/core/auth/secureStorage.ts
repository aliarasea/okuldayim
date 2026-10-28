import * as Crypto from 'expo-crypto';
import * as SecureStore from 'expo-secure-store';

import type { RefreshTokenStorage } from './tokens';

// Yalnızca cihaz kilidi açıkken okunur; yedekten geri yüklemede veya yeni cihaza taşınmaz.
const OPTIONS: SecureStore.SecureStoreOptions = {
  keychainAccessible: SecureStore.WHEN_UNLOCKED_THIS_DEVICE_ONLY,
};

const KEYS = {
  refreshToken: 'okuldayim.refresh_token',
  deviceId: 'okuldayim.device_id',
  databaseKey: 'okuldayim.db_key',
} as const;

export const secureRefreshTokenStorage: RefreshTokenStorage = {
  get: () => SecureStore.getItemAsync(KEYS.refreshToken, OPTIONS),
  set: (token) => SecureStore.setItemAsync(KEYS.refreshToken, token, OPTIONS),
  clear: () => SecureStore.deleteItemAsync(KEYS.refreshToken, OPTIONS),
};

/** Cihaz listesinde ve uzaktan oturum kapatmada (AUTH-07) kullanılan rastgele cihaz kimliği. */
export function getDeviceId(): Promise<string> {
  return getOrCreate(KEYS.deviceId, () => Crypto.randomUUID());
}

/** Yerel SQLCipher veritabanının 256 bitlik anahtarı (hex). */
export function getOrCreateDatabaseKey(): Promise<string> {
  return getOrCreate(KEYS.databaseKey, () => toHex(Crypto.getRandomBytes(32)));
}

export function deleteDatabaseKey(): Promise<void> {
  return SecureStore.deleteItemAsync(KEYS.databaseKey, OPTIONS);
}

async function getOrCreate(key: string, create: () => string): Promise<string> {
  const existing = await SecureStore.getItemAsync(key, OPTIONS);
  if (existing) return existing;
  const value = create();
  await SecureStore.setItemAsync(key, value, OPTIONS);
  return value;
}

function toHex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
}
