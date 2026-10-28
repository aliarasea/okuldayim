import * as SQLite from 'expo-sqlite';

import { deleteDatabaseKey, getOrCreateDatabaseKey } from '../auth/secureStorage';
import { logger } from '../privacy/logger';

const DATABASE_NAME = 'okuldayim.db';
const SCHEMA_VERSION = 1;

let opening: Promise<SQLite.SQLiteDatabase> | null = null;

/**
 * Yerel veritabanı (çevrimdışı kuyruk, ayarlar, oturum önbelleği). SQLCipher ile şifrelenir; anahtar
 * Keychain/Keystore'da durur. Üretim derlemesinde SQLCipher yoksa veritabanı açılmaz.
 */
export function getDatabase(): Promise<SQLite.SQLiteDatabase> {
  opening ??= openEncrypted().catch((error: unknown) => {
    opening = null;
    throw error;
  });
  return opening;
}

async function openEncrypted(): Promise<SQLite.SQLiteDatabase> {
  const key = await getOrCreateDatabaseKey();
  const db = await SQLite.openDatabaseAsync(DATABASE_NAME);
  // SQLCipher'da anahtar ilk ifade olmalıdır; 32 baytlık ham anahtar hex olarak verilir.
  await db.execAsync(`PRAGMA key = "x'${key}'"`);

  const cipher = await db.getFirstAsync<{ cipher_version: string }>('PRAGMA cipher_version');
  if (!cipher?.cipher_version) {
    if (!__DEV__) {
      await db.closeAsync();
      throw new Error('SQLCipher etkin değil; şifresiz yerel veritabanı açılmaz.');
    }
    logger.warn('SQLCipher bulunamadı (ör. Expo Go). Yerel veritabanı yalnızca geliştirmede şifresiz açılıyor.');
  }

  await db.execAsync('PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;');
  await migrate(db);
  return db;
}

async function migrate(db: SQLite.SQLiteDatabase): Promise<void> {
  const row = await db.getFirstAsync<{ user_version: number }>('PRAGMA user_version');
  const current = row?.user_version ?? 0;
  if (current >= SCHEMA_VERSION) return;

  if (current < 1) {
    await db.execAsync(`
      CREATE TABLE IF NOT EXISTS outbox (
        id TEXT PRIMARY KEY NOT NULL,
        method TEXT NOT NULL,
        path TEXT NOT NULL,
        body TEXT,
        created_at INTEGER NOT NULL,
        attempts INTEGER NOT NULL DEFAULT 0,
        next_attempt_at INTEGER NOT NULL,
        last_error TEXT,
        state TEXT NOT NULL DEFAULT 'PENDING'
      );
      CREATE INDEX IF NOT EXISTS outbox_state_created ON outbox (state, created_at);
      CREATE TABLE IF NOT EXISTS settings (
        key TEXT PRIMARY KEY NOT NULL,
        value TEXT NOT NULL
      );
    `);
  }
  await db.execAsync(`PRAGMA user_version = ${SCHEMA_VERSION}`);
}

export async function getSetting(key: string): Promise<string | null> {
  const db = await getDatabase();
  const row = await db.getFirstAsync<{ value: string }>('SELECT value FROM settings WHERE key = ?', key);
  return row?.value ?? null;
}

export async function setSetting(key: string, value: string): Promise<void> {
  const db = await getDatabase();
  await db.runAsync(
    'INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value',
    key,
    value,
  );
}

/** AUTH-07: oturum kapanınca veya sunucu oturumu iptal edince cihazdaki tüm yerel veri silinir. */
export async function wipeDatabase(): Promise<void> {
  const pending = opening;
  opening = null;
  if (pending) {
    try {
      const db = await pending;
      await db.closeAsync();
    } catch {
      // Veritabanı hiç açılamamış olabilir; yine de dosya ve anahtar silinir.
    }
  }
  try {
    await SQLite.deleteDatabaseAsync(DATABASE_NAME);
  } catch (error) {
    logger.warn('Yerel veritabanı silinemedi', error);
  }
  await deleteDatabaseKey();
}
