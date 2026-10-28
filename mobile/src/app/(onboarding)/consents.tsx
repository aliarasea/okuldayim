import { useTranslation } from 'react-i18next';

import { loadMe } from '@/core/session/sessionService';
import { findChild, useSessionStore } from '@/core/session/sessionStore';
import { ConsentEditor } from '@/features/consent/ConsentEditor';
import { Screen } from '@/ui/Screen';

/** KVKK-02: her çocuk için rızalar ayrı ayrı istenir; hiçbirini vermeden devam etmek mümkündür. */
export default function OnboardingConsentsScreen() {
  const { t } = useTranslation();
  const child = useSessionStore((state) => {
    const childId = state.me?.consentReviewChildIds[0];
    return childId ? findChild(state.me, childId) : undefined;
  });

  if (!child) return null;

  return (
    <Screen>
      <ConsentEditor key={child.id} child={child} submitLabel={t('consent.saveAndContinue')} onSaved={loadMe} />
    </Screen>
  );
}
