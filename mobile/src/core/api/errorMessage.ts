import i18n from '../i18n';
import { isApiError } from './errors';

/** Hata için kullanıcıya gösterilecek, kişisel veri içermeyen yerelleştirilmiş mesaj. */
export function errorMessage(error: unknown): string {
  if (!isApiError(error)) return i18n.t('errors.unknown');
  const attemptsLeft = Number(error.details.attemptsLeft ?? 0);
  switch (error.code) {
    case 'otp_invalid':
      return i18n.t('errors.otp_invalid', { count: attemptsLeft });
    case 'pin_invalid':
      return i18n.t('errors.pin_invalid', { count: attemptsLeft });
    default:
      return i18n.t(`errors.${error.code}`);
  }
}
