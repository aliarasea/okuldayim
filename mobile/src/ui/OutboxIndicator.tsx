import { Ionicons } from '@expo/vector-icons';
import { useTranslation } from 'react-i18next';
import { StyleSheet, View } from 'react-native';

import { useOutboxCounts } from '@/core/offline';

import { AppText } from './AppText';
import { spacing, useColors } from './theme';

/** Başlıkta gönderilmeyi bekleyen çevrimdışı kayıt sayısı. */
export function OutboxIndicator() {
  const { t } = useTranslation();
  const colors = useColors();
  const { pending, failed } = useOutboxCounts();
  if (pending === 0 && failed === 0) return null;
  const label =
    failed > 0 ? t('outbox.failed', { count: failed }) : t('outbox.pending', { count: pending });
  return (
    <View accessible accessibilityLabel={label} style={styles.row}>
      <Ionicons
        name={failed > 0 ? 'cloud-offline' : 'cloud-upload'}
        size={20}
        color={failed > 0 ? colors.danger : colors.warning}
      />
      <AppText variant="caption" style={{ color: failed > 0 ? colors.danger : colors.warning }}>
        {failed > 0 ? failed : pending}
      </AppText>
    </View>
  );
}

const styles = StyleSheet.create({
  row: { flexDirection: 'row', alignItems: 'center', gap: spacing.xs, paddingHorizontal: spacing.md },
});
