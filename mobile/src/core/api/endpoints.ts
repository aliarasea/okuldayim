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
  mealEntries: (classId: string) => `/v1/classes/${seg(classId)}/meal-records`,
  auditEvents: '/v1/audit/events',
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
        client.request<PrivacyNotice>({ method: 'GET', path: `/v1/organizations/${seg(kurumId)}/privacy-notice` }),
      acknowledgeNotice: (kurumId: string, version: string) =>
        client.request<null>({
          method: 'POST',
          path: `/v1/organizations/${seg(kurumId)}/privacy-notice/${seg(version)}/acknowledgement`,
        }),
      consents: (childId: string) =>
        client.request<ChildConsents>({ method: 'GET', path: `/v1/children/${seg(childId)}/consents` }),
      saveConsents: (childId: string, textVersion: string, decisions: { type: ConsentType; decision: ConsentDecision }[]) =>
        client.request<ChildConsents>({
          method: 'PUT',
          path: `/v1/children/${seg(childId)}/consents`,
          body: { textVersion, decisions },
        }),
      requests: () => client.request<KvkkRequest[]>({ method: 'GET', path: '/v1/privacy/requests' }),
      createRequest: (kurumId: string, type: KvkkRequestType, description: string) =>
        client.request<KvkkRequest>({ method: 'POST', path: '/v1/privacy/requests', body: { kurumId, type, description } }),
      requestDataExport: (childId: string) =>
        client.request<DataExportRequest>({ method: 'POST', path: '/v1/privacy/data-exports', body: { childId } }),
    },

    feed: {
      daily: (childId: string, date: string) =>
        client.request<DailyFeed>({ method: 'GET', path: `/v1/children/${seg(childId)}/feed`, query: { date } }),
    },

    classroom: {
      today: (classId: string) =>
        client.request<ClassToday>({ method: 'GET', path: `/v1/classes/${seg(classId)}/today` }),
    },

    pickup: {
      authorizedPersons: (childId: string) =>
        client.request<AuthorizedPerson[]>({ method: 'GET', path: `/v1/children/${seg(childId)}/authorized-pickup-people` }),
      activeRequest: (childId: string) =>
        client.request<PickupRequest | null>({
          method: 'GET',
          path: `/v1/children/${seg(childId)}/pickup-requests/active`,
        }),
      create: (childId: string, pickupPersonId: string, etaMinutes: number, idempotencyKey: string) =>
        client.request<PickupRequest>({
          method: 'POST',
          path: '/v1/pickup-requests',
          body: { childId, pickupPersonId, etaMinutes },
          idempotencyKey,
        }),
      queue: (classId: string) =>
        client.request<PickupRequest[]>({ method: 'GET', path: `/v1/classes/${seg(classId)}/pickup-queue` }),
      markPreparing: (requestId: string) =>
        client.request<PickupRequest>({ method: 'POST', path: `/v1/pickup-requests/${seg(requestId)}/preparing` }),
      handOver: (requestId: string, pin: string) =>
        client.request<PickupRequest>({
          method: 'POST',
          path: `/v1/pickup-requests/${seg(requestId)}/handover`,
          body: { pin },
        }),
    },
  };
}

export type Endpoints = ReturnType<typeof createEndpoints>;
