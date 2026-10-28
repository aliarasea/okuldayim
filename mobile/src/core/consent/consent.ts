import type { ConsentDecision, ConsentItem, ConsentType } from '../api/types';

export const CONSENT_TYPES: readonly ConsentType[] = [
  'SAGLIK',
  'FOTO_VELI',
  'FOTO_GRUP',
  'FOTO_TANITIM',
  'KONUM',
  'AKIS_YETKILI',
];

/**
 * KVKK-04: Velilerden birinin rızası yeterlidir; ancak velilerden biri açıkça reddettiyse veya
 * rızasını geri aldıysa çocuk için rıza uygulanmaz (en kısıtlayıcı rıza). Karar vermemiş veli
 * (null) diğerinin rızasını engellemez.
 */
export function resolveEffectiveConsent(decisions: readonly (ConsentDecision | null)[]): boolean {
  return decisions.includes('GRANTED') && !decisions.includes('DENIED');
}

export type ConsentStatus = 'effective' | 'deniedByMe' | 'blockedByOtherGuardian' | 'notGiven';

export function consentStatus(item: Pick<ConsentItem, 'myDecision' | 'otherGuardianDenied' | 'effective'>): ConsentStatus {
  if (item.effective) return 'effective';
  if (item.myDecision === 'DENIED') return 'deniedByMe';
  if (item.otherGuardianDenied) return 'blockedByOtherGuardian';
  return 'notGiven';
}
