import { Ionicons } from '@expo/vector-icons';
import type { TFunction } from 'i18next';
import { useTranslation } from 'react-i18next';
import { StyleSheet, View } from 'react-native';

import type { FeedEntry } from '@/core/api/types';
import { formatTime } from '@/core/date';
import { localeFor } from '@/core/i18n';
import { AppText } from '@/ui/AppText';
import { Card } from '@/ui/Card';
import { spacing, useColors } from '@/ui/theme';

interface Described {
  icon: keyof typeof Ionicons.glyphMap;
  text: string;
  detail: string | null;
}

export function FeedEntryRow({ entry }: { entry: FeedEntry }) {
  const { t, i18n } = useTranslation();
  const colors = useColors();
  const locale = localeFor(i18n.language);
  const { icon, text, detail } = describe(entry, t, locale);

  return (
    <Card style={styles.card}>
      <Ionicons name={icon} size={24} color={colors.primary} importantForAccessibility="no" />
      <View style={styles.body}>
        <AppText variant="caption" muted>
          {formatTime(entry.at, locale)}
        </AppText>
        <AppText variant="label">{text}</AppText>
        {detail ? <AppText muted>{detail}</AppText> : null}
      </View>
    </Card>
  );
}

function describe(entry: FeedEntry, t: TFunction, locale: string): Described {
  switch (entry.kind) {
    case 'MEAL':
      return {
        icon: 'restaurant-outline',
        text: t('feed.meal', { slot: t(`meals.slots.${entry.slot}`), amount: t(`meals.amounts.${entry.amount}`) }),
        detail: null,
      };
    case 'NAP':
      return {
        icon: 'moon-outline',
        text: entry.endAt
          ? t('feed.nap', { start: formatTime(entry.at, locale), end: formatTime(entry.endAt, locale) })
          : t('feed.napOngoing', { start: formatTime(entry.at, locale) }),
        detail: null,
      };
    case 'ACTIVITY':
      return { icon: 'color-palette-outline', text: entry.activityType, detail: entry.note };
    case 'PICKUP':
      return {
        icon: 'home-outline',
        text: t('feed.pickup', { name: entry.pickupPersonName, relation: entry.relation }),
        detail: null,
      };
  }
}

const styles = StyleSheet.create({
  card: { flexDirection: 'row', alignItems: 'flex-start', gap: spacing.md },
  body: { flex: 1, gap: 2 },
});
