/** Cihazın yerel saat dilimine göre YYYY-MM-DD. */
export function toLocalDateString(date: Date): string {
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${date.getFullYear()}-${month}-${day}`;
}

export function formatTime(iso: string, locale: string): string {
  return new Intl.DateTimeFormat(locale, { hour: '2-digit', minute: '2-digit' }).format(new Date(iso));
}

export function formatLongDate(localDate: string, locale: string): string {
  const [year, month, day] = localDate.split('-').map(Number);
  return new Intl.DateTimeFormat(locale, { weekday: 'long', day: 'numeric', month: 'long' }).format(
    new Date(year ?? 1970, (month ?? 1) - 1, day ?? 1),
  );
}

export function formatShortDate(iso: string, locale: string): string {
  return new Intl.DateTimeFormat(locale, { day: 'numeric', month: 'long', year: 'numeric' }).format(new Date(iso));
}

/** Oluşturulma anı + tahmini süre üzerinden kalan dakika (negatifse 0). */
export function minutesRemaining(createdAtIso: string, etaMinutes: number, now: number = Date.now()): number {
  const arrival = new Date(createdAtIso).getTime() + etaMinutes * 60_000;
  return Math.max(0, Math.round((arrival - now) / 60_000));
}
