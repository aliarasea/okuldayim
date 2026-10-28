// Çalışma zamanı yapılandırması. EXPO_PUBLIC_* değişkenleri derlemede pakete gömülür; sır içermez.
export type ApiMode = 'mock' | 'http';

const apiMode: ApiMode = process.env.EXPO_PUBLIC_API_MODE === 'http' ? 'http' : 'mock';
const apiBaseUrl = process.env.EXPO_PUBLIC_API_URL ?? 'http://127.0.0.1:8080';

// KVKK m.12: üretim derlemesinde şifresiz bağlantıya izin verilmez.
if (!__DEV__ && apiMode === 'http' && !apiBaseUrl.startsWith('https://')) {
  throw new Error('EXPO_PUBLIC_API_URL üretimde https:// ile başlamalı');
}

export const config = {
  apiMode,
  apiBaseUrl,
  requestTimeoutMs: 15_000,
} as const;
