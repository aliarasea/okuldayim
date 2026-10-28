import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { StyleSheet, View } from 'react-native';

import { api } from '@/core/api';
import { errorMessage } from '@/core/api/errorMessage';
import type { ChildConsents, ChildSummary, ConsentDecision, ConsentType } from '@/core/api/types';
import { AppText } from '@/ui/AppText';
import { Banner } from '@/ui/Banner';
import { Button } from '@/ui/Button';
import { ErrorState, LoadingState } from '@/ui/StateViews';
import { spacing } from '@/ui/theme';

import { ConsentCard } from './ConsentCard';

export const consentsQueryKey = (childId: string) => ['rizalar', childId] as const;

interface ConsentEditorProps {
  child: ChildSummary;
  submitLabel: string;
  onSaved?: () => Promise<unknown> | void;
}

/** Çocuk bazında ayrıştırılmış açık rızalar (KVKK-02…04). Çocuk değişince `key` ile yeniden kurulmalı. */
export function ConsentEditor({ child, submitLabel, onSaved }: ConsentEditorProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const consents = useQuery({ queryKey: consentsQueryKey(child.id), queryFn: () => api.privacy.consents(child.id) });
  const [draft, setDraft] = useState<Partial<Record<ConsentType, ConsentDecision>>>({});
  const [saved, setSaved] = useState(false);

  const save = useMutation({
    mutationFn: ({ textVersion, decisions }: { textVersion: string; decisions: { type: ConsentType; decision: ConsentDecision }[] }) =>
      api.privacy.saveConsents(child.id, textVersion, decisions),
    onSuccess: async (data: ChildConsents) => {
      queryClient.setQueryData(consentsQueryKey(child.id), data);
      setDraft({});
      setSaved(true);
      await onSaved?.();
    },
  });

  if (consents.isPending) return <LoadingState />;
  if (consents.isError) return <ErrorState error={consents.error} onRetry={() => void consents.refetch()} />;

  const { items, textVersion } = consents.data;
  const changes = items.flatMap((item) => {
    const decision = draft[item.type];
    return decision && decision !== item.myDecision ? [{ type: item.type, decision }] : [];
  });

  return (
    <View style={styles.container}>
      <AppText>{t('consent.intro', { name: child.firstName })}</AppText>
      {items.map((item) => {
        const decision = draft[item.type] ?? item.myDecision;
        return (
          <ConsentCard
            key={item.type}
            item={item}
            decision={decision}
            dirty={decision !== item.myDecision}
            onChange={(value) => {
              setSaved(false);
              setDraft((previous) => ({ ...previous, [item.type]: value }));
            }}
          />
        );
      })}
      {save.isError ? <Banner tone="danger">{errorMessage(save.error)}</Banner> : null}
      {saved ? <Banner tone="success">{t('consent.saved')}</Banner> : null}
      <Button title={submitLabel} loading={save.isPending} onPress={() => save.mutate({ textVersion, decisions: changes })} />
      <AppText variant="caption" muted>
        {t('consent.textVersion', { version: textVersion })}
      </AppText>
    </View>
  );
}

const styles = StyleSheet.create({
  container: { gap: spacing.lg },
});
