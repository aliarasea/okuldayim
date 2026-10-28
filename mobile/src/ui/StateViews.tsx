import { useTranslation } from 'react-i18next';
import { ActivityIndicator, StyleSheet, View } from 'react-native';

import { errorMessage } from '@/core/api/errorMessage';

import { AppText } from './AppText';
import { Banner } from './Banner';
import { Button } from './Button';
import { spacing, useColors } from './theme';

export function LoadingState() {
  const colors = useColors();
  const { t } = useTranslation();
  return (
    <View style={styles.center} accessible accessibilityLabel={t('common.loading')}>
      <ActivityIndicator color={colors.primary} />
    </View>
  );
}

export function ErrorState({ error, onRetry }: { error: unknown; onRetry?: () => void }) {
  const { t } = useTranslation();
  return (
    <View style={styles.block}>
      <Banner tone="danger">{errorMessage(error)}</Banner>
      {onRetry ? <Button title={t('common.retry')} variant="secondary" onPress={onRetry} /> : null}
    </View>
  );
}

export function EmptyState({ text }: { text: string }) {
  return (
    <View style={styles.center}>
      <AppText muted style={styles.text}>
        {text}
      </AppText>
    </View>
  );
}

const styles = StyleSheet.create({
  center: { paddingVertical: spacing.xxl, alignItems: 'center', justifyContent: 'center' },
  block: { gap: spacing.md },
  text: { textAlign: 'center' },
});
