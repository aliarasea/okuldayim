import '@/core/i18n';

import { QueryClientProvider } from '@tanstack/react-query';
import { Stack } from 'expo-router';
import * as ScreenCapture from 'expo-screen-capture';
import * as SplashScreen from 'expo-splash-screen';
import { StatusBar } from 'expo-status-bar';
import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Platform } from 'react-native';

import { OutboxSync } from '@/core/offline/OutboxSync';
import { logger } from '@/core/privacy/logger';
import { queryClient } from '@/core/query';
import { bootstrapSession } from '@/core/session/sessionService';
import { useOnboardingStep, useSessionStore } from '@/core/session/sessionStore';

void SplashScreen.preventAutoHideAsync();

export default function RootLayout() {
  const { t } = useTranslation();
  const status = useSessionStore((state) => state.status);
  const onboarding = useOnboardingStep() !== null;
  const signedIn = status === 'signedIn';

  useEffect(() => {
    void bootstrapSession();
    if (Platform.OS === 'ios') {
      // KVKK-15: uygulama arka plandayken görev değiştiricide içerik bulanıklaştırılır.
      ScreenCapture.enableAppSwitcherProtectionAsync(0.9).catch((error: unknown) =>
        logger.warn('Görev değiştirici koruması açılamadı', error),
      );
    }
  }, []);

  useEffect(() => {
    if (status !== 'booting') SplashScreen.hide();
  }, [status]);

  return (
    <QueryClientProvider client={queryClient}>
      <StatusBar style="auto" />
      <OutboxSync />
      <Stack screenOptions={{ headerBackButtonDisplayMode: 'minimal' }}>
        <Stack.Screen name="index" options={{ headerShown: false }} />
        <Stack.Protected guard={!signedIn} redirectTo="/">
          <Stack.Screen name="(auth)" options={{ headerShown: false }} />
        </Stack.Protected>
        <Stack.Protected guard={signedIn && onboarding} redirectTo="/">
          <Stack.Screen name="(onboarding)" options={{ headerShown: false }} />
        </Stack.Protected>
        <Stack.Protected guard={signedIn && !onboarding} redirectTo="/">
          <Stack.Screen name="(app)" options={{ headerShown: false }} />
          <Stack.Screen name="verify-pickup/[id]" options={{ presentation: 'modal', title: t('pickup.verifyTitle') }} />
        </Stack.Protected>
      </Stack>
    </QueryClientProvider>
  );
}
