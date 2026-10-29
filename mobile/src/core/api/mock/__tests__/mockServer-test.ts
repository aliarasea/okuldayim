import type { Transport, TransportResponse } from '../../transport';
import type { ChildConsents, ClassToday, Me, PickupRequest, TokenPair } from '../../types';
import { createMockTransport } from '../mockServer';

const NOW = new Date(2026, 9, 28, 16, 0).getTime();

async function signIn(transport: Transport, phone: string): Promise<string> {
  const challenge = await transport.send({ method: 'POST', path: '/v1/auth/otp', body: { phone } });
  const { challengeId } = challenge.body as { challengeId: string };
  const verified = await transport.send({ method: 'POST', path: '/v1/auth/otp/verify', body: { challengeId, code: '123456' } });
  return (verified.body as TokenPair).accessToken;
}

function authed(transport: Transport, token: string) {
  return (method: 'GET' | 'POST' | 'PUT', path: string, body?: unknown, headers: Record<string, string> = {}) =>
    transport.send({ method, path, body, headers: { Authorization: `Bearer ${token}`, ...headers } });
}

const errorCode = (response: TransportResponse) => (response.body as { error?: { code?: string } }).error?.code;

describe('mock sunucu: kimlik doğrulama', () => {
  it('kayıtlı olmayan numara için de aynı yanıtı verir (kullanıcı varlığı sızmaz)', async () => {
    const transport = createMockTransport({ latencyMs: 0, now: () => NOW });
    const known = await transport.send({ method: 'POST', path: '/v1/auth/otp', body: { phone: '+905550000001' } });
    const unknown = await transport.send({ method: 'POST', path: '/v1/auth/otp', body: { phone: '+905559999999' } });
    expect(known.status).toBe(200);
    expect(unknown.status).toBe(200);
    expect(Object.keys(unknown.body as object).sort()).toEqual(Object.keys(known.body as object).sort());

    const verify = await transport.send({
      method: 'POST',
      path: '/v1/auth/otp/verify',
      body: { challengeId: (unknown.body as { challengeId: string }).challengeId, code: '123456' },
    });
    expect(errorCode(verify)).toBe('otp_invalid');
  });

  it('5 hatalı denemeden sonra kodu kilitler', async () => {
    const transport = createMockTransport({ latencyMs: 0, now: () => NOW });
    const { body } = await transport.send({ method: 'POST', path: '/v1/auth/otp', body: { phone: '05550000001' } });
    const challengeId = (body as { challengeId: string }).challengeId;
    const codes: (string | undefined)[] = [];
    for (let i = 0; i < 6; i += 1) {
      const response = await transport.send({ method: 'POST', path: '/v1/auth/otp/verify', body: { challengeId, code: '000000' } });
      codes.push(errorCode(response));
    }
    expect(codes).toEqual(['otp_invalid', 'otp_invalid', 'otp_invalid', 'otp_invalid', 'otp_locked', 'otp_locked']);
  });

  it('kullanılmış yenileme anahtarı tekrar gelirse tüm oturum ailesini iptal eder', async () => {
    const transport = createMockTransport({ latencyMs: 0, now: () => NOW });
    const { body } = await transport.send({ method: 'POST', path: '/v1/auth/otp', body: { phone: '+905550000001' } });
    const verified = await transport.send({
      method: 'POST',
      path: '/v1/auth/otp/verify',
      body: { challengeId: (body as { challengeId: string }).challengeId, code: '123456' },
    });
    const first = verified.body as TokenPair;
    const rotated = await transport.send({ method: 'POST', path: '/v1/auth/refresh', body: { refreshToken: first.refreshToken } });
    expect(rotated.status).toBe(200);

    const replay = await transport.send({ method: 'POST', path: '/v1/auth/refresh', body: { refreshToken: first.refreshToken } });
    expect(replay.status).toBe(401);
    const afterTheft = await transport.send({
      method: 'POST',
      path: '/v1/auth/refresh',
      body: { refreshToken: (rotated.body as TokenPair).refreshToken },
    });
    expect(afterTheft.status).toBe(401);
  });
});

describe('mock sunucu: KVKK kuralları', () => {
  it('yeni veliden önce aydınlatma, sonra rıza incelemesi ister', async () => {
    const transport = createMockTransport({ latencyMs: 0, now: () => NOW });
    const call = authed(transport, await signIn(transport, '+905550000001'));

    let me = (await call('GET', '/v1/me')).body as Me;
    expect(me.pendingNoticeKurumIds).toEqual(['k-papatya']);
    expect(me.consentReviewChildIds).toEqual(['ch-ada', 'ch-can']);

    expect((await call('POST', '/v1/organizations/k-papatya/privacy-notice/2026-10/acknowledgement')).status).toBe(204);
    // Hiç rıza vermeden kaydetmek de incelemeyi tamamlar (rıza hizmet şartı değildir).
    expect((await call('PUT', '/v1/children/ch-ada/consents', { textVersion: '2026-10', decisions: [] })).status).toBe(200);

    me = (await call('GET', '/v1/me')).body as Me;
    expect(me.pendingNoticeKurumIds).toEqual([]);
    expect(me.consentReviewChildIds).toEqual(['ch-can']);
  });

  it('en kısıtlayıcı rızayı uygular: diğer velinin reddi, bu velinin onayını etkisiz kılar', async () => {
    const transport = createMockTransport({ latencyMs: 0, now: () => NOW });
    const call = authed(transport, await signIn(transport, '+905550000001'));

    const saved = await call('PUT', '/v1/children/ch-ada/consents', {
      textVersion: '2026-10',
      decisions: [{ type: 'FOTO_GRUP', decision: 'GRANTED' }],
    });
    const group = (saved.body as ChildConsents).items.find((item) => item.type === 'FOTO_GRUP');
    expect(group).toMatchObject({ myDecision: 'GRANTED', otherGuardianDenied: true, effective: false });
  });

  it('başka bir çocuğun rızalarına erişimi engeller', async () => {
    const transport = createMockTransport({ latencyMs: 0, now: () => NOW });
    const call = authed(transport, await signIn(transport, '+905550000001'));
    expect((await call('GET', '/v1/children/ch-ela/consents')).status).toBe(403);
  });

  it('sağlık rızası olmayan çocuğun alerji bilgisini öğretmene göndermez', async () => {
    const transport = createMockTransport({ latencyMs: 0, now: () => NOW });
    const call = authed(transport, await signIn(transport, '+905550000002'));
    const classroom = (await call('GET', '/v1/classes/c-kelebek/today')).body as ClassToday;
    const allergens = Object.fromEntries(classroom.children.map((child) => [child.id, child.allergens]));
    expect(allergens['ch-ada']).toEqual(['fıstık']);
    expect(allergens['ch-mert']).toBeNull();
    expect((await call('GET', '/v1/classes/c-ari/today')).status).toBe(403);
  });

  it('aynı Idempotency-Key ile gelen öğün kaydını bir kez işler', async () => {
    const transport = createMockTransport({ latencyMs: 0, now: () => NOW });
    const teacher = authed(transport, await signIn(transport, '+905550000002'));
    const entry = (amount: string) => ({ date: '2026-10-28', slot: 'IKINDI', entries: [{ childId: 'ch-ada', amount }] });
    const path = '/v1/classes/c-kelebek/meal-records';

    expect((await teacher('POST', path, entry('AZ'), { 'Idempotency-Key': 'k1' })).status).toBe(201);
    // Aynı anahtarla tekrar gönderim (ör. çevrimdışı kuyruğun yeniden denemesi) yeniden işlenmez.
    expect((await teacher('POST', path, entry('TAMAMI'), { 'Idempotency-Key': 'k1' })).status).toBe(201);

    const parent = authed(transport, await signIn(transport, '+905550000001'));
    const feed = (await parent('GET', '/v1/children/ch-ada/feed')).body as { entries: { kind: string; slot?: string; amount?: string }[] };
    const meals = feed.entries.filter((item) => item.kind === 'MEAL' && item.slot === 'IKINDI');
    expect(meals).toHaveLength(1);
    expect(meals[0]?.amount).toBe('AZ');
  });
});

describe('mock sunucu: teslim alma', () => {
  async function createPickup() {
    const transport = createMockTransport({ latencyMs: 0, now: () => NOW });
    const parent = authed(transport, await signIn(transport, '+905550000001'));
    const teacher = authed(transport, await signIn(transport, '+905550000002'));
    const created = await parent('POST', '/v1/pickup-requests', { childId: 'ch-ada', pickupPersonId: 'ap-ada-fatma', etaMinutes: 10 });
    return { parent, teacher, created };
  }

  it('kodu yalnızca veliye gösterir, öğretmen kuyruğunda kod yoktur', async () => {
    const { teacher, created } = await createPickup();
    const request = created.body as PickupRequest;
    expect(created.status).toBe(201);
    expect(request.pin).toMatch(/^\d{4}$/);

    const queue = (await teacher('GET', '/v1/classes/c-kelebek/pickup-queue')).body as PickupRequest[];
    expect(queue).toHaveLength(1);
    expect(queue[0]?.pin).toBeNull();
  });

  it('süresi dolmuş veya onaylanmamış yetkili kişiyle teslim talebi oluşturulamaz', async () => {
    const transport = createMockTransport({ latencyMs: 0, now: () => NOW });
    const parent = authed(transport, await signIn(transport, '+905550000001'));
    for (const pickupPersonId of ['ap-ada-gul', 'ap-ada-murat']) {
      const response = await parent('POST', '/v1/pickup-requests', { childId: 'ch-ada', pickupPersonId, etaMinutes: 10 });
      expect(errorCode(response)).toBe('validation');
    }
  });

  it('doğru kodla teslim eder ve veli akışına işler', async () => {
    const { parent, teacher, created } = await createPickup();
    const { id, pin } = created.body as PickupRequest;
    const delivered = await teacher('POST', `/v1/pickup-requests/${id}/handover`, { pin });
    expect((delivered.body as PickupRequest).status).toBe('TESLIM_EDILDI');

    const feed = (await parent('GET', '/v1/children/ch-ada/feed')).body as { entries: { kind: string }[] };
    expect(feed.entries.some((entry) => entry.kind === 'PICKUP')).toBe(true);
  });

  it('üç hatalı kodda teslimi durdurur (PICK-07)', async () => {
    const { teacher, created } = await createPickup();
    const { id, pin } = created.body as PickupRequest;
    const wrong = pin === '0000' ? '1111' : '0000';
    const codes: (string | undefined)[] = [];
    for (let i = 0; i < 3; i += 1) {
      codes.push(errorCode(await teacher('POST', `/v1/pickup-requests/${id}/handover`, { pin: wrong })));
    }
    expect(codes).toEqual(['pin_invalid', 'pin_invalid', 'pickup_blocked']);
    // Doğru kod artık kabul edilmez.
    expect(errorCode(await teacher('POST', `/v1/pickup-requests/${id}/handover`, { pin }))).toBe('pickup_blocked');
  });
});
