import {
  allergyMatches,
  defaultSlotFor,
  initialMealEntryState,
  mealEntryReducer,
  missingChildIds,
  toMealPayload,
} from '../mealEntry';

const CHILDREN = ['ada', 'can', 'ela'];

describe('mealEntryReducer (MEAL-04)', () => {
  it('"herkes tamamını yedi" sonrası istisnalar tek tek düzeltilir', () => {
    let state = initialMealEntryState(new Date(2026, 9, 28, 12, 30));
    expect(state.slot).toBe('OGLE');

    state = mealEntryReducer(state, { type: 'setAll', childIds: CHILDREN, amount: 'TAMAMI' });
    state = mealEntryReducer(state, { type: 'set', childId: 'can', amount: 'AZ' });

    expect(toMealPayload(state, '2026-10-28')).toEqual({
      date: '2026-10-28',
      slot: 'OGLE',
      entries: [
        { childId: 'ada', amount: 'TAMAMI' },
        { childId: 'can', amount: 'AZ' },
        { childId: 'ela', amount: 'TAMAMI' },
      ],
    });
  });

  it('öğün değişince girişler sıfırlanır', () => {
    let state = mealEntryReducer(initialMealEntryState(new Date(2026, 9, 28, 8)), {
      type: 'set',
      childId: 'ada',
      amount: 'COGU',
    });
    state = mealEntryReducer(state, { type: 'selectSlot', slot: 'ARA_OGUN' });
    expect(state).toEqual({ slot: 'ARA_OGUN', amounts: {} });
    expect(toMealPayload(state, '2026-10-28')).toBeNull();
  });

  it('giriş yapılmamış çocukları listeler', () => {
    const state = mealEntryReducer(initialMealEntryState(new Date()), { type: 'set', childId: 'ela', amount: 'HIC' });
    expect(missingChildIds(state, CHILDREN)).toEqual(['ada', 'can']);
  });
});

describe('defaultSlotFor', () => {
  it.each([
    [8, 'KAHVALTI'],
    [11, 'ARA_OGUN'],
    [13, 'OGLE'],
    [16, 'IKINDI'],
  ])('%i:00 → %s', (hour, slot) => {
    expect(defaultSlotFor(new Date(2026, 9, 28, hour))).toBe(slot);
  });
});

describe('allergyMatches (MEAL-05)', () => {
  const meal = { allergens: ['Fıstık', 'gluten'] };

  it('büyük/küçük harf ve Türkçe karakterden bağımsız eşleşir', () => {
    expect(allergyMatches({ allergens: ['fıstık', 'süt'] }, meal)).toEqual(['fıstık']);
    expect(allergyMatches({ allergens: ['GLUTEN'] }, meal)).toEqual(['GLUTEN']);
  });

  it('sağlık rızası yoksa (allergens = null) uyarı üretmez', () => {
    expect(allergyMatches({ allergens: null }, meal)).toEqual([]);
  });

  it('menü yoksa uyarı üretmez', () => {
    expect(allergyMatches({ allergens: ['fıstık'] }, undefined)).toEqual([]);
  });
});
