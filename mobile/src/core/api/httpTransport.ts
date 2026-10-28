import { ApiError } from './errors';
import type { Transport, TransportRequest } from './transport';

export function createHttpTransport(baseUrl: string, timeoutMs: number): Transport {
  return {
    async send(request) {
      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(), timeoutMs);
      try {
        const response = await fetch(buildUrl(baseUrl, request), {
          method: request.method,
          headers: {
            Accept: 'application/json',
            ...(request.body === undefined ? {} : { 'Content-Type': 'application/json' }),
            ...request.headers,
          },
          body: request.body === undefined ? undefined : JSON.stringify(request.body),
          signal: controller.signal,
        });
        const text = await response.text();
        return { status: response.status, body: text ? parseJson(text) : null };
      } catch {
        throw new ApiError(controller.signal.aborted ? 'timeout' : 'network', 0);
      } finally {
        clearTimeout(timer);
      }
    },
  };
}

function buildUrl(baseUrl: string, { path, query }: TransportRequest): string {
  const search = query ? `?${new URLSearchParams(query).toString()}` : '';
  return `${baseUrl.replace(/\/$/, '')}${path}${search}`;
}

function parseJson(text: string): unknown {
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
}
