import { useMutation, useQuery } from '@tanstack/react-query';
import { useReducer, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { FlatList, StyleSheet, View } from 'react-native';

import { api } from '@/core/api';
import { outboxPaths } from '@/core/api/endpoints';
import { errorMessage } from '@/core/api/errorMessage';
import type { ClassToday, MealEntryPayload } from '@/core/api/types';
import { enqueue, syncOutbox } from '@/core/offline';
import { useActiveMembership } from '@/core/session/sessionStore';
import { MealChildRow } from '@/features/meals/MealChildRow';
import {
  allergyMatches,
  initialMealEntryState,
  MEAL_SLOTS,
  mealEntryReducer,
  missingChildIds,
  toMealPayload,
} from '@/features/meals/mealEntry';
import { AppText } from '@/ui/AppText';
import { Banner } from '@/ui/Banner';
import { Button } from '@/ui/Button';
import { Card } from '@/ui/Card';
import { ChoiceChips } from '@/ui/ChoiceChips';
import { Screen } from '@/ui/Screen';
import { EmptyState, ErrorState, LoadingState } from '@/ui/StateViews';
import { spacing, useColors } from '@/ui/theme';

type SaveOutcome = { kind: 'online' } | { kind: 'queued' } | { kind: 'failed'; count: number };

/** MEAL-02…05: sınıfa toplu öğün girişi; hedef 20 çocuk için ≤ 30 sn. */
export default function MealsScreen() {
  const { t } = useTranslation();
  const classId = useActiveMembership()?.classIds[0];

  if (!classId) {
    return (
      <Screen>
        <EmptyState text={t('meals.noClass')} />
      </Screen>
    );
  }
  return <ClassMeals key={classId} classId={classId} />;
}

function ClassMeals({ classId }: { classId: string }) {
  const classroom = useQuery({ queryKey: ['sinif', classId, 'bugun'], queryFn: () => api.classroom.today(classId) });

  if (classroom.isPending) {
    return (
      <Screen>
        <LoadingState />
      </Screen>
    );
  }
  if (classroom.isError) {
    return (
      <Screen>
        <ErrorState error={classroom.error} onRetry={() => void classroom.refetch()} />
      </Screen>
    );
  }
  return <MealEntryList classroom={classroom.data} />;
}

function MealEntryList({ classroom }: { classroom: ClassToday }) {
  const { t } = useTranslation();
  const colors = useColors();
  const [state, dispatch] = useReducer(mealEntryReducer, new Date(), initialMealEntryState);
  const [outcome, setOutcome] = useState<SaveOutcome | null>(null);

  // Kayıt önce şifreli yerel kuyruğa yazılır, ardından gönderilir: bağlantı yoksa kaybolmaz.
  const save = useMutation({
    mutationFn: async (payload: MealEntryPayload): Promise<SaveOutcome> => {
      await enqueue('POST', outboxPaths.mealEntries(classroom.classId), payload);
      const result = await syncOutbox(true);
      if (result && result.failed > 0) return { kind: 'failed', count: result.failed };
      return result && result.remaining === 0 ? { kind: 'online' } : { kind: 'queued' };
    },
    onSuccess: (result) => {
      setOutcome(result);
      if (result.kind !== 'failed') dispatch({ type: 'reset' });
    },
  });

  const childIds = classroom.children.map((child) => child.id);
  const meal = classroom.menu.find((item) => item.slot === state.slot);
  const payload = toMealPayload(state, classroom.date);
  const missing = missingChildIds(state, childIds).length;
  const warnings = classroom.children.flatMap((child) => {
    const matches = allergyMatches(child, meal);
    return matches.length > 0 ? [t('meals.allergyMatch', { name: child.firstName, allergens: matches.join(', ') })] : [];
  });

  const change = (action: Parameters<typeof dispatch>[0]) => {
    setOutcome(null);
    dispatch(action);
  };

  return (
    <FlatList
      contentInsetAdjustmentBehavior="automatic"
      style={{ backgroundColor: colors.background }}
      contentContainerStyle={styles.content}
      data={classroom.children}
      keyExtractor={(child) => child.id}
      ItemSeparatorComponent={Separator}
      ListHeaderComponent={
        <View style={styles.section}>
          <AppText variant="heading">{classroom.className}</AppText>
          <ChoiceChips
            accessibilityLabel={t('meals.slotPicker')}
            value={state.slot}
            onChange={(slot) => change({ type: 'selectSlot', slot })}
            choices={MEAL_SLOTS.map((slot) => ({ value: slot, label: t(`meals.slots.${slot}`) }))}
          />
          {meal ? (
            <Card>
              <AppText variant="label">{t('meals.menu')}</AppText>
              <AppText>{meal.items.join(', ')}</AppText>
              {meal.allergens.length > 0 ? (
                <AppText variant="caption" muted>
                  {t('meals.allergens', { list: meal.allergens.join(', ') })}
                </AppText>
              ) : null}
            </Card>
          ) : null}
          {warnings.length > 0 ? (
            <Banner tone="danger" title={t('meals.allergyTitle')}>
              {warnings.join('\n')}
            </Banner>
          ) : null}
          <Button
            title={t('meals.everyoneAteAll')}
            variant="secondary"
            onPress={() => change({ type: 'setAll', childIds, amount: 'TAMAMI' })}
          />
        </View>
      }
      renderItem={({ item }) => (
        <MealChildRow
          child={item}
          amount={state.amounts[item.id] ?? null}
          allergyMatches={allergyMatches(item, meal)}
          onChange={(amount) => change({ type: 'set', childId: item.id, amount })}
        />
      )}
      ListFooterComponent={
        <View style={styles.section}>
          {missing > 0 ? (
            <AppText variant="caption" muted>
              {t('meals.missing', { count: missing })}
            </AppText>
          ) : null}
          {outcome?.kind === 'online' ? <Banner tone="success">{t('meals.savedOnline')}</Banner> : null}
          {outcome?.kind === 'queued' ? <Banner tone="warning">{t('meals.savedOffline')}</Banner> : null}
          {outcome?.kind === 'failed' ? <Banner tone="danger">{t('outbox.failed', { count: outcome.count })}</Banner> : null}
          {save.isError ? <Banner tone="danger">{errorMessage(save.error)}</Banner> : null}
          <Button
            title={t('common.save')}
            disabled={!payload}
            loading={save.isPending}
            onPress={() => payload && save.mutate(payload)}
          />
        </View>
      }
    />
  );
}

function Separator() {
  return <View style={styles.separator} />;
}

const styles = StyleSheet.create({
  content: { padding: spacing.lg, paddingBottom: spacing.xxl, width: '100%', maxWidth: 720, alignSelf: 'center' },
  section: { gap: spacing.md, paddingVertical: spacing.md },
  separator: { height: spacing.sm },
});
