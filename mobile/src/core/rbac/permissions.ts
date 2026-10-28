import type { Role } from '../api/types';

/**
 * Gereksinim analizi bölüm 2'deki yetki matrisi.
 * Yalnızca arayüzü şekillendirmek içindir; yetkilendirme ve kapsam kontrolü sunucuda zorunludur.
 *
 * FULL: kurum genelinde · OWN: yalnızca kendi sınıfı/çocuğu · READ: okuma · NONE: yetki yok
 */
export type Grant = 'FULL' | 'OWN' | 'READ' | 'NONE';

export type Action =
  | 'child.register'
  | 'daily.record'
  | 'feed.view'
  | 'media.upload'
  | 'announcement.institution'
  | 'announcement.class'
  | 'message.send'
  | 'pickup.notify'
  | 'pickup.confirm'
  | 'authorizedPerson.manage'
  | 'health.edit'
  | 'health.view'
  | 'report.export'
  | 'consent.manage'
  | 'access.restrict'
  | 'audit.view'
  | 'kvkkRequest.create'
  | 'kvkkRequest.respond';

type Row = Readonly<Record<Role, Grant>>;

const row = (KURUM_YONETICISI: Grant, OGRETMEN: Grant, VELI: Grant, YETKILI_TESLIM_ALAN: Grant): Row => ({
  KURUM_YONETICISI,
  OGRETMEN,
  VELI,
  YETKILI_TESLIM_ALAN,
});

const MATRIX: Readonly<Record<Action, Row>> = {
  'child.register': row('FULL', 'NONE', 'NONE', 'NONE'),
  'daily.record': row('FULL', 'OWN', 'NONE', 'NONE'),
  // Yetkili teslim alan yalnızca velinin AKIS_YETKILI rızasıyla, sağlık ve fotoğraf hariç okur.
  'feed.view': row('FULL', 'OWN', 'OWN', 'READ'),
  'media.upload': row('FULL', 'OWN', 'NONE', 'NONE'),
  'announcement.institution': row('FULL', 'NONE', 'NONE', 'NONE'),
  'announcement.class': row('FULL', 'OWN', 'NONE', 'NONE'),
  'message.send': row('FULL', 'OWN', 'OWN', 'NONE'),
  'pickup.notify': row('NONE', 'NONE', 'OWN', 'OWN'),
  'pickup.confirm': row('FULL', 'OWN', 'NONE', 'NONE'),
  'authorizedPerson.manage': row('FULL', 'NONE', 'OWN', 'NONE'),
  'health.edit': row('FULL', 'NONE', 'OWN', 'NONE'),
  'health.view': row('FULL', 'READ', 'OWN', 'NONE'),
  'report.export': row('FULL', 'OWN', 'OWN', 'NONE'),
  'consent.manage': row('READ', 'NONE', 'OWN', 'NONE'),
  'access.restrict': row('FULL', 'NONE', 'NONE', 'NONE'),
  'audit.view': row('FULL', 'NONE', 'READ', 'NONE'),
  'kvkkRequest.create': row('NONE', 'OWN', 'OWN', 'OWN'),
  'kvkkRequest.respond': row('FULL', 'NONE', 'NONE', 'NONE'),
};

export function grantFor(role: Role, action: Action): Grant {
  return MATRIX[action][role];
}

export function can(role: Role | null | undefined, action: Action): boolean {
  return role ? grantFor(role, action) !== 'NONE' : false;
}
