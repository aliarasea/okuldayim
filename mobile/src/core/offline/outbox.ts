import { isApiError } from '../api/errors';
import type { HttpMethod } from '../api/transport';

export type OutboxState = 'PENDING' | 'FAILED';

export interface OutboxItem {
  /** Aynı zamanda Idempotency-Key: sunucu tekrar gönderilen isteği bir kez işler. */
  id: string;
  method: HttpMethod;
  path: string;
  body: unknown;
  createdAt: number;
  attempts: number;
  nextAttemptAt: number;
  lastError: string | null;
  state: OutboxState;
}

export interface OutboxCounts {
  pending: number;
  failed: number;
}

export interface OutboxStore {
  insert(item: OutboxItem): Promise<void>;
  /** Bekleyen kayıtlar, eklenme sırasıyla. */
  listPending(): Promise<OutboxItem[]>;
  update(item: OutboxItem): Promise<void>;
  remove(id: string): Promise<void>;
  counts(): Promise<OutboxCounts>;
}

export type OutboxSender = (item: OutboxItem) => Promise<void>;

export interface FlushResult {
  sent: number;
  failed: number;
  remaining: number;
}

interface OutboxOptions {
  now?: () => number;
  newId: () => string;
  baseDelayMs?: number;
  maxDelayMs?: number;
}

/**
 * Öğretmen girişlerinin bağlantı yokken kaybolmamasını sağlayan giden işlem kuyruğu
 * (gereksinim 5, Çevrimdışı). Kayıtlar eklenme sırasıyla gönderilir; geçici hatada (ağ, sunucu)
 * kuyruk durur ve üstel bekleme uygulanır. Kalıcı hatalar (doğrulama, yetki) kuyruğu tıkamaz,
 * FAILED olarak işaretlenip kullanıcıya gösterilir.
 */
export class Outbox {
  private readonly store: OutboxStore;
  private readonly send: OutboxSender;
  private readonly now: () => number;
  private readonly newId: () => string;
  private readonly baseDelayMs: number;
  private readonly maxDelayMs: number;
  private flushing: Promise<FlushResult> | null = null;

  constructor(store: OutboxStore, send: OutboxSender, options: OutboxOptions) {
    this.store = store;
    this.send = send;
    this.now = options.now ?? Date.now;
    this.newId = options.newId;
    this.baseDelayMs = options.baseDelayMs ?? 2_000;
    this.maxDelayMs = options.maxDelayMs ?? 5 * 60_000;
  }

  async enqueue(method: HttpMethod, path: string, body: unknown): Promise<string> {
    const now = this.now();
    const id = this.newId();
    await this.store.insert({
      id,
      method,
      path,
      body,
      createdAt: now,
      attempts: 0,
      nextAttemptAt: now,
      lastError: null,
      state: 'PENDING',
    });
    return id;
  }

  counts(): Promise<OutboxCounts> {
    return this.store.counts();
  }

  /**
   * Aynı anda tek bir boşaltma çalışır; eşzamanlı çağrılar aynı sonucu bekler.
   * `ignoreBackoff`: bağlantı geri geldiğinde veya kullanıcı kaydettiğinde bekleme süresini atlar.
   */
  flush(ignoreBackoff = false): Promise<FlushResult> {
    this.flushing ??= this.flushOnce(ignoreBackoff).finally(() => {
      this.flushing = null;
    });
    return this.flushing;
  }

  private async flushOnce(ignoreBackoff: boolean): Promise<FlushResult> {
    let sent = 0;
    let failed = 0;
    const items = await this.store.listPending();

    for (const item of items) {
      if (!ignoreBackoff && item.nextAttemptAt > this.now()) break;
      try {
        await this.send(item);
        await this.store.remove(item.id);
        sent += 1;
      } catch (error) {
        const code = isApiError(error) ? error.code : 'unknown';
        if (isApiError(error) && !error.retryable && error.code !== 'unauthorized') {
          await this.store.update({ ...item, state: 'FAILED', attempts: item.attempts + 1, lastError: code });
          failed += 1;
          continue;
        }
        // Geçici hata veya oturum yenileniyor: sırayı korumak için dur.
        const attempts = item.attempts + 1;
        await this.store.update({ ...item, attempts, lastError: code, nextAttemptAt: this.now() + this.delayFor(attempts) });
        break;
      }
    }

    const { pending } = await this.store.counts();
    return { sent, failed, remaining: pending };
  }

  private delayFor(attempts: number): number {
    return Math.min(this.maxDelayMs, this.baseDelayMs * 2 ** (attempts - 1));
  }
}
