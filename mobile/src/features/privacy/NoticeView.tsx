import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Pressable, StyleSheet, View } from 'react-native';

import type { PrivacyNotice } from '@/core/api/types';
import { formatShortDate } from '@/core/date';
import { localeFor } from '@/core/i18n';
import { AppText } from '@/ui/AppText';
import { Card } from '@/ui/Card';
import { minTouchTarget, spacing } from '@/ui/theme';

/** KVKK-01: katmanlı aydınlatma; önce özet, istenirse tam metin. */
export function NoticeView({ notice }: { notice: PrivacyNotice }) {
  const { t, i18n } = useTranslation();
  const [expanded, setExpanded] = useState(false);

  return (
    <Card>
      <AppText variant="heading">{t('notice.heading')}</AppText>
      <AppText variant="caption" muted>
        {t('notice.dataController', { name: notice.kurumName })}
        {' · '}
        {t('notice.version', { version: notice.version, date: formatShortDate(notice.publishedAt, localeFor(i18n.language)) })}
      </AppText>
      {notice.summary.map((line) => (
        <View key={line} style={styles.bullet}>
          <AppText>•</AppText>
          <AppText style={styles.bulletText}>{line}</AppText>
        </View>
      ))}
      <Pressable
        accessibilityRole="button"
        accessibilityState={{ expanded }}
        onPress={() => setExpanded((value) => !value)}
        style={styles.toggle}
      >
        <AppText variant="label" tone="info">
          {expanded ? t('notice.hideFull') : t('notice.showFull')}
        </AppText>
      </Pressable>
      {expanded ? <AppText>{notice.fullText}</AppText> : null}
    </Card>
  );
}

const styles = StyleSheet.create({
  bullet: { flexDirection: 'row', gap: spacing.sm },
  bulletText: { flex: 1 },
  toggle: { minHeight: minTouchTarget, justifyContent: 'center' },
});
