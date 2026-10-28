import { useRouter } from 'expo-router';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Alert } from 'react-native';

import { isLanguage } from '@/core/i18n';
import { useOutboxCounts } from '@/core/offline';
import { changeLanguage, signOut } from '@/core/session/sessionService';
import { sessionActions, useActiveMembership, useSessionStore } from '@/core/session/sessionStore';
import { AppText } from '@/ui/AppText';
import { Banner } from '@/ui/Banner';
import { Button } from '@/ui/Button';
import { Card } from '@/ui/Card';
import { ChoiceChips } from '@/ui/ChoiceChips';
import { ListRow } from '@/ui/ListRow';
import { Screen } from '@/ui/Screen';

export default function ProfileScreen() {
  const { t, i18n } = useTranslation();
  const router = useRouter();
  const me = useSessionStore((state) => state.me);
  const offline = useSessionStore((state) => state.offline);
  const membership = useActiveMembership();
  const { pending } = useOutboxCounts();
  const [signingOut, setSigningOut] = useState(false);

  if (!me) return null;

  const confirmSignOut = () => {
    Alert.alert(
      t('profile.signOutConfirmTitle'),
      pending > 0 ? t('profile.signOutPending', { count: pending }) : t('profile.signOutConfirmBody'),
      [
        { text: t('common.cancel'), style: 'cancel' },
        {
          text: t('profile.signOut'),
          style: 'destructive',
          onPress: () => {
            setSigningOut(true);
            void signOut();
          },
        },
      ],
    );
  };

  return (
    <Screen>
      {offline ? <Banner tone="warning">{t('common.offlineSession')}</Banner> : null}
      <Card>
        <AppText variant="title">{me.user.displayName}</AppText>
        {membership ? (
          <AppText muted>
            {t(`roles.${membership.role}`)} · {membership.kurumName}
          </AppText>
        ) : null}
      </Card>

      {me.memberships.length > 1 ? (
        <Card>
          <AppText variant="heading">{t('profile.switchRole')}</AppText>
          {me.memberships.map((item) => (
            <ListRow
              key={item.id}
              title={t(`roles.${item.role}`)}
              subtitle={item.kurumName}
              selected={item.id === membership?.id}
              onPress={() => {
                sessionActions.setActiveMembership(item.id);
                router.replace('/');
              }}
            />
          ))}
        </Card>
      ) : null}

      {membership?.role === 'KURUM_YONETICISI' ? <Banner>{t('profile.adminHint')}</Banner> : null}

      {membership && membership.children.length > 0 ? (
        <Card>
          <AppText variant="heading">{t('profile.children')}</AppText>
          {membership.children.map((child) => (
            <ListRow key={child.id} title={`${child.firstName} ${child.lastInitial}.`} subtitle={child.className} />
          ))}
        </Card>
      ) : null}

      <Card>
        <ListRow icon="shield-checkmark-outline" title={t('profile.privacyCenter')} onPress={() => router.push('/profile/privacy')} />
        <ListRow icon="document-text-outline" title={t('profile.kvkkRequest')} onPress={() => router.push('/profile/requests')} />
      </Card>

      <Card>
        <AppText variant="heading">{t('profile.language')}</AppText>
        <ChoiceChips
          accessibilityLabel={t('profile.language')}
          value={isLanguage(i18n.language) ? i18n.language : 'tr'}
          onChange={(language) => void changeLanguage(language)}
          choices={[
            { value: 'tr', label: 'Türkçe' },
            { value: 'en', label: 'English' },
          ]}
        />
      </Card>

      <Button title={t('profile.signOut')} variant="danger" loading={signingOut} onPress={confirmSignOut} />
    </Screen>
  );
}
