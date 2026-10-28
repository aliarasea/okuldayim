import { useTranslation } from 'react-i18next';

import { useActiveChild, useActiveMembership } from '@/core/session/sessionStore';
import { ParentPickup } from '@/features/pickup/ParentPickup';
import { TeacherPickupQueue } from '@/features/pickup/TeacherPickupQueue';
import { Screen } from '@/ui/Screen';
import { EmptyState } from '@/ui/StateViews';

export default function PickupTab() {
  const { t } = useTranslation();
  const membership = useActiveMembership();
  const child = useActiveChild();

  if (membership?.role === 'OGRETMEN') {
    const classId = membership.classIds[0];
    if (classId) return <TeacherPickupQueue key={classId} classId={classId} />;
  }
  if (membership?.role === 'VELI' && child) {
    return <ParentPickup key={child.id} child={child} siblings={membership.children} />;
  }
  return (
    <Screen>
      <EmptyState text={membership?.role === 'OGRETMEN' ? t('meals.noClass') : t('common.noChild')} />
    </Screen>
  );
}
