import { Stack } from 'expo-router';
import { useTranslation } from 'react-i18next';

export default function OnboardingLayout() {
  const { t } = useTranslation();
  return (
    <Stack screenOptions={{ headerBackVisible: false, gestureEnabled: false }}>
      <Stack.Screen name="privacy-notice" options={{ title: t('notice.title') }} />
      <Stack.Screen name="consents" options={{ title: t('consent.title') }} />
    </Stack>
  );
}
