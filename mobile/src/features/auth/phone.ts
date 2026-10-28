/** Girdiden 10 haneli ulusal cep numarasını (5XXXXXXXXX) çıkarır; +90 ve baştaki 0'ı atar. */
export function sanitizeNationalNumber(input: string): string {
  let digits = input.replace(/\D/g, '');
  if (digits.startsWith('90') && digits.length > 10) digits = digits.slice(2);
  if (digits.startsWith('0')) digits = digits.slice(1);
  return digits.slice(0, 10);
}

export function isValidNationalNumber(digits: string): boolean {
  return /^5\d{9}$/.test(digits);
}

/** 5XX XXX XX XX */
export function formatNationalNumber(digits: string): string {
  return [digits.slice(0, 3), digits.slice(3, 6), digits.slice(6, 8), digits.slice(8, 10)].filter(Boolean).join(' ');
}

export function toE164(nationalNumber: string): string {
  return `+90${nationalNumber}`;
}

/** Ekranda gösterim için: +90 5•• ••• •• 01 */
export function maskPhone(e164: string): string {
  const national = e164.replace(/^\+90/, '');
  return `+90 ${national.slice(0, 1)}•• ••• •• ${national.slice(8)}`;
}
