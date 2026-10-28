import { redact, redactValue } from '../redact';

describe('redact', () => {
  it('telefon numaralarını maskeler', () => {
    expect(redact('Arayan: +905551112233')).toBe('Arayan: [telefon]');
    expect(redact('0555 111 22 33 numarası')).toBe('[telefon] numarası');
  });

  it('e-posta, TC kimlik no ve anahtarları maskeler', () => {
    expect(redact('veli@example.com')).toBe('[e-posta]');
    expect(redact('TCKN 12345678901')).toBe('TCKN [kimlik-no]');
    expect(redact('Authorization: Bearer abc.def-123')).toBe('Authorization: Bearer [anahtar]');
    expect(redact('jwt eyJhbGciOiJIUzI1.eyJzdWIiOiIxMjM0.SflKxwRJSMeKKF2QT4')).toBe('jwt [anahtar]');
  });

  it('kişisel veri içermeyen metni değiştirmez', () => {
    expect(redact('Öğün kaydı kuyruğa alındı (3 kayıt)')).toBe('Öğün kaydı kuyruğa alındı (3 kayıt)');
  });
});

describe('redactValue', () => {
  it('hassas alanları anahtar adına göre gizler', () => {
    expect(redactValue({ phone: '+905551112233', pin: '1234', status: 'YOLDA' })).toEqual({
      phone: '[gizli]',
      pin: '[gizli]',
      status: 'YOLDA',
    });
  });

  it('iç içe nesne ve dizilerde de çalışır', () => {
    expect(redactValue({ items: [{ refreshToken: 'x', note: 'ateşi var' }], message: 'mail: a@b.co' })).toEqual({
      items: [{ refreshToken: '[gizli]', note: '[gizli]' }],
      message: 'mail: [e-posta]',
    });
  });

  it('Error nesnesinin mesajını maskeler', () => {
    expect(redactValue(new Error('0555 111 22 33 bulunamadı'))).toEqual({ name: 'Error', message: '[telefon] bulunamadı' });
  });
});
