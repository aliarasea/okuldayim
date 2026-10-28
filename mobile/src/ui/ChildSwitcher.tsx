import { useTranslation } from 'react-i18next';

import type { ChildSummary } from '@/core/api/types';

import { ChoiceChips } from './ChoiceChips';

/** AUTH-03: birden fazla çocuğu olan veli için hızlı geçiş. Tek çocukta gösterilmez. */
export function ChildSwitcher({
  items,
  activeId,
  onChange,
}: {
  items: ChildSummary[];
  activeId: string | null;
  onChange: (childId: string) => void;
}) {
  const { t } = useTranslation();
  if (items.length < 2) return null;
  return (
    <ChoiceChips
      accessibilityLabel={t('common.childSwitcher')}
      choices={items.map((child) => ({ value: child.id, label: child.firstName }))}
      value={activeId}
      onChange={onChange}
    />
  );
}
