import * as Crypto from 'expo-crypto';
import { create } from 'zustand';

import { apiClient } from '../api';
import { logger } from '../privacy/logger';
import { getDatabase } from './database';
import { type FlushResult, Outbox, type OutboxCounts } from './outbox';
import { createSqliteOutboxStore } from './sqliteOutboxStore';

export const outbox = new Outbox(
  createSqliteOutboxStore(getDatabase),
  async (item) => {
    await apiClient.request<unknown>({ method: item.method, path: item.path, body: item.body, idempotencyKey: item.id });
  },
  { newId: () => Crypto.randomUUID() },
);

export const useOutboxCounts = create<OutboxCounts>()(() => ({ pending: 0, failed: 0 }));

export async function refreshOutboxCounts(): Promise<void> {
  useOutboxCounts.setState(await outbox.counts());
}

export function resetOutboxCounts(): void {
  useOutboxCounts.setState({ pending: 0, failed: 0 });
}

export async function enqueue(...args: Parameters<Outbox['enqueue']>): Promise<string> {
  const id = await outbox.enqueue(...args);
  await refreshOutboxCounts();
  return id;
}

/** Kuyruğu göndermeyi dener; hata fırlatmaz. */
export async function syncOutbox(ignoreBackoff = false): Promise<FlushResult | null> {
  try {
    const result = await outbox.flush(ignoreBackoff);
    await refreshOutboxCounts();
    return result;
  } catch (error) {
    logger.warn('Çevrimdışı kuyruk gönderilemedi', error);
    return null;
  }
}
