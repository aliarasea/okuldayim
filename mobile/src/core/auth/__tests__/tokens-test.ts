import { ApiError } from '../../api/errors';
import type { Transport, TransportRequest, TransportResponse } from '../../api/transport';
import { type RefreshTokenStorage, TokenManager } from '../tokens';

class MemoryStorage implements RefreshTokenStorage {
  value: string | null = null;
  async get() {
    return this.value;
  }
  async set(token: string) {
    this.value = token;
  }
  async clear() {
    this.value = null;
  }
}

function fakeTransport(handler: (request: TransportRequest) => Promise<TransportResponse>) {
  const send = jest.fn(handler);
  const transport: Transport = { send };
  return { transport, send };
}

describe('TokenManager', () => {
  it('erişim anahtarı geçerliyken yenileme yapmaz', async () => {
    const storage = new MemoryStorage();
    const { transport, send } = fakeTransport(async () => ({ status: 500, body: null }));
    const manager = new TokenManager(storage, transport, () => 0);
    await manager.setTokens({ accessToken: 'a1', refreshToken: 'r1', expiresInSec: 900 });

    await expect(manager.getValidAccessToken()).resolves.toBe('a1');
    expect(send).not.toHaveBeenCalled();
    expect(storage.value).toBe('r1');
  });

  it('eşzamanlı yenilemeleri tek isteğe indirir ve yenileme anahtarını döndürür', async () => {
    const storage = new MemoryStorage();
    storage.value = 'r1';
    const { transport, send } = fakeTransport(async () => ({
      status: 200,
      body: { accessToken: 'a2', refreshToken: 'r2', expiresInSec: 900 },
    }));
    const manager = new TokenManager(storage, transport, () => 0);

    const results = await Promise.all([manager.refresh(), manager.refresh(), manager.getValidAccessToken()]);
    expect(results).toEqual(['a2', 'a2', 'a2']);
    expect(send).toHaveBeenCalledTimes(1);
    expect(storage.value).toBe('r2');
  });

  it('sunucu yenileme anahtarını reddederse oturumu siler ve dinleyicileri bilgilendirir', async () => {
    const storage = new MemoryStorage();
    storage.value = 'revoked';
    const { transport } = fakeTransport(async () => ({ status: 401, body: { error: { code: 'unauthorized' } } }));
    const manager = new TokenManager(storage, transport);
    const listener = jest.fn();
    manager.onSessionEnded(listener);

    await expect(manager.refresh()).resolves.toBeNull();
    expect(storage.value).toBeNull();
    expect(listener).toHaveBeenCalledTimes(1);
  });

  it('çevrimdışıyken oturumu kapatmaz', async () => {
    const storage = new MemoryStorage();
    storage.value = 'r1';
    const { transport } = fakeTransport(async () => {
      throw new ApiError('network', 0);
    });
    const manager = new TokenManager(storage, transport);
    const listener = jest.fn();
    manager.onSessionEnded(listener);

    await expect(manager.refresh()).rejects.toMatchObject({ code: 'network' });
    expect(storage.value).toBe('r1');
    expect(listener).not.toHaveBeenCalled();
  });
});
