import { Platform } from 'react-native';

import type { ApiClient } from './client';
import type {
  AuthorizedPerson,
  ChildConsents,
  ClassToday,
  ConsentDecision,
  ConsentType,
  DailyFeed,
  DataExportRequest,
  KvkkRequest,
  KvkkRequestType,
  Me,
  OtpChallenge,
  PickupRequest,
  PrivacyNotice,
  TokenPair,
} from './types';

const seg = encodeURIComponent;

/** Çevrimdışı kuyruk üzerinden gönderilen isteklerin yolları. */
export const outboxPaths = {
  mealEntries: (classId: string) => `/v1/siniflar/${seg(classId)}/ogun-kayitlari`,
  auditEvents: '/v1/denetim/olaylar',
} as const;

export function createEndpoints(client: ApiClient) {
  return {
    auth: {
      requestOtp: (phone: string) =>
        client.request<OtpChallenge>({ method: 'POST', path: '/v1/auth/otp', body: { phone }, auth: false }),
      verifyOtp: (challengeId: string, code: string, deviceId: string) =>
        client.request<TokenPair>({
          method: 'POST',
          path: '/v1/auth/otp/verify',
          body: { challengeId, code, device: { id: deviceId, platform: Platform.OS } },
          auth: false,
        }),
      logout: (refreshToken: string) =>
        client.request<null>({ method: 'POST', path: '/v1/auth/logout', body: { refreshToken }, auth: false }),
    },

    me: {
      get: () => client.request<Me>({ method: 'GET', path: '/v1/me' }),
    },

    privacy: {
      notice: (kurumId: string) =>
        client.request<PrivacyNotice>({ method: 'GET', path: `/v1/kurumlar/${seg(kurumId)}/aydinlatma` }),
      acknowledgeNotice: (kurumId: string, version: string) =>
        client.request<null>({
          method: 'POST',
          path: `/v1/kurumlar/${seg(kurumId)}/aydinlatma/${seg(version)}/okundu`,
        }),
      consents: (childId: string) =>
        client.request<ChildConsents>({ method: 'GET', path: `/v1/cocuklar/${seg(childId)}/rizalar` }),
      saveConsents: (childId: string, textVersion: string, decisions: { type: ConsentType; decision: ConsentDecision }[]) =>
        client.request<ChildConsents>({
          method: 'PUT',
          path: `/v1/cocuklar/${seg(childId)}/rizalar`,
          body: { textVersion, decisions },
        }),
      requests: () => client.request<KvkkRequest[]>({ method: 'GET', path: '/v1/kvkk/basvurular' }),
      createRequest: (kurumId: string, type: KvkkRequestType, description: string) =>
        client.request<KvkkRequest>({ method: 'POST', path: '/v1/kvkk/basvurular', body: { kurumId, type, description } }),
      requestDataExport: (childId: string) =>
        client.request<DataExportRequest>({ method: 'POST', path: '/v1/kvkk/veri-dokumu', body: { childId } }),
    },

    feed: {
      daily: (childId: string, date: string) =>
        client.request<DailyFeed>({ method: 'GET', path: `/v1/cocuklar/${seg(childId)}/akis`, query: { tarih: date } }),
    },

    classroom: {
      today: (classId: string) =>
        client.request<ClassToday>({ method: 'GET', path: `/v1/siniflar/${seg(classId)}/bugun` }),
    },

    pickup: {
      authorizedPersons: (childId: string) =>
        client.request<AuthorizedPerson[]>({ method: 'GET', path: `/v1/cocuklar/${seg(childId)}/yetkili-kisiler` }),
      activeRequest: (childId: string) =>
        client.request<PickupRequest | null>({
          method: 'GET',
          path: `/v1/cocuklar/${seg(childId)}/teslim-talepleri/aktif`,
        }),
      create: (childId: string, pickupPersonId: string, etaMinutes: number) =>
        client.request<PickupRequest>({
          method: 'POST',
          path: '/v1/teslim-talepleri',
          body: { childId, pickupPersonId, etaMinutes },
        }),
      queue: (classId: string) =>
        client.request<PickupRequest[]>({ method: 'GET', path: `/v1/siniflar/${seg(classId)}/teslim-kuyrugu` }),
      markPreparing: (requestId: string) =>
        client.request<PickupRequest>({ method: 'POST', path: `/v1/teslim-talepleri/${seg(requestId)}/hazirlaniyor` }),
      handOver: (requestId: string, pin: string) =>
        client.request<PickupRequest>({
          method: 'POST',
          path: `/v1/teslim-talepleri/${seg(requestId)}/teslim`,
          body: { pin },
        }),
    },
  };
}

export type Endpoints = ReturnType<typeof createEndpoints>;
