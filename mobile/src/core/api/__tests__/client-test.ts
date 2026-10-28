import { type AccessTokenProvider, ApiClient } from '../client';
import type { Transport, TransportRequest } from '../transport';

function tokens(initial: string | null, refreshed: string | null): AccessTokenProvider & { refresh: jest.Mock } {
  return {
    getValidAccessToken: async () => initial,
    refresh: jest.fn(async () => refreshed),
  };
}

describe('ApiClient', () => {
  it('Authorization, Idempotency-Key ve varsayılan başlıkları ekler', async () => {
    const requests: TransportRequest[] = [];
    const transport: Transport = {
      send: async (request) => {
        requests.push(request);
        return { status: 201, body: { ok: true } };
      },
    };
    const client = new ApiClient(transport, tokens('a1', null), () => ({ 'Accept-Language': 'tr' }));

    await expect(client.request({ method: 'POST', path: '/x', body: {}, idempotencyKey: 'k1' })).resolves.toEqual({ ok: true });
    expect(requests[0]?.headers).toEqual({ 'Accept-Language': 'tr', Authorization: 'Bearer a1', 'Idempotency-Key': 'k1' });
  });

  it('401 alınca anahtarı yenileyip isteği bir kez tekrarlar', async () => {
    const seen: (string | undefined)[] = [];
    const transport: Transport = {
      send: async (request) => {
        seen.push(request.headers?.Authorization);
        return request.headers?.Authorization === 'Bearer a2' ? { status: 200, body: 'ok' } : { status: 401, body: null };
      },
    };
    const provider = tokens('a1', 'a2');
    const client = new ApiClient(transport, provider);

    await expect(client.request({ method: 'GET', path: '/x' })).resolves.toBe('ok');
    expect(seen).toEqual(['Bearer a1', 'Bearer a2']);
    expect(provider.refresh).toHaveBeenCalledTimes(1);
  });

  it('oturum sona erdiyse unauthorized hatası fırlatır', async () => {
    const transport: Transport = { send: async () => ({ status: 401, body: null }) };
    const client = new ApiClient(transport, tokens('a1', null));
    await expect(client.request({ method: 'GET', path: '/x' })).rejects.toMatchObject({ code: 'unauthorized' });
  });

  it('auth=false isteklerde anahtar istemez ve sunucu hata kodunu korur', async () => {
    const transport: Transport = {
      send: async (request) => {
        expect(request.headers?.Authorization).toBeUndefined();
        return { status: 400, body: { error: { code: 'otp_invalid', details: { attemptsLeft: 2 } } } };
      },
    };
    const client = new ApiClient(transport, tokens(null, null));
    await expect(client.request({ method: 'POST', path: '/v1/auth/otp/verify', auth: false })).rejects.toMatchObject({
      code: 'otp_invalid',
      status: 400,
      details: { attemptsLeft: 2 },
    });
  });
});
