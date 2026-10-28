import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useRouter } from 'expo-router';
import { useTranslation } from 'react-i18next';
import { StyleSheet, View } from 'react-native';

import { api } from '@/core/api';
import { errorMessage } from '@/core/api/errorMessage';
import type { PickupRequest } from '@/core/api/types';
import { minutesRemaining } from '@/core/date';
import { AppText } from '@/ui/AppText';
import { Banner } from '@/ui/Banner';
import { Button } from '@/ui/Button';
import { Card } from '@/ui/Card';
import { Screen } from '@/ui/Screen';
import { EmptyState, ErrorState, LoadingState } from '@/ui/StateViews';
import { spacing } from '@/ui/theme';

import { isAwaitingHandOver, pickupQueueKey } from './pickupRules';

/** PICK-03: öğretmenin canlı teslim kuyruğu (tahmini varışa göre sıralı). */
export function TeacherPickupQueue({ classId }: { classId: string }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const queue = useQuery({
    queryKey: pickupQueueKey(classId),
    queryFn: () => api.pickup.queue(classId),
    refetchInterval: 10_000,
  });
  const prepare = useMutation({
    mutationFn: (requestId: string) => api.pickup.markPreparing(requestId),
    onSettled: () => queryClient.invalidateQueries({ queryKey: pickupQueueKey(classId) }),
  });

  return (
    <Screen refreshing={queue.isRefetching} onRefresh={() => void queue.refetch()}>
      <AppText variant="heading">{t('pickup.queueTitle')}</AppText>
      {prepare.isError ? <Banner tone="danger">{errorMessage(prepare.error)}</Banner> : null}
      {queue.isPending ? (
        <LoadingState />
      ) : queue.isError ? (
        <ErrorState error={queue.error} onRetry={() => void queue.refetch()} />
      ) : queue.data.length === 0 ? (
        <EmptyState text={t('pickup.queueEmpty')} />
      ) : (
        queue.data.map((request) => (
          <QueueItem
            key={request.id}
            request={request}
            preparing={prepare.isPending && prepare.variables === request.id}
            onPrepare={() => prepare.mutate(request.id)}
          />
        ))
      )}
    </Screen>
  );
}

function QueueItem({ request, preparing, onPrepare }: { request: PickupRequest; preparing: boolean; onPrepare: () => void }) {
  const { t } = useTranslation();
  const router = useRouter();

  return (
    <Card>
      <View style={styles.header}>
        <AppText variant="heading">{request.childName}</AppText>
        <AppText variant="caption" tone={request.status === 'ENGELLENDI' ? 'danger' : 'info'}>
          {t(`pickup.status.${request.status}`)}
        </AppText>
      </View>
      <AppText>{t('pickup.person', { name: request.pickupPerson.displayName, relation: request.pickupPerson.relation })}</AppText>
      {request.status === 'ENGELLENDI' ? (
        <Banner tone="danger">{t('errors.pickup_blocked')}</Banner>
      ) : (
        <>
          <AppText muted>{t('pickup.arrivesIn', { count: minutesRemaining(request.createdAt, request.etaMinutes) })}</AppText>
          <View style={styles.actions}>
            {request.status === 'YOLDA' ? (
              <View style={styles.action}>
                <Button title={t('pickup.markPreparing')} variant="secondary" loading={preparing} onPress={onPrepare} />
              </View>
            ) : null}
            {isAwaitingHandOver(request.status) ? (
              <View style={styles.action}>
                <Button
                  title={t('pickup.handOver')}
                  onPress={() => router.push({ pathname: '/verify-pickup/[id]', params: { id: request.id } })}
                />
              </View>
            ) : null}
          </View>
        </>
      )}
    </Card>
  );
}

const styles = StyleSheet.create({
  header: { flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center', gap: spacing.sm },
  actions: { flexDirection: 'row', gap: spacing.sm },
  action: { flex: 1 },
});
