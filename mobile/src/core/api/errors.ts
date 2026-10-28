export type ApiErrorCode =
  | 'network'
  | 'timeout'
  | 'unauthorized'
  | 'forbidden'
  | 'not_found'
  | 'conflict'
  | 'validation'
  | 'rate_limited'
  | 'otp_invalid'
  | 'otp_expired'
  | 'otp_locked'
  | 'pin_invalid'
  | 'pickup_blocked'
  | 'server'
  | 'unknown';

const KNOWN_CODES: ReadonlySet<string> = new Set<ApiErrorCode>([
  'network',
  'timeout',
  'unauthorized',
  'forbidden',
  'not_found',
  'conflict',
  'validation',
  'rate_limited',
  'otp_invalid',
  'otp_expired',
  'otp_locked',
  'pin_invalid',
  'pickup_blocked',
  'server',
  'unknown',
]);

const RETRYABLE: ReadonlySet<ApiErrorCode> = new Set<ApiErrorCode>(['network', 'timeout', 'server', 'rate_limited']);

export class ApiError extends Error {
  readonly code: ApiErrorCode;
  readonly status: number;
  readonly details: Readonly<Record<string, unknown>>;

  constructor(code: ApiErrorCode, status: number, details: Record<string, unknown> = {}) {
    super(code);
    this.name = 'ApiError';
    this.code = code;
    this.status = status;
    this.details = details;
  }

  /** Aynı isteğin daha sonra tekrar denenmesi anlamlı mı (ağ, zaman aşımı, sunucu hatası). */
  get retryable(): boolean {
    return RETRYABLE.has(this.code);
  }
}

export function isApiError(error: unknown): error is ApiError {
  return error instanceof ApiError;
}

/** Sunucu hata gövdesi biçimi: `{ "error": { "code": "otp_invalid", "details": { ... } } }` */
export function apiErrorFromResponse(status: number, body: unknown): ApiError {
  const error = (body as { error?: { code?: unknown; details?: unknown } } | null)?.error;
  const code =
    typeof error?.code === 'string' && KNOWN_CODES.has(error.code) ? (error.code as ApiErrorCode) : codeForStatus(status);
  const details =
    error?.details !== null && typeof error?.details === 'object' ? (error.details as Record<string, unknown>) : {};
  return new ApiError(code, status, details);
}

function codeForStatus(status: number): ApiErrorCode {
  if (status === 401) return 'unauthorized';
  if (status === 403) return 'forbidden';
  if (status === 404) return 'not_found';
  if (status === 409) return 'conflict';
  if (status === 400 || status === 422) return 'validation';
  if (status === 429) return 'rate_limited';
  if (status >= 500) return 'server';
  return 'unknown';
}
