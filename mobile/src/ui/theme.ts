import { useColorScheme } from 'react-native';

// Kontrast oranları WCAG 2.1 AA hedefine göre seçildi (metin/arka plan ≥ 4.5:1).
const light = {
  background: '#F6F7F9',
  surface: '#FFFFFF',
  text: '#14181F',
  textMuted: '#525B68',
  border: '#D5DAE1',
  primary: '#2449C9',
  onPrimary: '#FFFFFF',
  chip: '#ECEFF4',
  danger: '#B3261E',
  dangerSurface: '#FCEBEA',
  warning: '#7A4F00',
  warningSurface: '#FFF3DC',
  success: '#17703A',
  successSurface: '#E5F5EA',
  info: '#1F3F9E',
  infoSurface: '#E8EEFD',
};

const dark: typeof light = {
  background: '#101418',
  surface: '#1A1F26',
  text: '#F1F3F6',
  textMuted: '#A9B2BF',
  border: '#313945',
  primary: '#8FAEFF',
  onPrimary: '#0B1020',
  chip: '#252C35',
  danger: '#FF9C94',
  dangerSurface: '#3B1E1C',
  warning: '#FFCF7A',
  warningSurface: '#3A2E14',
  success: '#86D4A0',
  successSurface: '#163723',
  info: '#A9C1FF',
  infoSurface: '#1C2A4A',
};

export type Colors = typeof light;
export type Tone = 'info' | 'success' | 'warning' | 'danger';

export const spacing = { xs: 4, sm: 8, md: 12, lg: 16, xl: 24, xxl: 32 } as const;
export const radius = { sm: 8, md: 12, lg: 16, pill: 999 } as const;
/** Dokunma hedefi alt sınırı (WCAG 2.5.5 / platform yönergeleri). */
export const minTouchTarget = 48;

export function useColors(): Colors {
  return useColorScheme() === 'dark' ? dark : light;
}

export function toneColors(colors: Colors, tone: Tone): { fg: string; bg: string } {
  switch (tone) {
    case 'success':
      return { fg: colors.success, bg: colors.successSurface };
    case 'warning':
      return { fg: colors.warning, bg: colors.warningSurface };
    case 'danger':
      return { fg: colors.danger, bg: colors.dangerSurface };
    default:
      return { fg: colors.info, bg: colors.infoSurface };
  }
}
