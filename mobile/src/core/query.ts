import { focusManager, onlineManager, QueryClient } from '@tanstack/react-query';
import { addNetworkStateListener } from 'expo-network';
import { AppState } from 'react-native';

import { isApiError } from './api/errors';

// Sunucu verisi önbelleği yalnızca bellekte tutulur, diske yazılmaz; oturum kapanınca temizlenir.
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 30_000,
      retry: (failureCount, error) => isApiError(error) && error.retryable && failureCount < 2,
    },
    mutations: {
      retry: false,
    },
  },
});

onlineManager.setEventListener((setOnline) => {
  const subscription = addNetworkStateListener((state) => setOnline(state.isConnected !== false));
  return () => subscription.remove();
});

focusManager.setEventListener((handleFocus) => {
  const subscription = AppState.addEventListener('change', (status) => handleFocus(status === 'active'));
  return () => subscription.remove();
});
