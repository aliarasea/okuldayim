import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { StyleSheet, View } from 'react-native';

import { api } from '@/core/api';
import { errorMessage } from '@/core/api/errorMessage';
import type { ChildSummary, PickupRequest, PickupStatus } from '@/core/api/types';
import { minutesRemaining } from '@/core/date';
import { useSensitiveScreen } from '@/core/privacy/useSensitiveScreen';
import { sessionActions } from '@/core/session/sessionStore';
import { AppText } from '@/ui/AppText';
import { Banner } from '@/ui/Banner';
import { Button } from '@/ui/Button';
import { Card } from '@/ui/Card';
import { ChildSwitcher } from '@/ui/ChildSwitcher';
import { ChoiceChips } from '@/ui/ChoiceChips';
import { ListRow } from '@/ui/ListRow';
import { Screen } from '@/ui/Screen';
import { ErrorState, LoadingState } from '@/ui/StateViews';
import { radius, spacing, useColors } from '@/ui/theme';

import { activePickupKey, ETA_OPTIONS, type EtaOption, isAwaitingHandOver, unavailableReason } from './pickupRules';

const STEPS: readonly PickupStatus[] = ['YOLDA', 'HAZIRLANIYOR', 'TESLIM_EDILDI'];

/** PICK-01…04: velinin "yola çıktım" bildirimi ve canlı durum takibi. */
export function ParentPickup({ child, siblings }: { child: ChildSummary; siblings: ChildSummary[] }) {
  const queryClient = useQueryClient();
  // WebSocket kanalı backend ile birlikte gelene kadar durum 10 sn'de bir sorgulanır.
  const active = useQuery({
    queryKey: activePickupKey(child.id),
    queryFn: () => api.pickup.activeRequest(child.id),
    refetchInterval: 10_000,
  });

  return (
    <Screen refreshing={active.isRefetching} onRefresh={() => void active.refetch()}>
      <ChildSwitcher items={siblings} activeId={child.id} onChange={sessionActions.setActiveChild} />
      {active.isPending ? (
        <LoadingState />
      ) : active.isError ? (
        <ErrorState error={active.error} onRetry={() => void active.refetch()} />
      ) : active.data ? (
        <ActivePickup request={active.data} />
      ) : (
        <PickupRequestForm
          child={child}
          onCreated={(request) => queryClient.setQueryData(activePickupKey(child.id), request)}
        />
      )}
    </Screen>
  );
}

function PickupRequestForm({ child, onCreated }: { child: ChildSummary; onCreated: (request: PickupRequest) => void }) {
  const { t } = useTranslation();
  const persons = useQuery({ queryKey: ['yetkili', child.id], queryFn: () => api.pickup.authorizedPersons(child.id) });
  const [personId, setPersonId] = useState<string | null>(null);
  const [eta, setEta] = useState<EtaOption>('10');
  const create = useMutation({
    mutationFn: (selectedPersonId: string) => api.pickup.create(child.id, selectedPersonId, Number(eta)),
    onSuccess: onCreated,
  });

  if (persons.isPending) return <LoadingState />;
  if (persons.isError) return <ErrorState error={persons.error} onRetry={() => void persons.refetch()} />;

  return (
    <>
      <Card>
        <AppText variant="heading">{t('pickup.whoPicksUp')}</AppText>
        <View accessibilityRole="radiogroup" accessibilityLabel={t('pickup.whoPicksUp')}>
          {persons.data.map((person) => {
            const reason = unavailableReason(person);
            return (
              <ListRow
                key={person.id}
                title={person.displayName}
                subtitle={reason ? `${person.relation} · ${t(reason)}` : person.relation}
                selected={personId === person.id}
                disabled={reason !== null}
                onPress={() => setPersonId(person.id)}
              />
            );
          })}
        </View>
      </Card>
      <Card>
        <AppText variant="heading">{t('pickup.eta')}</AppText>
        <ChoiceChips
          accessibilityLabel={t('pickup.eta')}
          value={eta}
          onChange={setEta}
          choices={ETA_OPTIONS.map((value) => ({ value, label: t('pickup.etaOption', { minutes: value }) }))}
        />
      </Card>
      {create.isError ? <Banner tone="danger">{errorMessage(create.error)}</Banner> : null}
      <Button
        title={t('pickup.onMyWay')}
        disabled={!personId}
        loading={create.isPending}
        onPress={() => personId && create.mutate(personId)}
      />
    </>
  );
}

function ActivePickup({ request }: { request: PickupRequest }) {
  const { t } = useTranslation();
  const colors = useColors();
  const currentStep = STEPS.indexOf(request.status);

  return (
    <Card>
      <AppText variant="heading">{request.childName}</AppText>
      <AppText>{t('pickup.person', { name: request.pickupPerson.displayName, relation: request.pickupPerson.relation })}</AppText>
      {request.status === 'ENGELLENDI' ? (
        <Banner tone="danger">{t('pickup.blocked')}</Banner>
      ) : (
        <View style={styles.steps} accessibilityRole="progressbar" accessibilityLabel={t(`pickup.status.${request.status}`)}>
          {STEPS.map((step, index) => {
            const reached = index <= currentStep;
            return (
              <View key={step} style={[styles.step, { backgroundColor: reached ? colors.primary : colors.chip }]}>
                <AppText variant="caption" style={{ color: reached ? colors.onPrimary : colors.textMuted, fontWeight: '600' }}>
                  {t(`pickup.status.${step}`)}
                </AppText>
              </View>
            );
          })}
        </View>
      )}
      {isAwaitingHandOver(request.status) ? (
        <AppText muted>{t('pickup.arrivesIn', { count: minutesRemaining(request.createdAt, request.etaMinutes) })}</AppText>
      ) : null}
      {request.pin && isAwaitingHandOver(request.status) ? <PickupPin pin={request.pin} /> : null}
    </Card>
  );
}

/** PICK-06: tek kullanımlık teslim kodu. Ekran görüntüsü engellenir (KVKK-15). */
function PickupPin({ pin }: { pin: string }) {
  const { t } = useTranslation();
  const colors = useColors();
  useSensitiveScreen('teslim-kodu');

  return (
    <View style={[styles.pin, { backgroundColor: colors.infoSurface }]}>
      <AppText variant="label">{t('pickup.pinTitle')}</AppText>
      <AppText style={[styles.pinDigits, { color: colors.text }]} accessibilityLabel={pin.split('').join(' ')}>
        {pin}
      </AppText>
      <AppText variant="caption" muted style={styles.center}>
        {t('pickup.pinHint')}
      </AppText>
    </View>
  );
}

const styles = StyleSheet.create({
  steps: { flexDirection: 'row', gap: spacing.xs },
  step: { flex: 1, paddingVertical: spacing.sm, borderRadius: radius.sm, alignItems: 'center' },
  pin: { borderRadius: radius.md, padding: spacing.lg, alignItems: 'center', gap: spacing.sm },
  pinDigits: { fontSize: 40, fontWeight: '700', letterSpacing: 12, fontVariant: ['tabular-nums'] },
  center: { textAlign: 'center' },
});
