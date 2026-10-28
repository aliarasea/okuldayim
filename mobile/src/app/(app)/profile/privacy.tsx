import { useMutation, useQuery } from '@tanstack/react-query';
import { useRouter } from 'expo-router';
import { useTranslation } from 'react-i18next';

import { api } from '@/core/api';
import { errorMessage } from '@/core/api/errorMessage';
import { sessionActions, useActiveChild, useActiveMembership } from '@/core/session/sessionStore';
import { ConsentEditor } from '@/features/consent/ConsentEditor';
import { NoticeView } from '@/features/privacy/NoticeView';
import { AppText } from '@/ui/AppText';
import { Banner } from '@/ui/Banner';
import { Button } from '@/ui/Button';
import { Card } from '@/ui/Card';
import { ChildSwitcher } from '@/ui/ChildSwitcher';
import { Screen } from '@/ui/Screen';
import { ErrorState, LoadingState } from '@/ui/StateViews';

/** KVKK-10: aydınlatma metni, rızalar, veri dökümü ve cihaz güvenliği tek ekranda. */
export default function PrivacyCenterScreen() {
  const { t } = useTranslation();
  const router = useRouter();
  const membership = useActiveMembership();
  const child = useActiveChild();
  const kurumId = membership?.kurumId ?? null;

  const notice = useQuery({
    queryKey: ['aydinlatma', kurumId],
    queryFn: () => api.privacy.notice(kurumId ?? ''),
    enabled: kurumId !== null,
  });
  const dataExport = useMutation({ mutationFn: (childId: string) => api.privacy.requestDataExport(childId) });

  return (
    <Screen>
      <AppText variant="heading">{t('privacy.noticeSection')}</AppText>
      {notice.isPending ? (
        <LoadingState />
      ) : notice.isError ? (
        <ErrorState error={notice.error} onRetry={() => void notice.refetch()} />
      ) : (
        <NoticeView notice={notice.data} />
      )}

      {membership?.role === 'VELI' && child ? (
        <>
          <AppText variant="heading">{t('privacy.consentsSection')}</AppText>
          <ChildSwitcher items={membership.children} activeId={child.id} onChange={sessionActions.setActiveChild} />
          <ConsentEditor key={child.id} child={child} submitLabel={t('common.save')} />

          <AppText variant="heading">{t('privacy.dataSection')}</AppText>
          <Card>
            <AppText muted>{t('privacy.dataExportHint')}</AppText>
            {dataExport.isSuccess ? (
              <Banner tone="success">{t('privacy.dataExportRequested', { hours: dataExport.data.linkValidHours })}</Banner>
            ) : null}
            {dataExport.isError ? <Banner tone="danger">{errorMessage(dataExport.error)}</Banner> : null}
            <Button
              title={t('privacy.dataExport')}
              variant="secondary"
              loading={dataExport.isPending}
              onPress={() => dataExport.mutate(child.id)}
            />
          </Card>
        </>
      ) : null}

      <AppText variant="heading">{t('privacy.deviceSection')}</AppText>
      <Card>
        <AppText muted>{t('privacy.deviceInfo')}</AppText>
      </Card>
      <Button title={t('privacy.kvkkLink')} variant="secondary" onPress={() => router.push('/profile/requests')} />
    </Screen>
  );
}
