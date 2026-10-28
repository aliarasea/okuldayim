import { Ionicons } from '@expo/vector-icons';
import type { ReactNode } from 'react';
import { StyleSheet, View } from 'react-native';

import { AppText } from './AppText';
import { radius, spacing, type Tone, toneColors, useColors } from './theme';

const ICONS: Record<Tone, keyof typeof Ionicons.glyphMap> = {
  info: 'information-circle',
  success: 'checkmark-circle',
  warning: 'warning',
  danger: 'alert-circle',
};

export function Banner({ tone = 'info', title, children }: { tone?: Tone; title?: string; children: ReactNode }) {
  const colors = useColors();
  const { fg, bg } = toneColors(colors, tone);
  return (
    <View
      accessibilityRole={tone === 'danger' || tone === 'warning' ? 'alert' : undefined}
      style={[styles.banner, { backgroundColor: bg }]}
    >
      <Ionicons name={ICONS[tone]} size={20} color={fg} accessibilityElementsHidden importantForAccessibility="no" />
      <View style={styles.body}>
        {title ? (
          <AppText variant="label" style={{ color: fg }}>
            {title}
          </AppText>
        ) : null}
        <AppText variant="caption" style={{ color: fg }}>
          {children}
        </AppText>
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  banner: { flexDirection: 'row', gap: spacing.sm, padding: spacing.md, borderRadius: radius.md, alignItems: 'flex-start' },
  body: { flex: 1, gap: spacing.xs },
});
