import { StyleSheet, Text, type TextProps } from 'react-native';

import { type Tone, toneColors, useColors } from './theme';

type Variant = 'title' | 'heading' | 'body' | 'label' | 'caption';

interface AppTextProps extends TextProps {
  variant?: Variant;
  muted?: boolean;
  tone?: Tone;
}

// Yazı boyutları sistem ayarına göre ölçeklenir (dinamik yazı boyutu); allowFontScaling kapatılmaz.
export function AppText({ variant = 'body', muted, tone, style, ...rest }: AppTextProps) {
  const colors = useColors();
  const color = tone ? toneColors(colors, tone).fg : muted ? colors.textMuted : colors.text;
  return (
    <Text
      accessibilityRole={variant === 'title' || variant === 'heading' ? 'header' : undefined}
      style={[styles[variant], { color }, style]}
      {...rest}
    />
  );
}

const styles = StyleSheet.create({
  title: { fontSize: 26, fontWeight: '700', lineHeight: 32 },
  heading: { fontSize: 18, fontWeight: '600', lineHeight: 24 },
  body: { fontSize: 16, lineHeight: 22 },
  label: { fontSize: 15, fontWeight: '600', lineHeight: 20 },
  caption: { fontSize: 13, lineHeight: 18 },
});
