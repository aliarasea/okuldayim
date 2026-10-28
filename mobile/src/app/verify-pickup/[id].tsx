import { Ionicons } from '@expo/vector-icons';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useLocalSearchParams, useRouter } from 'expo-router';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { StyleSheet, View } from 'react-native';

import { api } from '@/core/api';
import { errorMessage } from '@/core/api/errorMessage';
import { useSensitiveScreen } from '@/core/privacy/useSensitiveScreen';
import { useActiveMembership } from '@/core/session/sessionStore';
import { isAwaitingHandOver, pickupQueueKey } from '@/features/pickup/pickupRules';
import { AppText } from '@/ui/AppText';
import { Banner } from '@/ui/Banner';
import { Button } from '@/ui/Button';
import { Card } from '@/ui/Card';
import { Screen } from '@/ui/Screen';
import { EmptyState, ErrorState, LoadingState } from '@/ui/StateViews';
import { TextField } from '@/ui/TextField';
import { radius, useColors } from '@/ui/theme';

const PIN_LENGTH = 4;

/**
 * PICK-06/07: teslimde tek kullanımlık kod doğrulaması atlanamaz. Üç hatalı denemede sunucu teslimi
 * durdurur, veli ve yönetici uyarılır. Ekran görüntüsü engellenir (KVKK-15).
 */
export default function PickupVerifyScreen() {
  const { t } = useTranslation();
  const router = useRouter();
  const colors = useColors();
  const queryClient = useQueryClient();
  const { id = '' } = useLocalSearchParams<{ id?: string }>();
  const classId = useActiveMembership()?.classIds[0] ?? '';
  const [pin, setPin] = useState('');
  const [message, setMessage] = useState<string | null>(null);
  useSensitiveScreen('teslim-dogrulama');

  const queue = useQuery({
    queryKey: pickupQueueKey(classId),
    queryFn: () => api.pickup.queue(classId),
    enabled: classId !== '',
  });
  const request = queue.data?.find((item) => item.id === id);

  const handOver = useMutation({
    mutationFn: (code: string) => api.pickup.handOver(id, code),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: pickupQueueKey(classId) });
      router.back();
    },
    onError: (failure) => {
      setPin('');
      setMessage(errorMessage(failure));
      void queryClient.invalidateQueries({ queryKey: pickupQueueKey(classId) });
    },
  });

  if (queue.isPending) {
    return (
      <Screen>
        <LoadingState />
      </Screen>
    );
  }
  if (queue.isError) {
    return (
      <Screen>
        <ErrorState error={queue.error} onRetry={() => void queue.refetch()} />
      </Screen>
    );
  }
  if (!request) {
    return (
      <Screen>
        <EmptyState text={t('pickup.requestNotFound')} />
      </Screen>
    );
  }

  const canHandOver = isAwaitingHandOver(request.status);

  return (
    <Screen>
      <AppText muted>{t('pickup.verifyPersonHint')}</AppText>
      <Card style={styles.person}>
        {/* Gerçek sistemde kurum kaydındaki fotoğraf kısa ömürlü imzalı URL ile gösterilir ve cihazda saklanmaz. */}
        <View style={[styles.avatar, { backgroundColor: colors.chip }]}>
          <Ionicons name="person" size={56} color={colors.textMuted} importantForAccessibility="no" />
        </View>
        <AppText variant="title">{request.pickupPerson.displayName}</AppText>
        <AppText>
          {request.pickupPerson.relation} · {request.childName}
        </AppText>
        <AppText variant="caption" tone="success">
          {t('pickup.photoVerified')}
        </AppText>
      </Card>

      {request.status === 'ENGELLENDI' ? <Banner tone="danger">{t('errors.pickup_blocked')}</Banner> : null}
      <TextField
        label={t('pickup.pinLabel')}
        value={pin}
        onChangeText={(value) => {
          setMessage(null);
          setPin(value.replace(/\D/g, '').slice(0, PIN_LENGTH));
        }}
        keyboardType="number-pad"
        maxLength={PIN_LENGTH}
        autoComplete="off"
        textContentType="none"
        editable={canHandOver}
      />
      {message ? <Banner tone="danger">{message}</Banner> : null}
      <Button
        title={t('pickup.verifyAndHandOver')}
        disabled={!canHandOver || pin.length !== PIN_LENGTH}
        loading={handOver.isPending}
        onPress={() => handOver.mutate(pin)}
      />
      <Banner tone="warning">{t('pickup.notOnList')}</Banner>
    </Screen>
  );
}

const styles = StyleSheet.create({
  person: { alignItems: 'center' },
  avatar: { width: 112, height: 112, borderRadius: radius.pill, alignItems: 'center', justifyContent: 'center' },
});
