import type { AuthorizedPerson, PickupStatus } from '@/core/api/types';

export const ETA_OPTIONS = ['5', '10', '15', '30'] as const;
export type EtaOption = (typeof ETA_OPTIONS)[number];

export type UnavailableReason = 'pickup.personUnverified' | 'pickup.personExpired';

/**
 * PICK-05 / KVKK-07: aydınlatma bağlantısından fotoğrafını onaylamamış veya yetki süresi dolmuş kişi
 * teslim alan olarak seçilemez. Sunucu aynı kuralı ayrıca uygular.
 */
export function unavailableReason(person: AuthorizedPerson, now: number = Date.now()): UnavailableReason | null {
  if (!person.verified) return 'pickup.personUnverified';
  if (person.validUntil && Date.parse(person.validUntil) < now) return 'pickup.personExpired';
  return null;
}

export function isAwaitingHandOver(status: PickupStatus): boolean {
  return status === 'YOLDA' || status === 'HAZIRLANIYOR';
}

export const pickupQueueKey = (classId: string) => ['teslim', 'kuyruk', classId] as const;
export const activePickupKey = (childId: string) => ['teslim', 'aktif', childId] as const;
