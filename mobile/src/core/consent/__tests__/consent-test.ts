import { consentStatus, resolveEffectiveConsent } from '../consent';

describe('resolveEffectiveConsent (KVKK-04, en kısıtlayıcı rıza)', () => {
  it('velilerden birinin rızası yeterlidir', () => {
    expect(resolveEffectiveConsent(['GRANTED', null])).toBe(true);
  });

  it('herhangi bir velinin reddi rızayı geçersiz kılar', () => {
    expect(resolveEffectiveConsent(['GRANTED', 'DENIED'])).toBe(false);
    expect(resolveEffectiveConsent(['DENIED', 'GRANTED'])).toBe(false);
  });

  it('karar verilmemişse rıza yoktur (varsayılan kapalı)', () => {
    expect(resolveEffectiveConsent([null, null])).toBe(false);
    expect(resolveEffectiveConsent([])).toBe(false);
  });
});

describe('consentStatus', () => {
  it('diğer velinin reddini ayrıca belirtir', () => {
    expect(consentStatus({ myDecision: 'GRANTED', otherGuardianDenied: true, effective: false })).toBe(
      'blockedByOtherGuardian',
    );
  });

  it('kendi reddini, verilmemiş rızadan ayırır', () => {
    expect(consentStatus({ myDecision: 'DENIED', otherGuardianDenied: false, effective: false })).toBe('deniedByMe');
    expect(consentStatus({ myDecision: null, otherGuardianDenied: false, effective: false })).toBe('notGiven');
  });

  it('uygulanan rızayı döner', () => {
    expect(consentStatus({ myDecision: null, otherGuardianDenied: false, effective: true })).toBe('effective');
  });
});
