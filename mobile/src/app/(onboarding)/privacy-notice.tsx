import { useMutation, useQuery } from '@tanstack/react-query';
import { useRouter } from 'expo-router';
import { useTranslation } from 'react-i18next';

import { api } from '@/core/api';
import { errorMessage } from '@/core/api/errorMessage';
import type { PrivacyNotice } from '@/core/api/types';
import { loadMe } from '@/core/session/sessionService';
import { selectOnboardingStep, useSessionStore } from '@/core/session/sessionStore';
import { NoticeView } from '@/features/privacy/NoticeView';
import { Banner } from '@/ui/Banner';
import { Button } from '@/ui/Button';
import { Screen } from '@/ui/Screen';
import { ErrorState, LoadingState } from '@/ui/StateViews';

/** KVKK-01: aydınlatma, açık rızadan ayrı ve önce gösterilir; okuma kaydı metin sürümüyle saklanır. */
export default function NoticeScreen() {
  const { t } = useTranslation();
  const router = useRouter();
  const kurumId = useSessionStore((state) => state.me?.pendingNoticeKurumIds[0] ?? null);

  const notice = useQuery({
    queryKey: ['aydinlatma', kurumId],
    queryFn: () => api.privacy.notice(kurumId ?? ''),
    enabled: kurumId !== null,
  });

  const acknowledge = useMutation({
    mutationFn: (current: PrivacyNotice) => api.privacy.acknowledgeNotice(current.kurumId, current.version),
    onSuccess: async () => {
      await loadMe();
      // Onboarding bittiyse koruma kuralı ana ekrana yönlendirir; rıza adımı varsa oraya geçilir.
      if (selectOnboardingStep(useSessionStore.getState()) === 'consents') router.replace('/consents');
    },
  });

  return (
    <Screen>
      <Banner tone="info">{t('notice.infoOnly')}</Banner>
      {notice.isPending ? (
        <LoadingState />
      ) : notice.isError ? (
        <ErrorState error={notice.error} onRetry={() => void notice.refetch()} />
      ) : (
        <NoticeView notice={notice.data} />
      )}
      {acknowledge.isError ? <Banner tone="danger">{errorMessage(acknowledge.error)}</Banner> : null}
      <Button
        title={t('notice.acknowledge')}
        disabled={!notice.data}
        loading={acknowledge.isPending}
        onPress={() => notice.data && acknowledge.mutate(notice.data)}
      />
    </Screen>
  );
}
