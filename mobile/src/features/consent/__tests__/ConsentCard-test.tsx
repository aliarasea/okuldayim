import { fireEvent, render, screen } from '@testing-library/react-native';

import type { ConsentItem } from '@/core/api/types';
import i18n from '@/core/i18n';

import { ConsentCard } from '../ConsentCard';

const item = (overrides: Partial<ConsentItem>): ConsentItem => ({
  type: 'FOTO_GRUP',
  myDecision: null,
  otherGuardianDenied: false,
  effective: false,
  decidedAt: null,
  ...overrides,
});

describe('ConsentCard', () => {
  beforeAll(async () => {
    await i18n.changeLanguage('tr');
  });

  it('hiçbir seçenek önceden seçili gelmez (varsayılan: rıza yok)', async () => {
    await render(<ConsentCard item={item({})} decision={null} dirty={false} onChange={jest.fn()} />);

    expect(screen.getByRole('radio', { name: 'Onaylıyorum' })).not.toBeChecked();
    expect(screen.getByRole('radio', { name: 'Onaylamıyorum' })).not.toBeChecked();
    expect(screen.getByText('Rıza verilmedi')).toBeOnTheScreen();
  });

  it('diğer velinin reddini açıkça gösterir ve seçimi bildirir', async () => {
    const onChange = jest.fn();
    await render(
      <ConsentCard item={item({ myDecision: 'GRANTED', otherGuardianDenied: true })} decision="GRANTED" dirty={false} onChange={onChange} />,
    );

    expect(screen.getByRole('radio', { name: 'Onaylıyorum' })).toBeChecked();
    expect(screen.getByText('Diğer veli onaylamadığı için uygulanmıyor')).toBeOnTheScreen();

    await fireEvent.press(screen.getByRole('radio', { name: 'Onaylamıyorum' }));
    expect(onChange).toHaveBeenCalledWith('DENIED');
  });

  it('sağlık rızası yoksa alerji uyarısının çalışmayacağını belirtir', async () => {
    await render(<ConsentCard item={item({ type: 'SAGLIK' })} decision={null} dirty={false} onChange={jest.fn()} />);
    expect(screen.getByText('Rıza vermezseniz menüdeki alerjen uyarısı çocuğunuz için çalışmaz.')).toBeOnTheScreen();
  });
});
