import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

import { api } from '@/core/api';
import { errorMessage } from '@/core/api/errorMessage';
import type { KvkkRequestType } from '@/core/api/types';
import { formatShortDate } from '@/core/date';
import { localeFor } from '@/core/i18n';
import { useActiveMembership, useSessionStore } from '@/core/session/sessionStore';
import { AppText } from '@/ui/AppText';
import { Banner } from '@/ui/Banner';
import { Button } from '@/ui/Button';
import { Card } from '@/ui/Card';
import { ChoiceChips } from '@/ui/ChoiceChips';
import { ListRow } from '@/ui/ListRow';
import { Screen } from '@/ui/Screen';
import { EmptyState, ErrorState, LoadingState } from '@/ui/StateViews';
import { TextField } from '@/ui/TextField';

const REQUEST_TYPES: readonly KvkkRequestType[] = ['BILGI', 'DUZELTME', 'SILME', 'ITIRAZ', 'HESAP_SILME'];
const REQUESTS_KEY = ['kvkk-basvurulari'] as const;
const MAX_DESCRIPTION = 2000;

/** KVKK-05: m.11 başvurusu kuruma iletilir; 30 günlük yasal süre kurum panelinde izlenir. AUTH-04 dahil. */
export default function KvkkRequestScreen() {
  const { t, i18n } = useTranslation();
  const locale = localeFor(i18n.language);
  const queryClient = useQueryClient();
  const me = useSessionStore((state) => state.me);
  const activeKurumId = useActiveMembership()?.kurumId ?? null;
  const institutions = [...new Map((me?.memberships ?? []).map((m) => [m.kurumId, m.kurumName])).entries()];

  const [kurumId, setKurumId] = useState<string | null>(activeKurumId ?? institutions[0]?.[0] ?? null);
  const [type, setType] = useState<KvkkRequestType | null>(null);
  const [description, setDescription] = useState('');

  const history = useQuery({ queryKey: REQUESTS_KEY, queryFn: () => api.privacy.requests() });
  const submit = useMutation({
    mutationFn: (input: { kurumId: string; type: KvkkRequestType; description: string }) =>
      api.privacy.createRequest(input.kurumId, input.type, input.description),
    onSuccess: () => {
      setType(null);
      setDescription('');
      void queryClient.invalidateQueries({ queryKey: REQUESTS_KEY });
    },
  });

  return (
    <Screen>
      <AppText muted>{t('kvkk.intro')}</AppText>

      {institutions.length > 1 ? (
        <Card>
          <AppText variant="heading">{t('kvkk.institution')}</AppText>
          <ChoiceChips
            accessibilityLabel={t('kvkk.institution')}
            value={kurumId}
            onChange={setKurumId}
            choices={institutions.map(([id, name]) => ({ value: id, label: name }))}
          />
        </Card>
      ) : null}

      <Card>
        <AppText variant="heading">{t('kvkk.type')}</AppText>
        {REQUEST_TYPES.map((item) => (
          <ListRow
            key={item}
            title={t(`kvkk.types.${item}.title`)}
            subtitle={t(`kvkk.types.${item}.description`)}
            selected={type === item}
            onPress={() => setType(item)}
          />
        ))}
      </Card>
      {type === 'HESAP_SILME' ? <Banner tone="warning">{t('kvkk.accountDeletionInfo')}</Banner> : null}

      <TextField
        label={t('kvkk.description')}
        hint={t('kvkk.descriptionHint')}
        value={description}
        onChangeText={setDescription}
        multiline
        maxLength={MAX_DESCRIPTION}
      />
      {submit.isSuccess ? (
        <Banner tone="success">{t('kvkk.submitted', { date: formatShortDate(submit.data.dueAt, locale) })}</Banner>
      ) : null}
      {submit.isError ? <Banner tone="danger">{errorMessage(submit.error)}</Banner> : null}
      <Button
        title={t('kvkk.submit')}
        disabled={!type || !kurumId}
        loading={submit.isPending}
        onPress={() => kurumId && type && submit.mutate({ kurumId, type, description: description.trim() })}
      />

      <AppText variant="heading">{t('kvkk.history')}</AppText>
      {history.isPending ? (
        <LoadingState />
      ) : history.isError ? (
        <ErrorState error={history.error} onRetry={() => void history.refetch()} />
      ) : history.data.length === 0 ? (
        <EmptyState text={t('kvkk.historyEmpty')} />
      ) : (
        history.data.map((request) => (
          <Card key={request.id}>
            <AppText variant="label">{t(`kvkk.types.${request.type}.title`)}</AppText>
            <AppText variant="caption" muted>
              {t(`kvkk.status.${request.status}`)} · {t('kvkk.dueAt', { date: formatShortDate(request.dueAt, locale) })}
            </AppText>
          </Card>
        ))
      )}
    </Screen>
  );
}
