import { ApiError, apiErrorFromResponse } from './errors';
import type { Transport, TransportRequest } from './transport';

export interface AccessTokenProvider {
  /** Geçerli erişim anahtarı; süresi dolmak üzereyse yeniler. Oturum yoksa null. */
  getValidAccessToken(): Promise<string | null>;
  /** Anahtarı yeniler; oturum sona erdiyse null döner. */
  refresh(): Promise<string | null>;
}

export interface ApiRequest extends TransportRequest {
  /** false ise Authorization başlığı eklenmez (ör. OTP istekleri). */
  auth?: boolean;
  /** Tekrarlanan isteğin sunucuda bir kez işlenmesini sağlar (çevrimdışı kuyruk). */
  idempotencyKey?: string;
}

export class ApiClient {
  private readonly transport: Transport;
  private readonly tokens: AccessTokenProvider;
  private readonly defaultHeaders: () => Record<string, string>;

  constructor(transport: Transport, tokens: AccessTokenProvider, defaultHeaders: () => Record<string, string> = () => ({})) {
    this.transport = transport;
    this.tokens = tokens;
    this.defaultHeaders = defaultHeaders;
  }

  async request<T>(request: ApiRequest): Promise<T> {
    const { auth = true, idempotencyKey, ...rest } = request;
    const send = (token: string | null) =>
      this.transport.send({
        ...rest,
        headers: {
          ...this.defaultHeaders(),
          ...rest.headers,
          ...(token ? { Authorization: `Bearer ${token}` } : {}),
          ...(idempotencyKey ? { 'Idempotency-Key': idempotencyKey } : {}),
        },
      });

    let token: string | null = null;
    if (auth) {
      token = await this.tokens.getValidAccessToken();
      if (!token) throw new ApiError('unauthorized', 401);
    }

    let response = await send(token);
    if (auth && response.status === 401) {
      token = await this.tokens.refresh();
      if (!token) throw new ApiError('unauthorized', 401);
      response = await send(token);
    }

    if (response.status >= 200 && response.status < 300) {
      return response.body as T;
    }
    throw apiErrorFromResponse(response.status, response.body);
  }
}
