import { useTranslation } from 'react-i18next';

import type { ConsentDecision, ConsentItem } from '@/core/api/types';
import { consentStatus } from '@/core/consent/consent';
import { AppText } from '@/ui/AppText';
import { Banner } from '@/ui/Banner';
import { Card } from '@/ui/Card';
import { ChoiceChips } from '@/ui/ChoiceChips';

interface ConsentCardProps {
  item: ConsentItem;
  decision: ConsentDecision | null;
  dirty: boolean;
  onChange: (decision: ConsentDecision) => void;
}

/**
 * Tek bir rıza türü. "Onaylıyorum / Onaylamıyorum" seçeneklerinden hiçbiri önceden seçili gelmez;
 * karar verilmemiş rıza verilmemiş sayılır (KVKK-02).
 */
export function ConsentCard({ item, decision, dirty, onChange }: ConsentCardProps) {
  const { t } = useTranslation();
  const status = consentStatus(item);
  const title = t(`consent.types.${item.type}.title`);

  return (
    <Card>
      <AppText variant="heading">{title}</AppText>
      <AppText muted>{t(`consent.types.${item.type}.description`)}</AppText>
      {item.type === 'SAGLIK' && status !== 'effective' ? <Banner tone="warning">{t('consent.healthWarning')}</Banner> : null}
      <ChoiceChips
        fill
        accessibilityLabel={title}
        value={decision}
        onChange={onChange}
        choices={[
          { value: 'GRANTED', label: t('consent.grant') },
          { value: 'DENIED', label: t('consent.deny') },
        ]}
      />
      <AppText
        variant="caption"
        tone={status === 'effective' ? 'success' : status === 'blockedByOtherGuardian' ? 'warning' : undefined}
        muted={status === 'deniedByMe' || status === 'notGiven'}
      >
        {t(`consent.status.${status}`)}
        {dirty ? ` · ${t('common.unsavedChanges')}` : ''}
      </AppText>
    </Card>
  );
}
