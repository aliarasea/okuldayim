import type { ReactNode } from 'react';
import { RefreshControl, ScrollView, StyleSheet, View } from 'react-native';

import { spacing, useColors } from './theme';

interface ScreenProps {
  children: ReactNode;
  refreshing?: boolean;
  onRefresh?: () => void;
}

/**
 * Kaydırılabilir ekran gövdesi. iOS 26'da başlıklar varsayılan olarak şeffaf olduğundan içerik
 * `contentInsetAdjustmentBehavior="automatic"` ile başlığın altına girmez.
 */
export function Screen({ children, refreshing = false, onRefresh }: ScreenProps) {
  const colors = useColors();
  return (
    <ScrollView
      contentInsetAdjustmentBehavior="automatic"
      keyboardShouldPersistTaps="handled"
      style={{ backgroundColor: colors.background }}
      contentContainerStyle={styles.content}
      refreshControl={onRefresh ? <RefreshControl refreshing={refreshing} onRefresh={onRefresh} /> : undefined}
    >
      <View style={styles.inner}>{children}</View>
    </ScrollView>
  );
}

const styles = StyleSheet.create({
  content: { padding: spacing.lg, paddingBottom: spacing.xxl, alignItems: 'center' },
  // Tablette satırların aşırı uzamaması için içerik genişliği sınırlandırılır.
  inner: { width: '100%', maxWidth: 720, gap: spacing.lg },
});
