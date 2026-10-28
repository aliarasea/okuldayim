import { Pressable, StyleSheet, View } from 'react-native';

import { AppText } from './AppText';
import { minTouchTarget, radius, spacing, useColors } from './theme';

export interface Choice<T extends string> {
  value: T;
  label: string;
  /** Ekran okuyucu için daha açıklayıcı etiket. */
  accessibilityLabel?: string;
}

interface ChoiceChipsProps<T extends string> {
  choices: readonly Choice<T>[];
  value: T | null;
  onChange: (value: T) => void;
  accessibilityLabel: string;
  /** Satıra eşit genişlikte yayılır (öğün miktarı gibi). */
  fill?: boolean;
  compact?: boolean;
}

/** Tek seçimli düğme grubu. Hiçbir seçenek önceden seçili olmayabilir (value=null). */
export function ChoiceChips<T extends string>({ choices, value, onChange, accessibilityLabel, fill, compact }: ChoiceChipsProps<T>) {
  const colors = useColors();
  return (
    <View accessibilityRole="radiogroup" accessibilityLabel={accessibilityLabel} style={[styles.row, fill && styles.fillRow]}>
      {choices.map((choice) => {
        const selected = choice.value === value;
        return (
          <Pressable
            key={choice.value}
            accessibilityRole="radio"
            accessibilityLabel={choice.accessibilityLabel ?? choice.label}
            accessibilityState={{ checked: selected }}
            onPress={() => onChange(choice.value)}
            style={({ pressed }) => [
              styles.chip,
              compact && styles.compact,
              fill && styles.fill,
              {
                backgroundColor: selected ? colors.primary : colors.chip,
                borderColor: selected ? colors.primary : colors.border,
                opacity: pressed ? 0.85 : 1,
              },
            ]}
          >
            <AppText
              variant={compact ? 'caption' : 'label'}
              numberOfLines={1}
              adjustsFontSizeToFit
              style={{ color: selected ? colors.onPrimary : colors.text, fontWeight: '600' }}
            >
              {choice.label}
            </AppText>
          </Pressable>
        );
      })}
    </View>
  );
}

const styles = StyleSheet.create({
  row: { flexDirection: 'row', flexWrap: 'wrap', gap: spacing.sm },
  fillRow: { flexWrap: 'nowrap' },
  chip: {
    minHeight: minTouchTarget,
    paddingHorizontal: spacing.lg,
    borderRadius: radius.pill,
    borderWidth: 1,
    alignItems: 'center',
    justifyContent: 'center',
  },
  compact: { paddingHorizontal: spacing.sm, borderRadius: radius.sm },
  fill: { flex: 1 },
});
