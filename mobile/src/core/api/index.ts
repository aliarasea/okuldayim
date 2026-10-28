import { secureRefreshTokenStorage } from '../auth/secureStorage';
import { TokenManager } from '../auth/tokens';
import { config } from '../config';
import i18n from '../i18n';
import { ApiClient } from './client';
import { createEndpoints } from './endpoints';
import { createHttpTransport } from './httpTransport';
import { createMockTransport } from './mock/mockServer';

const transport =
  config.apiMode === 'http' ? createHttpTransport(config.apiBaseUrl, config.requestTimeoutMs) : createMockTransport();

export const tokenManager = new TokenManager(secureRefreshTokenStorage, transport);

export const apiClient = new ApiClient(transport, tokenManager, () => ({ 'Accept-Language': i18n.language }));

export const api = createEndpoints(apiClient);
