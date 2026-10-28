import { Ionicons } from '@expo/vector-icons';
import { useTranslation } from 'react-i18next';
import { StyleSheet, View } from 'react-native';

import type { ClassChild, MealAmount } from '@/core/api/types';
import { AppText } from '@/ui/AppText';
import { ChoiceChips } from '@/ui/ChoiceChips';
import { radius, spacing, useColors } from '@/ui/theme';

import { MEAL_AMOUNTS } from './mealEntry';

interface MealChildRowProps {
  child: ClassChild;
  amount: MealAmount | null;
  allergyMatches: string[];
  onChange: (amount: MealAmount) => void;
}

export function MealChildRow({ child, amount, allergyMatches, onChange }: MealChildRowProps) {
  const { t } = useTranslation();
  const colors = useColors();
  const name = `${child.firstName} ${child.lastInitial}.`;

  return (
    <View style={[styles.row, { backgroundColor: colors.surface, borderColor: allergyMatches.length ? colors.danger : colors.border }]}>
      <View style={styles.header}>
        <AppText variant="label">{name}</AppText>
        {allergyMatches.length > 0 ? (
          <View style={[styles.badge, { backgroundColor: colors.dangerSurface }]}>
            <Ionicons name="warning" size={14} color={colors.danger} importantForAccessibility="no" />
            <AppText variant="caption" tone="danger">
              {allergyMatches.join(', ')}
            </AppText>
          </View>
        ) : null}
      </View>
      {child.allergens === null ? (
        <AppText variant="caption" muted>
          {t('meals.noHealthConsent')}
        </AppText>
      ) : null}
      <ChoiceChips
        fill
        compact
        accessibilityLabel={name}
        value={amount}
        onChange={onChange}
        choices={MEAL_AMOUNTS.map((value) => ({
          value,
          label: t(`meals.amounts.${value}`),
          accessibilityLabel: t('meals.amountFor', { name, amount: t(`meals.amounts.${value}`) }),
        }))}
      />
    </View>
  );
}

const styles = StyleSheet.create({
  row: { borderWidth: 1, borderRadius: radius.lg, padding: spacing.md, gap: spacing.sm },
  header: { flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between', gap: spacing.sm },
  badge: { flexDirection: 'row', alignItems: 'center', gap: spacing.xs, paddingHorizontal: spacing.sm, paddingVertical: 2, borderRadius: radius.pill },
});
