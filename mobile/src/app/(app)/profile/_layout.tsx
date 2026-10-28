import { Stack } from 'expo-router';
import { useTranslation } from 'react-i18next';

import { OutboxIndicator } from '@/ui/OutboxIndicator';

export const unstable_settings = { anchor: 'index' };

export default function ProfileLayout() {
  const { t } = useTranslation();
  return (
    <Stack screenOptions={{ headerBackButtonDisplayMode: 'minimal' }}>
      <Stack.Screen name="index" options={{ title: t('profile.title'), headerRight: () => <OutboxIndicator /> }} />
      <Stack.Screen name="privacy" options={{ title: t('privacy.title') }} />
      <Stack.Screen name="requests" options={{ title: t('kvkk.title') }} />
    </Stack>
  );
}
