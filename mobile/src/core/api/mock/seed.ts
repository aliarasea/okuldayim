// Geliştirme ve demo için sentetik veri. Gerçek kişi, kurum veya çocuk verisi içermez.
import type { ConsentDecision, FeedEntry, MenuMeal, Role } from '../types';

export const DEMO_OTP = '123456';
export const CONSENT_TEXT_VERSION = '2026-10';
export const NOTICE_VERSION = '2026-10';

export interface MockMembership {
  id: string;
  kurumId: string;
  role: Role;
  classIds: string[];
  childIds: string[];
}

export interface MockUser {
  id: string;
  /** null: demo girişi olmayan veli (ör. diğer ebeveyn). */
  phone: string | null;
  displayName: string;
  memberships: MockMembership[];
}

export interface MockChild {
  id: string;
  firstName: string;
  lastInitial: string;
  classId: string;
  allergens: string[];
  guardianIds: string[];
}

export interface MockClass {
  id: string;
  name: string;
}

export interface MockAuthorizedPerson {
  id: string;
  displayName: string;
  relation: string;
  verified: boolean;
  validUntil: string | null;
}

export interface MockConsent {
  decision: ConsentDecision;
  textVersion: string;
  at: string;
}

export const KURUM = {
  id: 'k-papatya',
  name: 'Papatya Anaokulu (Demo)',
} as const;

export const NOTICE = {
  version: NOTICE_VERSION,
  publishedAt: '2026-10-01T09:00:00.000Z',
  summary: [
    'Veri sorumlusu Papatya Anaokulu (Demo)dur. Uygulama sağlayıcısı, kurum adına veri işleyen olarak hizmet verir.',
    'Çocuğunuzun kimlik, sınıf, günlük bakım, yoklama ve teslim kayıtları eğitim ve bakım hizmetinin yürütülmesi için işlenir.',
    'Sağlık bilgisi, fotoğraf, konum ve yetkili kişinin akışı görmesi yalnızca ayrıca vereceğiniz açık rızayla işlenir. Rıza vermemeniz uygulamayı kullanmanızı engellemez.',
    'Veriler Türkiye’deki sunucularda saklanır. Bildirim metinlerinde çocuğunuzun adı veya sağlık bilgisi yer almaz.',
    'Saklama süresi dolan veriler silinir veya anonim hale getirilir.',
    'KVKK m.11 kapsamındaki haklarınız için uygulama içinden başvurabilirsiniz; başvurular en geç 30 gün içinde yanıtlanır.',
  ],
  fullText: [
    'Bu metin 6698 sayılı Kişisel Verilerin Korunması Kanunu’nun 10. maddesi uyarınca hazırlanmıştır.',
    'İşlenen veriler: veli ve yetkili kişilerin kimlik ve iletişim bilgileri; çocuğun kimlik, sınıf, günlük kayıt (aktivite, öğün, uyku, tuvalet), yoklama ve teslim bilgileri; açık rıza verilmesi halinde sağlık ve diyet bilgileri, fotoğraf ve videolar, teslim için konum olayı.',
    'Amaçlar ve hukuki sebepler: eğitim ve bakım hizmetinin yürütülmesi ve velinin bilgilendirilmesi (sözleşmenin ifası, m.5/2-c); çocuğun güvenli teslimi (meşru menfaat, m.5/2-f); veri güvenliğinin sağlanması (hukuki yükümlülük, m.5/2-ç); sağlık verisi, fotoğraf ve konum için açık rıza (m.5/1, m.6/3-a).',
    'Aktarım: veriler yalnızca kurum personeli, ilgili veliler ve Türkiye’de hizmet veren alt veri işleyenlerle (barındırma, SMS) paylaşılır. Push bildirim altyapısına kişisel veri içermeyen bildirimler gönderilir.',
    'Saklama: veriler kurumun saklama ve imha politikasında belirtilen süreler kadar saklanır; süre sonunda silinir veya anonimleştirilir.',
    'Haklarınız: KVKK m.11 uyarınca bilgi talep etme, düzeltme, silme, itiraz ve zararın giderilmesini isteme haklarına sahipsiniz. Başvurunuzu uygulamadaki KVKK başvurusu ekranından yapabilirsiniz.',
  ].join('\n\n'),
} as const;

export const CLASSES: MockClass[] = [
  { id: 'c-kelebek', name: 'Kelebekler (4 yaş)' },
  { id: 'c-ari', name: 'Arılar (5 yaş)' },
];

export const CHILDREN: MockChild[] = [
  { id: 'ch-ada', firstName: 'Ada', lastInitial: 'Y', classId: 'c-kelebek', allergens: ['fıstık'], guardianIds: ['u-selin', 'u-burak'] },
  { id: 'ch-ela', firstName: 'Ela', lastInitial: 'K', classId: 'c-kelebek', allergens: ['süt'], guardianIds: ['u-emre', 'u-asli'] },
  { id: 'ch-deniz', firstName: 'Deniz', lastInitial: 'A', classId: 'c-kelebek', allergens: [], guardianIds: ['u-g1'] },
  { id: 'ch-mert', firstName: 'Mert', lastInitial: 'Ş', classId: 'c-kelebek', allergens: ['yumurta'], guardianIds: ['u-g2'] },
  { id: 'ch-zeynep', firstName: 'Zeynep', lastInitial: 'B', classId: 'c-kelebek', allergens: ['gluten'], guardianIds: ['u-g3'] },
  { id: 'ch-can', firstName: 'Can', lastInitial: 'Y', classId: 'c-ari', allergens: [], guardianIds: ['u-selin', 'u-burak'] },
  { id: 'ch-efe', firstName: 'Efe', lastInitial: 'T', classId: 'c-ari', allergens: [], guardianIds: ['u-g4'] },
  { id: 'ch-lina', firstName: 'Lina', lastInitial: 'M', classId: 'c-ari', allergens: ['susam'], guardianIds: ['u-g5'] },
];

const veli = (id: string, childIds: string[]): MockMembership => ({ id, kurumId: KURUM.id, role: 'VELI', classIds: [], childIds });
const ogretmen = (id: string, classIds: string[]): MockMembership => ({
  id,
  kurumId: KURUM.id,
  role: 'OGRETMEN',
  classIds,
  childIds: [],
});

export const USERS: MockUser[] = [
  { id: 'u-selin', phone: '+905550000001', displayName: 'Selin Y.', memberships: [veli('m-selin', ['ch-ada', 'ch-can'])] },
  { id: 'u-ayse', phone: '+905550000002', displayName: 'Ayşe D.', memberships: [ogretmen('m-ayse', ['c-kelebek'])] },
  {
    id: 'u-emre',
    phone: '+905550000003',
    displayName: 'Emre K.',
    memberships: [ogretmen('m-emre-ogretmen', ['c-ari']), veli('m-emre-veli', ['ch-ela'])],
  },
  { id: 'u-burak', phone: null, displayName: 'Burak Y.', memberships: [veli('m-burak', ['ch-ada', 'ch-can'])] },
  { id: 'u-asli', phone: null, displayName: 'Aslı K.', memberships: [veli('m-asli', ['ch-ela'])] },
  ...['u-g1', 'u-g2', 'u-g3', 'u-g4', 'u-g5'].map((id, index) => ({
    id,
    phone: null,
    displayName: `Veli ${index + 1}`,
    memberships: [veli(`m-${id}`, CHILDREN.filter((child) => child.guardianIds.includes(id)).map((child) => child.id))],
  })),
];

/** `${childId}|${guardianId}|${ConsentType}` → karar */
export const CONSENTS: [string, ConsentDecision][] = [
  // Ada'nın babası sağlık rızası verdi ama grup fotoğrafına itiraz etti (en kısıtlayıcı rıza örneği).
  ['ch-ada|u-burak|SAGLIK', 'GRANTED'],
  ['ch-ada|u-burak|FOTO_VELI', 'GRANTED'],
  ['ch-ada|u-burak|FOTO_GRUP', 'DENIED'],
  ['ch-ela|u-asli|SAGLIK', 'GRANTED'],
  ['ch-zeynep|u-g3|SAGLIK', 'GRANTED'],
  ['ch-lina|u-g5|SAGLIK', 'GRANTED'],
  // Mert'in velisi sağlık rızası vermedi: öğretmen alerji bilgisini göremez.
];

export function authorizedPersons(now: Date): Record<string, MockAuthorizedPerson[]> {
  const yesterday = new Date(now.getTime() - 24 * 60 * 60_000).toISOString();
  return {
    'ch-ada': [
      { id: 'ap-ada-selin', displayName: 'Selin Y.', relation: 'Anne', verified: true, validUntil: null },
      { id: 'ap-ada-burak', displayName: 'Burak Y.', relation: 'Baba', verified: true, validUntil: null },
      { id: 'ap-ada-fatma', displayName: 'Fatma Y.', relation: 'Babaanne', verified: true, validUntil: null },
      { id: 'ap-ada-gul', displayName: 'Gül T.', relation: 'Bakıcı', verified: true, validUntil: yesterday },
      { id: 'ap-ada-murat', displayName: 'Murat A.', relation: 'Dayı', verified: false, validUntil: null },
    ],
    'ch-can': [
      { id: 'ap-can-selin', displayName: 'Selin Y.', relation: 'Anne', verified: true, validUntil: null },
      { id: 'ap-can-burak', displayName: 'Burak Y.', relation: 'Baba', verified: true, validUntil: null },
      { id: 'ap-can-fatma', displayName: 'Fatma Y.', relation: 'Babaanne', verified: true, validUntil: null },
    ],
    'ch-ela': [
      { id: 'ap-ela-emre', displayName: 'Emre K.', relation: 'Baba', verified: true, validUntil: null },
      { id: 'ap-ela-asli', displayName: 'Aslı K.', relation: 'Anne', verified: true, validUntil: null },
    ],
  };
}

export const MENU: MenuMeal[] = [
  { slot: 'KAHVALTI', items: ['Haşlanmış yumurta', 'Beyaz peynir', 'Domates', 'Tam buğday ekmeği'], allergens: ['yumurta', 'süt', 'gluten'] },
  { slot: 'ARA_OGUN', items: ['Elma', 'Fıstık ezmeli kraker'], allergens: ['fıstık', 'gluten'] },
  { slot: 'OGLE', items: ['Mercimek çorbası', 'Sebzeli bulgur pilavı', 'Ayran'], allergens: ['süt', 'gluten'] },
  { slot: 'IKINDI', items: ['Muz', 'Susamlı simit'], allergens: ['susam', 'gluten'] },
];

/** Bugünün akışı; şu andan sonraki kayıtlar eklenmez. */
export function initialFeed(now: Date): Record<string, FeedEntry[]> {
  const at = (hour: number, minute: number) =>
    new Date(now.getFullYear(), now.getMonth(), now.getDate(), hour, minute).toISOString();
  const feed: Record<string, FeedEntry[]> = {
    'ch-ada': [
      { id: 'f-ada-1', kind: 'MEAL', at: at(8, 45), slot: 'KAHVALTI', amount: 'TAMAMI' },
      { id: 'f-ada-2', kind: 'ACTIVITY', at: at(10, 0), activityType: 'Sanat', note: 'Parmak boyama ile sonbahar yaprakları' },
      { id: 'f-ada-3', kind: 'MEAL', at: at(12, 30), slot: 'OGLE', amount: 'YARISI' },
      { id: 'f-ada-4', kind: 'NAP', at: at(13, 15), endAt: at(14, 30) },
    ],
    'ch-can': [
      { id: 'f-can-1', kind: 'MEAL', at: at(9, 0), slot: 'KAHVALTI', amount: 'COGU' },
      { id: 'f-can-2', kind: 'ACTIVITY', at: at(11, 0), activityType: 'Açık hava', note: null },
    ],
    'ch-ela': [
      { id: 'f-ela-1', kind: 'MEAL', at: at(8, 50), slot: 'KAHVALTI', amount: 'AZ' },
      { id: 'f-ela-2', kind: 'NAP', at: at(13, 0), endAt: null },
    ],
  };
  const nowIso = now.toISOString();
  return Object.fromEntries(
    Object.entries(feed).map(([childId, entries]) => [childId, entries.filter((entry) => entry.at <= nowIso)]),
  );
}
