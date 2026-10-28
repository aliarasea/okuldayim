import { redact, redactValue } from './redact';

type Level = 'debug' | 'info' | 'warn' | 'error';

/**
 * Uygulama içi log. Mesaj ve ekler maskelenir; üretim derlemesinde konsola hiçbir şey yazılmaz.
 * Üretimde hata izleme yurt içinde barındırılan Sentry'ye bağlanacak (gereksinim 8, İzleme).
 */
function write(level: Level, message: string, meta?: unknown): void {
  if (!__DEV__) return;
  const safeMessage = `[okuldayim] ${redact(message)}`;
  if (meta === undefined) {
    console[level](safeMessage);
  } else {
    console[level](safeMessage, redactValue(meta));
  }
}

export const logger = {
  debug: (message: string, meta?: unknown) => write('debug', message, meta),
  info: (message: string, meta?: unknown) => write('info', message, meta),
  warn: (message: string, meta?: unknown) => write('warn', message, meta),
  error: (message: string, meta?: unknown) => write('error', message, meta),
};
