// Backend ile paylaşılan veri sözleşmesi (REST, /v1). Alan adları backend DTO'larıyla aynıdır.

export type Role = 'KURUM_YONETICISI' | 'OGRETMEN' | 'VELI' | 'YETKILI_TESLIM_ALAN';

export interface OtpChallenge {
  challengeId: string;
  expiresInSec: number;
  retryAfterSec: number;
}

export interface TokenPair {
  accessToken: string;
  refreshToken: string;
  expiresInSec: number;
}

export interface ChildSummary {
  id: string;
  firstName: string;
  lastInitial: string;
  classId: string;
  className: string;
}

export interface Membership {
  id: string;
  kurumId: string;
  kurumName: string;
  role: Role;
  classIds: string[];
  /** Yalnızca VELI üyeliklerinde dolu. */
  children: ChildSummary[];
}

export interface Me {
  user: { id: string; displayName: string };
  memberships: Membership[];
  /** Aydınlatma metninin güncel sürümünü henüz okumadığı kurumlar (KVKK-01). */
  pendingNoticeKurumIds: string[];
  /** Rıza metni sürümü için rıza ekranını henüz görmediği çocuklar (KVKK-03). */
  consentReviewChildIds: string[];
}

export interface PrivacyNotice {
  kurumId: string;
  kurumName: string;
  version: string;
  publishedAt: string;
  summary: string[];
  fullText: string;
  acknowledged: boolean;
}

export type ConsentType = 'SAGLIK' | 'FOTO_VELI' | 'FOTO_GRUP' | 'FOTO_TANITIM' | 'KONUM' | 'AKIS_YETKILI';
export type ConsentDecision = 'GRANTED' | 'DENIED';

export interface ConsentItem {
  type: ConsentType;
  /** Bu velinin kararı; hiç karar vermediyse null. */
  myDecision: ConsentDecision | null;
  /** Diğer velilerden biri açıkça reddetti veya rızasını geri aldı. */
  otherGuardianDenied: boolean;
  /** Çocuk için uygulanan sonuç (en kısıtlayıcı rıza, KVKK-04). */
  effective: boolean;
  decidedAt: string | null;
}

export interface ChildConsents {
  childId: string;
  textVersion: string;
  items: ConsentItem[];
}

export type MealSlot = 'KAHVALTI' | 'ARA_OGUN' | 'OGLE' | 'IKINDI';
export type MealAmount = 'HIC' | 'AZ' | 'YARISI' | 'COGU' | 'TAMAMI';

export type FeedEntry =
  | { id: string; kind: 'MEAL'; at: string; slot: MealSlot; amount: MealAmount }
  | { id: string; kind: 'NAP'; at: string; endAt: string | null }
  | { id: string; kind: 'ACTIVITY'; at: string; activityType: string; note: string | null }
  | { id: string; kind: 'PICKUP'; at: string; pickupPersonName: string; relation: string };

export interface DailyFeed {
  childId: string;
  date: string;
  entries: FeedEntry[];
}

export interface ClassChild {
  id: string;
  firstName: string;
  lastInitial: string;
  /** null: velinin sağlık rızası yok, öğretmen alerji bilgisini göremez (MEAL-05 çalışmaz). */
  allergens: string[] | null;
}

export interface MenuMeal {
  slot: MealSlot;
  items: string[];
  allergens: string[];
}

export interface ClassToday {
  classId: string;
  className: string;
  date: string;
  children: ClassChild[];
  menu: MenuMeal[];
}

export interface MealEntryPayload {
  date: string;
  slot: MealSlot;
  entries: { childId: string; amount: MealAmount }[];
}

export interface AuthorizedPerson {
  id: string;
  displayName: string;
  relation: string;
  /** Kişinin aydınlatma bağlantısından fotoğrafını yükleyip onaylaması (KVKK-07). */
  verified: boolean;
  validUntil: string | null;
}

export type PickupStatus = 'YOLDA' | 'HAZIRLANIYOR' | 'TESLIM_EDILDI' | 'ENGELLENDI';

export interface PickupRequest {
  id: string;
  childId: string;
  childName: string;
  pickupPerson: { id: string; displayName: string; relation: string };
  etaMinutes: number;
  createdAt: string;
  status: PickupStatus;
  /** Tek kullanımlık doğrulama kodu; yalnızca talebi oluşturan veliye döner (PICK-06). */
  pin: string | null;
}

export type KvkkRequestType = 'BILGI' | 'DUZELTME' | 'SILME' | 'ITIRAZ' | 'HESAP_SILME';

export interface KvkkRequest {
  id: string;
  kurumId: string;
  type: KvkkRequestType;
  createdAt: string;
  /** KVKK m.13: başvuru en geç 30 gün içinde sonuçlandırılır. */
  dueAt: string;
  status: 'ACIK' | 'YANITLANDI';
}

export interface DataExportRequest {
  requestId: string;
  linkValidHours: number;
}
