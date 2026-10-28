import type { ClassChild, MealAmount, MealEntryPayload, MealSlot, MenuMeal } from '@/core/api/types';

export const MEAL_SLOTS: readonly MealSlot[] = ['KAHVALTI', 'ARA_OGUN', 'OGLE', 'IKINDI'];
/** MEAL-03: 5'li ölçek. */
export const MEAL_AMOUNTS: readonly MealAmount[] = ['HIC', 'AZ', 'YARISI', 'COGU', 'TAMAMI'];

export interface MealEntryState {
  slot: MealSlot;
  amounts: Readonly<Partial<Record<string, MealAmount>>>;
}

export type MealEntryAction =
  | { type: 'selectSlot'; slot: MealSlot }
  | { type: 'setAll'; childIds: readonly string[]; amount: MealAmount }
  | { type: 'set'; childId: string; amount: MealAmount }
  | { type: 'reset' };

export function defaultSlotFor(date: Date): MealSlot {
  const hour = date.getHours();
  if (hour < 10) return 'KAHVALTI';
  if (hour < 12) return 'ARA_OGUN';
  if (hour < 15) return 'OGLE';
  return 'IKINDI';
}

export function initialMealEntryState(date: Date): MealEntryState {
  return { slot: defaultSlotFor(date), amounts: {} };
}

/** MEAL-04: önce "herkes tamamını yedi", ardından istisnalar tek dokunuşla düzeltilir. */
export function mealEntryReducer(state: MealEntryState, action: MealEntryAction): MealEntryState {
  switch (action.type) {
    case 'selectSlot':
      return action.slot === state.slot ? state : { slot: action.slot, amounts: {} };
    case 'setAll':
      return { ...state, amounts: Object.fromEntries(action.childIds.map((childId) => [childId, action.amount])) };
    case 'set':
      return { ...state, amounts: { ...state.amounts, [action.childId]: action.amount } };
    case 'reset':
      return { ...state, amounts: {} };
  }
}

export function missingChildIds(state: MealEntryState, childIds: readonly string[]): string[] {
  return childIds.filter((childId) => !state.amounts[childId]);
}

export function toMealPayload(state: MealEntryState, date: string): MealEntryPayload | null {
  const entries = Object.entries(state.amounts).flatMap(([childId, amount]) => (amount ? [{ childId, amount }] : []));
  return entries.length > 0 ? { date, slot: state.slot, entries } : null;
}

/**
 * MEAL-05: menüdeki alerjenlerle çocuğun alerjilerinin kesişimi. Velinin sağlık rızası yoksa sunucu
 * alerji bilgisini göndermez (allergens = null) ve uyarı üretilemez.
 */
export function allergyMatches(child: Pick<ClassChild, 'allergens'>, meal: Pick<MenuMeal, 'allergens'> | undefined): string[] {
  if (!child.allergens || !meal) return [];
  const menu = new Set(meal.allergens.map(normalize));
  return child.allergens.filter((allergen) => menu.has(normalize(allergen)));
}

function normalize(value: string): string {
  return value.trim().toLocaleLowerCase('tr-TR');
}
