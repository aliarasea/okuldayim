import type { AuthorizedPerson } from '@/core/api/types';

import { isAwaitingHandOver, unavailableReason } from '../pickupRules';

const NOW = Date.parse('2026-10-28T13:00:00.000Z');
const person = (overrides: Partial<AuthorizedPerson>): AuthorizedPerson => ({
  id: 'p1',
  displayName: 'Fatma Y.',
  relation: 'Babaanne',
  verified: true,
  validUntil: null,
  ...overrides,
});

describe('unavailableReason (PICK-05, KVKK-07)', () => {
  it('onaylı ve süresiz yetkili seçilebilir', () => {
    expect(unavailableReason(person({}), NOW)).toBeNull();
    expect(unavailableReason(person({ validUntil: '2026-10-29T00:00:00.000Z' }), NOW)).toBeNull();
  });

  it('aydınlatma onayını vermemiş kişi seçilemez', () => {
    expect(unavailableReason(person({ verified: false }), NOW)).toBe('pickup.personUnverified');
  });

  it('yetki süresi dolmuş kişi seçilemez', () => {
    expect(unavailableReason(person({ validUntil: '2026-10-27T00:00:00.000Z' }), NOW)).toBe('pickup.personExpired');
  });
});

describe('isAwaitingHandOver', () => {
  it('yalnızca yolda ve hazırlanıyor durumları teslim bekler', () => {
    expect(isAwaitingHandOver('YOLDA')).toBe(true);
    expect(isAwaitingHandOver('HAZIRLANIYOR')).toBe(true);
    expect(isAwaitingHandOver('TESLIM_EDILDI')).toBe(false);
    expect(isAwaitingHandOver('ENGELLENDI')).toBe(false);
  });
});
