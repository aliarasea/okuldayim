export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'DELETE';

export interface TransportRequest {
  method: HttpMethod;
  path: string;
  query?: Record<string, string>;
  body?: unknown;
  headers?: Record<string, string>;
}

export interface TransportResponse {
  status: number;
  body: unknown;
}

/**
 * HTTP istemcisi ile geliştirme amaçlı mock sunucunun ortak arayüzü.
 * Ağ hatalarında `ApiError('network' | 'timeout')` fırlatır; HTTP hata kodlarını yanıt olarak döner.
 */
export interface Transport {
  send(request: TransportRequest): Promise<TransportResponse>;
}
