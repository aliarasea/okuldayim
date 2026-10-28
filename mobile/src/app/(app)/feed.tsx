import { useQuery } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';

import { api } from '@/core/api';
import type { ChildSummary } from '@/core/api/types';
import { formatLongDate, toLocalDateString } from '@/core/date';
import { localeFor } from '@/core/i18n';
import { sessionActions, useActiveChild, useActiveMembership, useSessionStore } from '@/core/session/sessionStore';
import { FeedEntryRow } from '@/features/feed/FeedEntryRow';
import { AppText } from '@/ui/AppText';
import { Banner } from '@/ui/Banner';
import { ChildSwitcher } from '@/ui/ChildSwitcher';
import { Screen } from '@/ui/Screen';
import { EmptyState, ErrorState, LoadingState } from '@/ui/StateViews';

/** ACT-04: velinin çocuğunun gününü izlediği akış. */
export default function FeedScreen() {
  const { t } = useTranslation();
  const membership = useActiveMembership();
  const child = useActiveChild();

  if (!membership || !child) {
    return (
      <Screen>
        <EmptyState text={t('common.noChild')} />
      </Screen>
    );
  }
  return <ChildFeed key={child.id} child={child} siblings={membership.children} />;
}

function ChildFeed({ child, siblings }: { child: ChildSummary; siblings: ChildSummary[] }) {
  const { t, i18n } = useTranslation();
  const offline = useSessionStore((state) => state.offline);
  const today = toLocalDateString(new Date());
  const feed = useQuery({ queryKey: ['akis', child.id, today], queryFn: () => api.feed.daily(child.id, today) });

  return (
    <Screen refreshing={feed.isRefetching} onRefresh={() => void feed.refetch()}>
      {offline ? <Banner tone="warning">{t('common.offlineSession')}</Banner> : null}
      <ChildSwitcher items={siblings} activeId={child.id} onChange={sessionActions.setActiveChild} />
      <AppText variant="heading">{formatLongDate(today, localeFor(i18n.language))}</AppText>
      {feed.isPending ? (
        <LoadingState />
      ) : feed.isError ? (
        <ErrorState error={feed.error} onRetry={() => void feed.refetch()} />
      ) : feed.data.entries.length === 0 ? (
        <EmptyState text={t('feed.empty')} />
      ) : (
        feed.data.entries.map((entry) => <FeedEntryRow key={entry.id} entry={entry} />)
      )}
    </Screen>
  );
}
