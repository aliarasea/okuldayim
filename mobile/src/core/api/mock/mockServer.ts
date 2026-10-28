// Backend hazır olana kadar kullanılan bellek içi sunucu. Sözleşme ve kurallar gerçek backend'le aynı
// tutulur: kapsam kontrolü, en kısıtlayıcı rıza, rızaya bağlı alerji görünürlüğü, tek kullanımlık
// teslim kodu ve OTP deneme sınırı sunucu tarafında uygulanır.
import { CONSENT_TYPES, resolveEffectiveConsent } from '../../consent/consent';
import { toLocalDateString } from '../../date';
import type { HttpMethod, Transport, TransportRequest, TransportResponse } from '../transport';
import type {
  ChildConsents,
  ClassChild,
  ConsentDecision,
  ConsentType,
  FeedEntry,
  KvkkRequest,
  KvkkRequestType,
  MealAmount,
  MealSlot,
  Me,
  PickupRequest,
  PickupStatus,
} from '../types';
import {
  authorizedPersons,
  CHILDREN,
  CLASSES,
  CONSENT_TEXT_VERSION,
  CONSENTS,
  DEMO_OTP,
  initialFeed,
  KURUM,
  MENU,
  type MockAuthorizedPerson,
  type MockChild,
  type MockConsent,
  type MockUser,
  NOTICE,
  USERS,
} from './seed';

const ACCESS_TTL_SEC = 15 * 60;
const OTP_TTL_MS = 3 * 60_000;
const MAX_OTP_ATTEMPTS = 5;
const MAX_PIN_ATTEMPTS = 3;
const KVKK_RESPONSE_DAYS = 30;
const MEAL_SLOTS: readonly MealSlot[] = ['KAHVALTI', 'ARA_OGUN', 'OGLE', 'IKINDI'];
const MEAL_AMOUNTS: readonly MealAmount[] = ['HIC', 'AZ', 'YARISI', 'COGU', 'TAMAMI'];
const KVKK_TYPES: readonly KvkkRequestType[] = ['BILGI', 'DUZELTME', 'SILME', 'ITIRAZ', 'HESAP_SILME'];

interface MockPickup {
  id: string;
  childId: string;
  requestedBy: string;
  person: MockAuthorizedPerson;
  etaMinutes: number;
  createdAt: string;
  status: PickupStatus;
  pin: string;
  failedAttempts: number;
  deliveredAt: string | null;
}

interface Context {
  params: string[];
  body: Record<string, unknown>;
  query: Record<string, string>;
  headers: Record<string, string>;
  user: MockUser;
  path: string;
}

type Handler = (ctx: Context) => TransportResponse;

interface Route {
  method: HttpMethod;
  pattern: RegExp;
  auth: boolean;
  handle: Handler;
}

export interface MockTransportOptions {
  latencyMs?: number;
  now?: () => number;
}

export function createMockTransport({ latencyMs = 250, now = Date.now }: MockTransportOptions = {}): Transport {
  const startedAt = new Date(now());
  const users = new Map(USERS.map((user) => [user.id, user]));
  const children = new Map(CHILDREN.map((child) => [child.id, child]));
  const consents = new Map<string, MockConsent>(
    CONSENTS.map(([key, decision]) => [key, { decision, textVersion: CONSENT_TEXT_VERSION, at: startedAt.toISOString() }]),
  );
  const consentReviews = new Set<string>();
  const acknowledgements = new Set<string>();
  const persons = authorizedPersons(startedAt);
  const feed = new Map<string, FeedEntry[]>(
    Object.entries(initialFeed(startedAt)).map(([childId, entries]) => [feedKey(childId, toLocalDateString(startedAt)), entries]),
  );
  const pickups: MockPickup[] = [];
  const kvkkRequests: { userId: string; request: KvkkRequest }[] = [];
  const challenges = new Map<string, { userId: string | null; expiresAt: number; attempts: number }>();
  const accessTokens = new Map<string, { userId: string; familyId: string; expiresAt: number }>();
  const refreshTokens = new Map<string, { userId: string; familyId: string; used: boolean }>();
  const revokedFamilies = new Set<string>();
  const idempotency = new Map<string, TransportResponse>();
  const auditEvents: unknown[] = [];

  // --- Yardımcılar ---
  const isoNow = () => new Date(now()).toISOString();
  const childName = (child: MockChild) => `${child.firstName} ${child.lastInitial}.`;
  const consentKey = (childId: string, guardianId: string, type: ConsentType) => `${childId}|${guardianId}|${type}`;

  const isGuardian = (user: MockUser, childId: string) =>
    user.memberships.some((m) => m.role === 'VELI' && m.childIds.includes(childId));
  const isTeacherOf = (user: MockUser, classId: string) =>
    user.memberships.some((m) => m.role === 'OGRETMEN' && m.classIds.includes(classId));
  const isMemberOf = (user: MockUser, kurumId: string) => user.memberships.some((m) => m.kurumId === kurumId);

  const effectiveConsent = (child: MockChild, type: ConsentType) =>
    resolveEffectiveConsent(child.guardianIds.map((id) => consents.get(consentKey(child.id, id, type))?.decision ?? null));

  const consentsFor = (child: MockChild, user: MockUser): ChildConsents => ({
    childId: child.id,
    textVersion: CONSENT_TEXT_VERSION,
    items: CONSENT_TYPES.map((type) => {
      const mine = consents.get(consentKey(child.id, user.id, type)) ?? null;
      const others = child.guardianIds
        .filter((id) => id !== user.id)
        .map((id) => consents.get(consentKey(child.id, id, type))?.decision ?? null);
      return {
        type,
        myDecision: mine?.decision ?? null,
        otherGuardianDenied: others.includes('DENIED'),
        effective: resolveEffectiveConsent([mine?.decision ?? null, ...others]),
        decidedAt: mine?.at ?? null,
      };
    }),
  });

  const issueTokens = (userId: string, familyId: string) => {
    const accessToken = randomToken();
    const refreshToken = randomToken();
    accessTokens.set(accessToken, { userId, familyId, expiresAt: now() + ACCESS_TTL_SEC * 1000 });
    refreshTokens.set(refreshToken, { userId, familyId, used: false });
    return { accessToken, refreshToken, expiresInSec: ACCESS_TTL_SEC };
  };

  const authenticate = (headers: Record<string, string>): MockUser | null => {
    const token = headers.Authorization?.replace(/^Bearer /, '');
    const session = token ? accessTokens.get(token) : undefined;
    if (!session || session.expiresAt < now() || revokedFamilies.has(session.familyId)) return null;
    return users.get(session.userId) ?? null;
  };

  const pickupDto = (pickup: MockPickup, viewer: MockUser): PickupRequest => {
    const child = children.get(pickup.childId);
    return {
      id: pickup.id,
      childId: pickup.childId,
      childName: child ? childName(child) : '',
      pickupPerson: { id: pickup.person.id, displayName: pickup.person.displayName, relation: pickup.person.relation },
      etaMinutes: pickup.etaMinutes,
      createdAt: pickup.createdAt,
      status: pickup.status,
      pin: pickup.requestedBy === viewer.id && pickup.status !== 'TESLIM_EDILDI' ? pickup.pin : null,
    };
  };

  const isActivePickup = (pickup: MockPickup) =>
    pickup.status === 'YOLDA' ||
    pickup.status === 'HAZIRLANIYOR' ||
    pickup.status === 'ENGELLENDI' ||
    (pickup.status === 'TESLIM_EDILDI' && !!pickup.deliveredAt && now() - Date.parse(pickup.deliveredAt) < 30 * 60_000);

  const addFeedEntry = (childId: string, entry: FeedEntry) => {
    const key = feedKey(childId, toLocalDateString(new Date(now())));
    feed.set(key, [...(feed.get(key) ?? []), entry]);
  };

  // --- Rotalar ---
  const routes: Route[] = [
    route('POST', '/v1/auth/otp', false, ({ body }) => {
      const phone = normalizePhone(body.phone);
      if (!phone) return fail(422, 'validation', { field: 'phone' });
      const user = USERS.find((candidate) => candidate.phone === phone) ?? null;
      const challengeId = randomToken();
      challenges.set(challengeId, { userId: user?.id ?? null, expiresAt: now() + OTP_TTL_MS, attempts: 0 });
      // Numara kayıtlı olmasa da aynı yanıt döner: kullanıcı varlığı dışarıya sızdırılmaz.
      return ok({ challengeId, expiresInSec: OTP_TTL_MS / 1000, retryAfterSec: 60 });
    }),

    route('POST', '/v1/auth/otp/verify', false, ({ body }) => {
      const challengeId = String(body.challengeId ?? '');
      const challenge = challenges.get(challengeId);
      if (!challenge || challenge.expiresAt < now()) return fail(400, 'otp_expired');
      if (challenge.attempts >= MAX_OTP_ATTEMPTS) return fail(429, 'otp_locked');
      challenge.attempts += 1;
      if (body.code !== DEMO_OTP || !challenge.userId) {
        const attemptsLeft = MAX_OTP_ATTEMPTS - challenge.attempts;
        return attemptsLeft > 0 ? fail(400, 'otp_invalid', { attemptsLeft }) : fail(429, 'otp_locked');
      }
      challenges.delete(challengeId);
      return ok(issueTokens(challenge.userId, randomToken()));
    }),

    route('POST', '/v1/auth/refresh', false, ({ body }) => {
      const stored = refreshTokens.get(String(body.refreshToken ?? ''));
      if (!stored || revokedFamilies.has(stored.familyId)) return fail(401, 'unauthorized');
      if (stored.used) {
        // Kullanılmış yenileme anahtarı tekrar geldi: anahtar çalınmış olabilir, tüm aile iptal edilir.
        revokedFamilies.add(stored.familyId);
        return fail(401, 'unauthorized');
      }
      stored.used = true;
      return ok(issueTokens(stored.userId, stored.familyId));
    }),

    route('POST', '/v1/auth/logout', false, ({ body }) => {
      const stored = refreshTokens.get(String(body.refreshToken ?? ''));
      if (stored) revokedFamilies.add(stored.familyId);
      return noContent();
    }),

    route('GET', '/v1/me', true, ({ user }) => {
      const me: Me = {
        user: { id: user.id, displayName: user.displayName },
        memberships: user.memberships.map((membership) => ({
          id: membership.id,
          kurumId: membership.kurumId,
          kurumName: KURUM.name,
          role: membership.role,
          classIds: membership.classIds,
          children: membership.childIds.flatMap((childId) => {
            const child = children.get(childId);
            if (!child) return [];
            const klass = CLASSES.find((c) => c.id === child.classId);
            return [
              {
                id: child.id,
                firstName: child.firstName,
                lastInitial: child.lastInitial,
                classId: child.classId,
                className: klass?.name ?? '',
              },
            ];
          }),
        })),
        pendingNoticeKurumIds: unique(user.memberships.map((m) => m.kurumId)).filter(
          (kurumId) => !acknowledgements.has(`${user.id}|${kurumId}|${NOTICE.version}`),
        ),
        consentReviewChildIds: unique(
          user.memberships.filter((m) => m.role === 'VELI').flatMap((m) => m.childIds),
        ).filter((childId) => !consentReviews.has(`${childId}|${user.id}|${CONSENT_TEXT_VERSION}`)),
      };
      return ok(me);
    }),

    route('GET', '/v1/kurumlar/:kurumId/aydinlatma', true, ({ user, params: [kurumId] }) => {
      if (kurumId !== KURUM.id || !isMemberOf(user, kurumId)) return fail(403, 'forbidden');
      return ok({
        kurumId,
        kurumName: KURUM.name,
        version: NOTICE.version,
        publishedAt: NOTICE.publishedAt,
        summary: NOTICE.summary,
        fullText: NOTICE.fullText,
        acknowledged: acknowledgements.has(`${user.id}|${kurumId}|${NOTICE.version}`),
      });
    }),

    route('POST', '/v1/kurumlar/:kurumId/aydinlatma/:version/okundu', true, ({ user, params: [kurumId, version] }) => {
      if (kurumId !== KURUM.id || !isMemberOf(user, kurumId)) return fail(403, 'forbidden');
      if (version !== NOTICE.version) return fail(409, 'conflict');
      acknowledgements.add(`${user.id}|${kurumId}|${version}`);
      return noContent();
    }),

    route('GET', '/v1/cocuklar/:childId/rizalar', true, ({ user, params: [childId = ''] }) => {
      const child = children.get(childId);
      if (!child || !isGuardian(user, childId)) return fail(403, 'forbidden');
      return ok(consentsFor(child, user));
    }),

    route('PUT', '/v1/cocuklar/:childId/rizalar', true, ({ user, params: [childId = ''], body }) => {
      const child = children.get(childId);
      if (!child || !isGuardian(user, childId)) return fail(403, 'forbidden');
      if (body.textVersion !== CONSENT_TEXT_VERSION) return fail(409, 'conflict');
      const decisions = Array.isArray(body.decisions) ? (body.decisions as { type?: unknown; decision?: unknown }[]) : null;
      if (!decisions) return fail(422, 'validation', { field: 'decisions' });
      for (const item of decisions) {
        if (!CONSENT_TYPES.includes(item.type as ConsentType) || (item.decision !== 'GRANTED' && item.decision !== 'DENIED')) {
          return fail(422, 'validation', { field: 'decisions' });
        }
      }
      for (const item of decisions) {
        consents.set(consentKey(childId, user.id, item.type as ConsentType), {
          decision: item.decision as ConsentDecision,
          textVersion: CONSENT_TEXT_VERSION,
          at: isoNow(),
        });
      }
      consentReviews.add(`${childId}|${user.id}|${CONSENT_TEXT_VERSION}`);
      return ok(consentsFor(child, user));
    }),

    route('GET', '/v1/cocuklar/:childId/akis', true, ({ user, params: [childId = ''], query }) => {
      const child = children.get(childId);
      if (!child || !(isGuardian(user, childId) || isTeacherOf(user, child.classId))) return fail(403, 'forbidden');
      const date = query.tarih ?? toLocalDateString(new Date(now()));
      const entries = [...(feed.get(feedKey(childId, date)) ?? [])].sort((a, b) => a.at.localeCompare(b.at));
      return ok({ childId, date, entries });
    }),

    route('GET', '/v1/siniflar/:classId/bugun', true, ({ user, params: [classId = ''] }) => {
      const klass = CLASSES.find((c) => c.id === classId);
      if (!klass || !isTeacherOf(user, classId)) return fail(403, 'forbidden');
      const classChildren: ClassChild[] = CHILDREN.filter((child) => child.classId === classId).map((child) => ({
        id: child.id,
        firstName: child.firstName,
        lastInitial: child.lastInitial,
        // Sağlık rızası yoksa alerji bilgisi öğretmene hiç gönderilmez.
        allergens: effectiveConsent(child, 'SAGLIK') ? child.allergens : null,
      }));
      return ok({ classId, className: klass.name, date: toLocalDateString(new Date(now())), children: classChildren, menu: MENU });
    }),

    route('POST', '/v1/siniflar/:classId/ogun-kayitlari', true, ({ user, params: [classId = ''], body }) => {
      if (!isTeacherOf(user, classId)) return fail(403, 'forbidden');
      const slot = body.slot as MealSlot;
      const entries = Array.isArray(body.entries) ? (body.entries as { childId?: unknown; amount?: unknown }[]) : [];
      if (!MEAL_SLOTS.includes(slot) || entries.length === 0) return fail(422, 'validation');
      for (const entry of entries) {
        const child = children.get(String(entry.childId));
        if (!child || child.classId !== classId || !MEAL_AMOUNTS.includes(entry.amount as MealAmount)) {
          return fail(422, 'validation');
        }
      }
      const date = toLocalDateString(new Date(now()));
      for (const entry of entries) {
        const childId = String(entry.childId);
        const key = feedKey(childId, date);
        // Aynı öğün yeniden girilirse önceki kayıt güncellenir.
        const others = (feed.get(key) ?? []).filter((item) => !(item.kind === 'MEAL' && item.slot === slot));
        feed.set(key, [...others, { id: randomToken(), kind: 'MEAL', at: isoNow(), slot, amount: entry.amount as MealAmount }]);
      }
      return created({ saved: entries.length });
    }),

    route('GET', '/v1/cocuklar/:childId/yetkili-kisiler', true, ({ user, params: [childId = ''] }) => {
      if (!isGuardian(user, childId)) return fail(403, 'forbidden');
      return ok(persons[childId] ?? []);
    }),

    route('GET', '/v1/cocuklar/:childId/teslim-talepleri/aktif', true, ({ user, params: [childId = ''] }) => {
      if (!isGuardian(user, childId)) return fail(403, 'forbidden');
      const active = pickups.filter((p) => p.childId === childId && isActivePickup(p)).at(-1);
      return ok(active ? pickupDto(active, user) : null);
    }),

    route('POST', '/v1/teslim-talepleri', true, ({ user, body }) => {
      const childId = String(body.childId ?? '');
      const etaMinutes = Number(body.etaMinutes);
      if (!isGuardian(user, childId)) return fail(403, 'forbidden');
      const person = (persons[childId] ?? []).find((p) => p.id === body.pickupPersonId);
      const expired = person?.validUntil ? Date.parse(person.validUntil) < now() : false;
      if (!person || !person.verified || expired) return fail(422, 'validation', { field: 'pickupPersonId' });
      if (!Number.isInteger(etaMinutes) || etaMinutes < 1 || etaMinutes > 120) return fail(422, 'validation', { field: 'etaMinutes' });
      if (pickups.some((p) => p.childId === childId && isActivePickup(p) && p.status !== 'TESLIM_EDILDI')) {
        return fail(409, 'conflict');
      }
      const pickup: MockPickup = {
        id: randomToken(),
        childId,
        requestedBy: user.id,
        person,
        etaMinutes,
        createdAt: isoNow(),
        status: 'YOLDA',
        pin: String(Math.floor(1000 + Math.random() * 9000)),
        failedAttempts: 0,
        deliveredAt: null,
      };
      pickups.push(pickup);
      return created(pickupDto(pickup, user));
    }),

    route('GET', '/v1/siniflar/:classId/teslim-kuyrugu', true, ({ user, params: [classId = ''] }) => {
      if (!isTeacherOf(user, classId)) return fail(403, 'forbidden');
      const queue = pickups
        .filter((p) => children.get(p.childId)?.classId === classId && isActivePickup(p) && p.status !== 'TESLIM_EDILDI')
        .sort((a, b) => arrivalTime(a) - arrivalTime(b))
        .map((p) => pickupDto(p, user));
      return ok(queue);
    }),

    route('POST', '/v1/teslim-talepleri/:id/hazirlaniyor', true, ({ user, params: [id] }) => {
      const pickup = pickups.find((p) => p.id === id);
      const child = pickup ? children.get(pickup.childId) : undefined;
      if (!pickup || !child || !isTeacherOf(user, child.classId)) return fail(403, 'forbidden');
      if (pickup.status !== 'YOLDA') return fail(409, 'conflict');
      pickup.status = 'HAZIRLANIYOR';
      return ok(pickupDto(pickup, user));
    }),

    route('POST', '/v1/teslim-talepleri/:id/teslim', true, ({ user, params: [id], body }) => {
      const pickup = pickups.find((p) => p.id === id);
      const child = pickup ? children.get(pickup.childId) : undefined;
      if (!pickup || !child || !isTeacherOf(user, child.classId)) return fail(403, 'forbidden');
      if (pickup.status === 'ENGELLENDI') return fail(423, 'pickup_blocked');
      if (pickup.status === 'TESLIM_EDILDI') return fail(409, 'conflict');
      if (body.pin !== pickup.pin) {
        pickup.failedAttempts += 1;
        if (pickup.failedAttempts >= MAX_PIN_ATTEMPTS) {
          // PICK-07: teslim durur; gerçek sistemde veli ve yöneticiye anında bildirim gider.
          pickup.status = 'ENGELLENDI';
          auditEvents.push({ type: 'TESLIM_ENGELLENDI', pickupId: pickup.id, at: isoNow() });
          return fail(423, 'pickup_blocked');
        }
        return fail(422, 'pin_invalid', { attemptsLeft: MAX_PIN_ATTEMPTS - pickup.failedAttempts });
      }
      pickup.status = 'TESLIM_EDILDI';
      pickup.deliveredAt = isoNow();
      addFeedEntry(pickup.childId, {
        id: randomToken(),
        kind: 'PICKUP',
        at: pickup.deliveredAt,
        pickupPersonName: pickup.person.displayName,
        relation: pickup.person.relation,
      });
      return ok(pickupDto(pickup, user));
    }),

    route('GET', '/v1/kvkk/basvurular', true, ({ user }) =>
      ok(kvkkRequests.filter((entry) => entry.userId === user.id).map((entry) => entry.request)),
    ),

    route('POST', '/v1/kvkk/basvurular', true, ({ user, body }) => {
      const kurumId = String(body.kurumId ?? '');
      const type = body.type as KvkkRequestType;
      const description = typeof body.description === 'string' ? body.description : '';
      if (!isMemberOf(user, kurumId)) return fail(403, 'forbidden');
      if (!KVKK_TYPES.includes(type) || description.length > 2000) return fail(422, 'validation');
      const createdAt = new Date(now());
      const request: KvkkRequest = {
        id: randomToken(),
        kurumId,
        type,
        createdAt: createdAt.toISOString(),
        dueAt: new Date(createdAt.getTime() + KVKK_RESPONSE_DAYS * 24 * 60 * 60_000).toISOString(),
        status: 'ACIK',
      };
      kvkkRequests.push({ userId: user.id, request });
      return created(request);
    }),

    route('POST', '/v1/kvkk/veri-dokumu', true, ({ user, body }) => {
      if (!isGuardian(user, String(body.childId ?? ''))) return fail(403, 'forbidden');
      return json(202, { requestId: randomToken(), linkValidHours: 24 });
    }),

    route('POST', '/v1/denetim/olaylar', true, ({ user, body }) => {
      auditEvents.push({ ...body, userId: user.id, receivedAt: isoNow() });
      return noContent();
    }),
  ];

  const dispatch = (request: TransportRequest): TransportResponse => {
    const headers = request.headers ?? {};
    for (const candidate of routes) {
      if (candidate.method !== request.method) continue;
      const match = candidate.pattern.exec(request.path);
      if (!match) continue;
      const user = candidate.auth ? authenticate(headers) : null;
      if (candidate.auth && !user) return fail(401, 'unauthorized');
      const idempotencyKey = headers['Idempotency-Key'];
      const replayKey = idempotencyKey ? `${user?.id ?? ''}|${request.path}|${idempotencyKey}` : null;
      const replay = replayKey ? idempotency.get(replayKey) : undefined;
      if (replay) return replay;
      const response = candidate.handle({
        params: match.slice(1).map((value) => decodeURIComponent(value)),
        body: (request.body ?? {}) as Record<string, unknown>,
        query: request.query ?? {},
        headers,
        user: user ?? ANONYMOUS,
        path: request.path,
      });
      if (replayKey && response.status < 500) idempotency.set(replayKey, response);
      return response;
    }
    return fail(404, 'not_found');
  };

  return {
    async send(request) {
      if (latencyMs > 0) await delay(latencyMs);
      return clone(dispatch(request));
    },
  };
}

// --- Modül düzeyi yardımcılar ---
const ANONYMOUS: MockUser = { id: '', phone: null, displayName: '', memberships: [] };

function route(method: HttpMethod, template: string, auth: boolean, handle: Handler): Route {
  return { method, pattern: new RegExp(`^${template.replace(/:[a-zA-Z]+/g, '([^/]+)')}$`), auth, handle };
}

function json(status: number, body: unknown): TransportResponse {
  return { status, body };
}

const ok = (body: unknown) => json(200, body);
const created = (body: unknown) => json(201, body);
const noContent = () => json(204, null);
const fail = (status: number, code: string, details?: Record<string, unknown>) =>
  json(status, { error: { code, ...(details ? { details } : {}) } });

function feedKey(childId: string, date: string): string {
  return `${childId}|${date}`;
}

function arrivalTime(pickup: MockPickup): number {
  return Date.parse(pickup.createdAt) + pickup.etaMinutes * 60_000;
}

function normalizePhone(input: unknown): string | null {
  if (typeof input !== 'string') return null;
  const digits = input.replace(/\D/g, '');
  const national = digits.startsWith('90') ? digits.slice(2) : digits.replace(/^0/, '');
  return /^5\d{9}$/.test(national) ? `+90${national}` : null;
}

function unique<T>(values: T[]): T[] {
  return [...new Set(values)];
}

function randomToken(): string {
  return `${Math.random().toString(36).slice(2)}${Math.random().toString(36).slice(2)}`;
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function clone<T>(value: T): T {
  return value === null || value === undefined ? value : (JSON.parse(JSON.stringify(value)) as T);
}
