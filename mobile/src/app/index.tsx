import { Redirect } from 'expo-router';

import { useActiveRole, useOnboardingStep, useSessionStore } from '@/core/session/sessionStore';

/** Oturum ve onboarding durumuna göre doğru ekrana yönlendirir; açılış ekranı bu sırada görünür kalır. */
export default function Index() {
  const status = useSessionStore((state) => state.status);
  const step = useOnboardingStep();
  const role = useActiveRole();

  if (status === 'booting') return null;
  if (status === 'signedOut') return <Redirect href="/login" />;
  if (step === 'notice') return <Redirect href="/privacy-notice" />;
  if (step === 'consents') return <Redirect href="/consents" />;
  if (role === 'OGRETMEN') return <Redirect href="/meals" />;
  if (role === 'VELI') return <Redirect href="/feed" />;
  return <Redirect href="/profile" />;
}
