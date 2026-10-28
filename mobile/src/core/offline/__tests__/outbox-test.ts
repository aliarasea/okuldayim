import { ApiError } from '../../api/errors';
import { MemoryOutboxStore } from '../memoryOutboxStore';
import { Outbox, type OutboxItem } from '../outbox';

function setup(send: (item: OutboxItem) => Promise<void>) {
  let now = 1_000;
  let id = 0;
  const store = new MemoryOutboxStore();
  const outbox = new Outbox(store, send, {
    now: () => now,
    newId: () => `key-${++id}`,
    baseDelayMs: 1_000,
    maxDelayMs: 8_000,
  });
  return { store, outbox, advance: (ms: number) => (now += ms) };
}

describe('Outbox', () => {
  it('kayıtları eklenme sırasıyla gönderir ve idempotency anahtarı olarak kimliği kullanır', async () => {
    const sent: string[] = [];
    const { outbox, advance } = setup(async (item) => {
      sent.push(item.id);
    });
    await outbox.enqueue('POST', '/a', { n: 1 });
    advance(1);
    await outbox.enqueue('POST', '/b', { n: 2 });

    await expect(outbox.flush()).resolves.toEqual({ sent: 2, failed: 0, remaining: 0 });
    expect(sent).toEqual(['key-1', 'key-2']);
  });

  it('geçici hatada durur, sırayı korur ve üstel bekleme uygular', async () => {
    let online = false;
    const sent: string[] = [];
    const { store, outbox, advance } = setup(async (item) => {
      if (!online) throw new ApiError('network', 0);
      sent.push(item.path);
    });
    await outbox.enqueue('POST', '/a', {});
    advance(1);
    await outbox.enqueue('POST', '/b', {});

    await expect(outbox.flush()).resolves.toEqual({ sent: 0, failed: 0, remaining: 2 });
    const [first] = await store.listPending();
    expect(first?.attempts).toBe(1);
    expect(first?.lastError).toBe('network');

    online = true;
    // Bekleme süresi dolmadan gönderilmez...
    await expect(outbox.flush()).resolves.toMatchObject({ sent: 0, remaining: 2 });
    // ...bağlantı geri geldiğinde bekleme atlanabilir.
    await expect(outbox.flush(true)).resolves.toEqual({ sent: 2, failed: 0, remaining: 0 });
    expect(sent).toEqual(['/a', '/b']);
  });

  it('bekleme süresi üst sınırı aşmaz', async () => {
    const { store, outbox, advance } = setup(async () => {
      throw new ApiError('server', 503);
    });
    await outbox.enqueue('POST', '/a', {});
    for (let attempt = 0; attempt < 6; attempt += 1) {
      await outbox.flush(true);
      advance(1);
    }
    const [item] = await store.listPending();
    expect(item?.attempts).toBe(6);
    expect((item?.nextAttemptAt ?? 0) - 1_000).toBeLessThanOrEqual(8_000 + 6);
  });

  it('kalıcı hatayı FAILED olarak işaretler ve sonraki kayıtları göndermeye devam eder', async () => {
    const { outbox, advance } = setup(async (item) => {
      if (item.path === '/bad') throw new ApiError('validation', 422);
    });
    await outbox.enqueue('POST', '/bad', {});
    advance(1);
    await outbox.enqueue('POST', '/good', {});

    await expect(outbox.flush()).resolves.toEqual({ sent: 1, failed: 1, remaining: 0 });
    await expect(outbox.counts()).resolves.toEqual({ pending: 0, failed: 1 });
  });

  it('oturum hatasında kaydı silmez, yeniden dener', async () => {
    const { outbox } = setup(async () => {
      throw new ApiError('unauthorized', 401);
    });
    await outbox.enqueue('POST', '/a', {});
    await expect(outbox.flush()).resolves.toEqual({ sent: 0, failed: 0, remaining: 1 });
  });

  it('eşzamanlı boşaltma çağrıları tek gönderim üretir', async () => {
    const send = jest.fn(async () => undefined);
    const { outbox } = setup(send);
    await outbox.enqueue('POST', '/a', {});
    await Promise.all([outbox.flush(), outbox.flush(), outbox.flush()]);
    expect(send).toHaveBeenCalledTimes(1);
  });
});
