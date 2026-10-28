import { useMutation } from '@tanstack/react-query';
import { Redirect, useRouter } from 'expo-router';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { KeyboardAvoidingView, Platform, ScrollView, StyleSheet, TextInput, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';

import { api } from '@/core/api';
import { errorMessage } from '@/core/api/errorMessage';
import { ApiError } from '@/core/api/errors';
import { getDeviceId } from '@/core/auth/secureStorage';
import { completeSignIn } from '@/core/session/sessionService';
import { otpFlowActions, useOtpFlow } from '@/features/auth/otpFlow';
import { maskPhone } from '@/features/auth/phone';
import { AppText } from '@/ui/AppText';
import { Banner } from '@/ui/Banner';
import { Button } from '@/ui/Button';
import { minTouchTarget, radius, spacing, useColors } from '@/ui/theme';

const CODE_LENGTH = 6;

export default function OtpScreen() {
  const { t } = useTranslation();
  const router = useRouter();
  const insets = useSafeAreaInsets();
  const colors = useColors();
  const flow = useOtpFlow();
  const [code, setCode] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [cooldown, setCooldown] = useState(flow.retryAfterSec);

  useEffect(() => {
    if (cooldown <= 0) return;
    const timer = setTimeout(() => setCooldown((seconds) => seconds - 1), 1000);
    return () => clearTimeout(timer);
  }, [cooldown]);

  const verify = useMutation({
    mutationFn: async (value: string) => {
      if (!flow.challengeId) throw new ApiError('otp_expired', 400);
      const tokens = await api.auth.verifyOtp(flow.challengeId, value, await getDeviceId());
      await completeSignIn(tokens);
    },
    // Giriş tamamlanınca telefon numarası bellekten de silinir; yönlendirmeyi koruma kuralı yapar.
    onSuccess: () => otpFlowActions.clear(),
    onError: (failure) => {
      setCode('');
      setError(errorMessage(failure));
    },
  });

  const resend = useMutation({
    mutationFn: (phone: string) => api.auth.requestOtp(phone),
    onSuccess: (challenge, phone) => {
      otpFlowActions.start(phone, challenge);
      setCooldown(challenge.retryAfterSec);
      setError(null);
    },
    onError: (failure) => setError(errorMessage(failure)),
  });

  if (!flow.phone) return verify.isSuccess ? null : <Redirect href="/login" />;
  const phone = flow.phone;

  const onChange = (value: string) => {
    const digits = value.replace(/\D/g, '').slice(0, CODE_LENGTH);
    setCode(digits);
    setError(null);
    if (digits.length === CODE_LENGTH && !verify.isPending) verify.mutate(digits);
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
          <AppText variant="title">{t('auth.otpTitle')}</AppText>
          <AppText muted>{t('auth.otpSubtitle', { phone: maskPhone(phone) })}</AppText>
          <TextInput
            accessibilityLabel={t('auth.otpLabel')}
            value={code}
            onChangeText={onChange}
            keyboardType="number-pad"
            textContentType="oneTimeCode"
            autoComplete="sms-otp"
            maxLength={CODE_LENGTH}
            autoFocus
            style={[styles.code, { color: colors.text, borderColor: colors.border, backgroundColor: colors.surface }]}
          />
          {error ? <Banner tone="danger">{error}</Banner> : null}
          <Button
            title={t('auth.verify')}
            disabled={code.length !== CODE_LENGTH}
            loading={verify.isPending}
            onPress={() => verify.mutate(code)}
          />
          {cooldown > 0 ? (
            <AppText variant="caption" muted>
              {t('auth.resendIn', { seconds: cooldown })}
            </AppText>
          ) : (
            <Button title={t('auth.resend')} variant="secondary" loading={resend.isPending} onPress={() => resend.mutate(phone)} />
          )}
          <Button title={t('auth.changeNumber')} variant="secondary" onPress={() => router.back()} />
        </View>
      </ScrollView>
    </KeyboardAvoidingView>
  );
}

const styles = StyleSheet.create({
  flex: { flex: 1 },
  content: { paddingHorizontal: spacing.xl, alignItems: 'center' },
  inner: { width: '100%', maxWidth: 480, gap: spacing.lg },
  code: {
    minHeight: minTouchTarget + 8,
    borderWidth: 1,
    borderRadius: radius.md,
    fontSize: 28,
    letterSpacing: 10,
    textAlign: 'center',
    fontVariant: ['tabular-nums'],
  },
});
