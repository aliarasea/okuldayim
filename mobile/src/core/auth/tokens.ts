import { apiErrorFromResponse } from '../api/errors';
import type { Transport } from '../api/transport';
import type { TokenPair } from '../api/types';

export interface RefreshTokenStorage {
  get(): Promise<string | null>;
  set(token: string): Promise<void>;
  clear(): Promise<void>;
}

type SessionEndedListener = () => void;

const EXPIRY_MARGIN_MS = 30_000;

/**
 * Erişim anahtarı yalnızca bellekte, yenileme anahtarı güvenli depoda (Keychain/Keystore) tutulur.
 * Yenileme tek uçuşludur: aynı anda alınan 401'ler tek bir yenileme isteği üretir. Sunucu yenileme
 * anahtarını reddederse (süre dolumu, uzaktan oturum kapatma) dinleyiciler bilgilendirilir.
 */
export class TokenManager {
  private readonly storage: RefreshTokenStorage;
  private readonly transport: Transport;
  private readonly now: () => number;
  private readonly listeners = new Set<SessionEndedListener>();
  private accessToken: string | null = null;
  private expiresAt = 0;
  private inflight: Promise<string | null> | null = null;

  constructor(storage: RefreshTokenStorage, transport: Transport, now: () => number = Date.now) {
    this.storage = storage;
    this.transport = transport;
    this.now = now;
  }

  async setTokens(pair: TokenPair): Promise<void> {
    this.accessToken = pair.accessToken;
    this.expiresAt = this.now() + pair.expiresInSec * 1000;
    await this.storage.set(pair.refreshToken);
  }

  async hasStoredSession(): Promise<boolean> {
    return (await this.storage.get()) !== null;
  }

  getRefreshToken(): Promise<string | null> {
    return this.storage.get();
  }

  async getValidAccessToken(): Promise<string | null> {
    if (this.accessToken && this.now() < this.expiresAt - EXPIRY_MARGIN_MS) {
      return this.accessToken;
    }
    return this.refresh();
  }

  refresh(): Promise<string | null> {
    this.inflight ??= this.refreshOnce().finally(() => {
      this.inflight = null;
    });
    return this.inflight;
  }

  async clear(): Promise<void> {
    this.accessToken = null;
    this.expiresAt = 0;
    await this.storage.clear();
  }

  onSessionEnded(listener: SessionEndedListener): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  private async refreshOnce(): Promise<string | null> {
    const refreshToken = await this.storage.get();
    if (!refreshToken) return null;

    // Ağ hataları yukarı fırlatılır: çevrimdışıyken oturum kapatılmaz.
    const response = await this.transport.send({ method: 'POST', path: '/v1/auth/refresh', body: { refreshToken } });
    if (response.status === 200) {
      await this.setTokens(response.body as TokenPair);
      return this.accessToken;
    }
    if (response.status === 400 || response.status === 401) {
      await this.clear();
      this.listeners.forEach((listener) => listener());
      return null;
    }
    throw apiErrorFromResponse(response.status, response.body);
  }
}
