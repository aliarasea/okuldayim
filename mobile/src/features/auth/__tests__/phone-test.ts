import { formatNationalNumber, isValidNationalNumber, maskPhone, sanitizeNationalNumber, toE164 } from '../phone';

describe('telefon numarası yardımcıları', () => {
  it.each([
    ['555 000 00 01', '5550000001'],
    ['0555 000 00 01', '5550000001'],
    ['+90 555 000 00 01', '5550000001'],
    ['905550000001', '5550000001'],
    ['55500000019999', '5550000001'],
  ])('%s → %s', (input, expected) => {
    expect(sanitizeNationalNumber(input)).toBe(expected);
  });

  it('yalnızca 5 ile başlayan 10 haneli cep numaralarını kabul eder', () => {
    expect(isValidNationalNumber('5550000001')).toBe(true);
    expect(isValidNationalNumber('2120000000')).toBe(false);
    expect(isValidNationalNumber('555000000')).toBe(false);
  });

  it('biçimlendirir, E.164 üretir ve ekranda maskeler', () => {
    expect(formatNationalNumber('5550000001')).toBe('555 000 00 01');
    expect(formatNationalNumber('5550')).toBe('555 0');
    expect(toE164('5550000001')).toBe('+905550000001');
    expect(maskPhone('+905550000001')).toBe('+90 5•• ••• •• 01');
  });
});
