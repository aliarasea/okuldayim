import { useNetworkState } from 'expo-network';
import { useEffect } from 'react';
import { AppState } from 'react-native';

import { useSessionStore } from '../session/sessionStore';
import { syncOutbox } from '.';

const SYNC_INTERVAL_MS = 30_000;

/** Bağlantı geri geldiğinde, uygulama öne geldiğinde ve belirli aralıklarla kuyruğu gönderir. */
export function OutboxSync() {
  const signedIn = useSessionStore((state) => state.status === 'signedIn');
  const { isConnected } = useNetworkState();

  useEffect(() => {
    if (signedIn && isConnected !== false) void syncOutbox(true);
  }, [signedIn, isConnected]);

  useEffect(() => {
    if (!signedIn) return;
    const subscription = AppState.addEventListener('change', (status) => {
      if (status === 'active') void syncOutbox();
    });
    const timer = setInterval(() => void syncOutbox(), SYNC_INTERVAL_MS);
    return () => {
      subscription.remove();
      clearInterval(timer);
    };
  }, [signedIn]);

  return null;
}
