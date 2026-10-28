import { useMutation } from '@tanstack/react-query';
import { useRouter } from 'expo-router';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { KeyboardAvoidingView, Platform, ScrollView, StyleSheet, TextInput, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';

import { api } from '@/core/api';
import { errorMessage } from '@/core/api/errorMessage';
import { config } from '@/core/config';
import { useSessionStore } from '@/core/session/sessionStore';
import { otpFlowActions } from '@/features/auth/otpFlow';
import { formatNationalNumber, isValidNationalNumber, sanitizeNationalNumber, toE164 } from '@/features/auth/phone';
import { AppText } from '@/ui/AppText';
import { Banner } from '@/ui/Banner';
import { Button } from '@/ui/Button';
import { minTouchTarget, radius, spacing, useColors } from '@/ui/theme';

/** AUTH-01/02: davetli telefon numarası ile SMS OTP girişi. */
export default function PhoneScreen() {
  const { t } = useTranslation();
  const router = useRouter();
  const insets = useSafeAreaInsets();
  const colors = useColors();
  const sessionExpired = useSessionStore((state) => state.signedOutReason === 'expired');
  const [digits, setDigits] = useState('');

  const request = useMutation({
    mutationFn: (phone: string) => api.auth.requestOtp(phone),
    onSuccess: (challenge, phone) => {
      otpFlowActions.start(phone, challenge);
      router.push('/verify-otp');
    },
  });

  const valid = isValidNationalNumber(digits);
  const submit = () => {
    if (valid && !request.isPending) request.mutate(toE164(digits));
  };

  return (
    <KeyboardAvoidingView
      behavior={Platform.OS === 'ios' ? 'padding' : undefined}
      style={[styles.flex, { backgroundColor: colors.background }]}
    >
      <ScrollView
        keyboardShouldPersistTaps="handled"
        contentContainerStyle={[
          styles.content,
          { paddingTop: insets.top + spacing.xxl, paddingBottom: insets.bottom + spacing.xl },
        ]}
      >
        <View style={styles.inner}>
          <AppText variant="title">{t('auth.title')}</AppText>
          <AppText muted>{t('auth.subtitle')}</AppText>
          {sessionExpired ? <Banner tone="warning">{t('auth.sessionExpired')}</Banner> : null}

          <View style={styles.field}>
            <AppText variant="label">{t('auth.phoneLabel')}</AppText>
            <View style={[styles.phoneRow, { borderColor: colors.border, backgroundColor: colors.surface }]}>
              <AppText variant="label" muted>
                +90
              </AppText>
              <TextInput
                accessibilityLabel={t('auth.phoneLabel')}
                value={formatNationalNumber(digits)}
                onChangeText={(value) => setDigits(sanitizeNationalNumber(value))}
                keyboardType="phone-pad"
                textContentType="telephoneNumber"
                autoComplete="tel"
                placeholder={t('auth.phonePlaceholder')}
                placeholderTextColor={colors.textMuted}
                returnKeyType="done"
                onSubmitEditing={submit}
                style={[styles.phoneInput, { color: colors.text }]}
              />
            </View>
          </View>

          {request.isError ? <Banner tone="danger">{errorMessage(request.error)}</Banner> : null}
          <Button title={t('auth.sendCode')} disabled={!valid} loading={request.isPending} onPress={submit} />
          <AppText variant="caption" muted>
            {t('auth.privacyFootnote')}
          </AppText>
          {config.apiMode === 'mock' ? <Banner tone="info">{t('auth.demoHint')}</Banner> : null}
        </View>
      </ScrollView>
    </KeyboardAvoidingView>
  );
}

const styles = StyleSheet.create({
  flex: { flex: 1 },
  content: { paddingHorizontal: spacing.xl, alignItems: 'center' },
  inner: { width: '100%', maxWidth: 480, gap: spacing.lg },
  field: { gap: spacing.xs },
  phoneRow: {
    minHeight: minTouchTarget,
    flexDirection: 'row',
    alignItems: 'center',
    gap: spacing.sm,
    borderWidth: 1,
    borderRadius: radius.md,
    paddingHorizontal: spacing.md,
  },
  phoneInput: { flex: 1, fontSize: 18, minHeight: minTouchTarget },
});
