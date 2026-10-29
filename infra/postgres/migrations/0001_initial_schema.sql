-- Canonical Okuldayim application schema: PostgreSQL 18+, UUIDv7, tenant-scoped RLS.
-- The complete DDL and migration-ledger write run atomically through migrate.sh.
BEGIN;
SET LOCAL ROLE okuldayim_owner;
SET LOCAL lock_timeout = '2s';
SET LOCAL statement_timeout = '15s';
SET LOCAL search_path = app, pg_catalog;

ALTER DEFAULT PRIVILEGES FOR ROLE okuldayim_owner IN SCHEMA app
  REVOKE UPDATE, DELETE ON TABLES FROM okuldayim_app;
ALTER DEFAULT PRIVILEGES FOR ROLE okuldayim_owner IN SCHEMA app
  REVOKE EXECUTE ON FUNCTIONS FROM PUBLIC, okuldayim_app;

-- Organization, identity and access
CREATE TABLE app.organization (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  name text NOT NULL CHECK (length(btrim(name)) BETWEEN 1 AND 200),
  status text NOT NULL DEFAULT 'AKTIF' CHECK (status IN ('AKTIF', 'ASKIDA', 'KAPALI')),
  transit_key_name text NOT NULL UNIQUE,
  contract_end_date date,
  timezone text NOT NULL DEFAULT 'Europe/Istanbul',
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  updated_at timestamptz NOT NULL DEFAULT clock_timestamp()
);

CREATE TABLE app.branch (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  name text NOT NULL CHECK (length(btrim(name)) BETWEEN 1 AND 160),
  address text,
  is_active boolean NOT NULL DEFAULT true,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (organization_id, id)
);

CREATE TABLE app.school_year (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  name text NOT NULL CHECK (length(btrim(name)) BETWEEN 1 AND 80),
  start_date date NOT NULL,
  end_date date NOT NULL,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  CHECK (end_date > start_date),
  UNIQUE (organization_id, id),
  UNIQUE (organization_id, name)
);

CREATE TABLE app.classroom (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  branch_id uuid,
  school_year_id uuid NOT NULL,
  name text NOT NULL CHECK (length(btrim(name)) BETWEEN 1 AND 120),
  is_active boolean NOT NULL DEFAULT true,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (organization_id, id),
  FOREIGN KEY (organization_id, branch_id) REFERENCES app.branch(organization_id, id) ON DELETE RESTRICT,
  FOREIGN KEY (organization_id, school_year_id) REFERENCES app.school_year(organization_id, id) ON DELETE RESTRICT
);

CREATE TABLE app.user_account (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  first_name_ciphertext text,
  last_name_ciphertext text,
  status text NOT NULL DEFAULT 'AKTIF' CHECK (status IN ('AKTIF', 'KAPALI', 'SILINME_TALEBI', 'SILINMIS')),
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  deleted_at timestamptz
);

CREATE TABLE app.user_contact (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  type text NOT NULL CHECK (type IN ('TELEFON', 'EPOSTA')),
  value_ciphertext text NOT NULL,
  blind_index bytea NOT NULL CHECK (octet_length(blind_index) = 32),
  blind_index_version smallint NOT NULL CHECK (blind_index_version > 0),
  verified_at timestamptz,
  is_primary boolean NOT NULL DEFAULT false,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (id, user_id),
  UNIQUE (type, blind_index_version, blind_index)
);
CREATE UNIQUE INDEX user_contact_primary_per_type_uq
  ON app.user_contact (user_id, type) WHERE is_primary;
CREATE INDEX user_contact_user_idx ON app.user_contact (user_id);

CREATE TABLE app.mfa_record (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  type text NOT NULL CHECK (type IN ('TOTP')),
  secret_ciphertext text NOT NULL,
  is_enabled boolean NOT NULL DEFAULT false,
  enabled_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (user_id, type)
);

CREATE TABLE app.mfa_recovery_code (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  mfa_record_id uuid NOT NULL REFERENCES app.mfa_record(id) ON DELETE RESTRICT,
  code_hash bytea NOT NULL CHECK (octet_length(code_hash) = 32),
  used_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (mfa_record_id, code_hash)
);
CREATE INDEX mfa_recovery_unused_idx ON app.mfa_recovery_code (mfa_record_id) WHERE used_at IS NULL;

CREATE TABLE app.invitation (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  created_by_user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  target_type text NOT NULL CHECK (target_type IN ('TELEFON', 'EPOSTA')),
  target_ciphertext text NOT NULL,
  target_blind_index bytea NOT NULL CHECK (octet_length(target_blind_index) = 32),
  blind_index_version smallint NOT NULL CHECK (blind_index_version > 0),
  role text NOT NULL CHECK (role IN ('KURUM_YONETICISI', 'OGRETMEN', 'VELI', 'KVKK_IRTIBAT', 'YETKILI_TESLIM_ALAN')),
  token_hash bytea NOT NULL UNIQUE CHECK (octet_length(token_hash) = 32),
  status text NOT NULL DEFAULT 'BEKLIYOR' CHECK (status IN ('BEKLIYOR', 'TUKETILDI', 'IPTAL', 'SURESI_DOLDU')),
  expires_at timestamptz NOT NULL,
  consumed_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (organization_id, id),
  CHECK ((status = 'TUKETILDI') = (consumed_at IS NOT NULL))
);
CREATE INDEX invitation_organization_status_expiry_idx ON app.invitation (organization_id, status, expires_at);
CREATE INDEX invitation_target_lookup_idx
  ON app.invitation (organization_id, target_type, blind_index_version, target_blind_index)
  WHERE status = 'BEKLIYOR';

CREATE TABLE app.invitation_scope (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  invitation_id uuid NOT NULL,
  class_id uuid,
  child_id uuid,
  CHECK (num_nonnulls(class_id, child_id) = 1),
  FOREIGN KEY (organization_id, invitation_id) REFERENCES app.invitation(organization_id, id) ON DELETE RESTRICT,
  FOREIGN KEY (organization_id, class_id) REFERENCES app.classroom(organization_id, id) ON DELETE RESTRICT
);
CREATE INDEX invitation_scope_invitation_idx ON app.invitation_scope (organization_id, invitation_id);

CREATE TABLE app.membership (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  role text NOT NULL CHECK (role IN ('KURUM_YONETICISI', 'OGRETMEN', 'VELI', 'KVKK_IRTIBAT', 'PLATFORM_YONETICISI')),
  status text NOT NULL DEFAULT 'AKTIF' CHECK (status IN ('DAVETLI', 'AKTIF', 'ASKIDA', 'SONLANDI')),
  starts_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  ends_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (organization_id, id),
  CHECK (ends_at IS NULL OR ends_at >= starts_at)
);
CREATE UNIQUE INDEX membership_one_active_per_organization_user_uq
  ON app.membership (organization_id, user_id) WHERE status = 'AKTIF';
CREATE INDEX membership_user_status_organization_idx ON app.membership (user_id, status, organization_id);

CREATE TABLE app.membership_class (
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  membership_id uuid NOT NULL,
  class_id uuid NOT NULL,
  assigned_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  PRIMARY KEY (organization_id, membership_id, class_id),
  FOREIGN KEY (organization_id, membership_id) REFERENCES app.membership(organization_id, id) ON DELETE RESTRICT,
  FOREIGN KEY (organization_id, class_id) REFERENCES app.classroom(organization_id, id) ON DELETE RESTRICT
);
CREATE INDEX membership_class_class_idx ON app.membership_class (organization_id, class_id, membership_id);

CREATE TABLE app.child (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  class_id uuid NOT NULL,
  first_name_ciphertext text NOT NULL,
  last_name_ciphertext text NOT NULL,
  birth_date date,
  enrollment_date date,
  departure_date date,
  status text NOT NULL DEFAULT 'AKTIF' CHECK (status IN ('AKTIF', 'AYRILDI', 'ARSIV')),
  profile_media_id uuid,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (organization_id, id),
  FOREIGN KEY (organization_id, class_id) REFERENCES app.classroom(organization_id, id) ON DELETE RESTRICT,
  CHECK (departure_date IS NULL OR enrollment_date IS NULL OR departure_date >= enrollment_date),
  CHECK (birth_date IS NULL OR birth_date <= CURRENT_DATE)
);
ALTER TABLE app.invitation_scope
  ADD CONSTRAINT invitation_scope_child_fk
  FOREIGN KEY (organization_id, child_id) REFERENCES app.child(organization_id, id) ON DELETE RESTRICT;
CREATE TABLE app.guardian_child (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  guardian_user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  child_id uuid NOT NULL,
  relationship text NOT NULL CHECK (relationship IN ('ANNE', 'BABA', 'VASI', 'DIGER')),
  access_level text NOT NULL DEFAULT 'TAM' CHECK (access_level IN ('TAM', 'SALT_OKUNUR', 'KAPALI')),
  is_active boolean NOT NULL DEFAULT true,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (organization_id, id),
  UNIQUE (organization_id, child_id, guardian_user_id),
  FOREIGN KEY (organization_id, child_id) REFERENCES app.child(organization_id, id) ON DELETE RESTRICT
);
CREATE INDEX guardian_child_user_idx ON app.guardian_child (organization_id, guardian_user_id, is_active, child_id);

CREATE TABLE app.access_restriction (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  child_id uuid NOT NULL,
  guardian_user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  scope text NOT NULL CHECK (scope IN ('SALT_OKUNUR', 'KAPALI')),
  decision_date date NOT NULL,
  decision_number text NOT NULL CHECK (length(btrim(decision_number)) > 0),
  supporting_document_ref text,
  starts_at timestamptz NOT NULL,
  ends_at timestamptz,
  changed_by_user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  FOREIGN KEY (organization_id, child_id, guardian_user_id)
    REFERENCES app.guardian_child(organization_id, child_id, guardian_user_id) ON DELETE RESTRICT,
  CHECK (ends_at IS NULL OR ends_at > starts_at)
);
CREATE INDEX access_restriction_target_idx ON app.access_restriction (organization_id, child_id, guardian_user_id, starts_at DESC);

CREATE TABLE app.organization_settings (
  organization_id uuid PRIMARY KEY REFERENCES app.organization(id) ON DELETE RESTRICT,
  daily_feed_mode text NOT NULL DEFAULT 'MANUEL' CHECK (daily_feed_mode IN ('MANUEL', 'GUN_SONU')),
  end_of_day_time time NOT NULL DEFAULT '17:30',
  authorized_person_approval_required boolean NOT NULL DEFAULT false,
  updated_at timestamptz NOT NULL DEFAULT clock_timestamp()
);

CREATE TABLE app.organization_module (
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  module_code text NOT NULL CHECK (module_code IN ('AKIS', 'OGUN', 'TESLIM', 'YOKLAMA', 'MEDYA', 'MESAJ', 'TAKVIM')),
  is_enabled boolean NOT NULL DEFAULT true,
  PRIMARY KEY (organization_id, module_code)
);

CREATE TABLE app.activity_type (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  code text NOT NULL CHECK (code ~ '^[a-z][a-z0-9_]{1,39}$'),
  name text NOT NULL CHECK (length(btrim(name)) BETWEEN 1 AND 100),
  sort_order smallint NOT NULL DEFAULT 0,
  is_enabled boolean NOT NULL DEFAULT true,
  UNIQUE (organization_id, id),
  UNIQUE (organization_id, code)
);
CREATE INDEX activity_type_organization_order_idx ON app.activity_type (organization_id, is_enabled, sort_order);

-- Privacy, consent and health data
CREATE TABLE app.privacy_notice (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  audience text NOT NULL CHECK (audience IN ('VELI', 'YETKILI_KISI', 'PERSONEL')),
  version text NOT NULL CHECK (length(btrim(version)) BETWEEN 1 AND 40),
  title text NOT NULL,
  summary text NOT NULL,
  full_text text NOT NULL,
  content_sha256 bytea NOT NULL CHECK (octet_length(content_sha256) = 32),
  effective_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (organization_id, id),
  UNIQUE (organization_id, audience, version)
);

CREATE TABLE app.privacy_notice_read (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  notice_id uuid NOT NULL,
  user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  child_id uuid,
  channel text NOT NULL CHECK (channel IN ('MOBIL', 'WEB', 'DIGER')),
  device_ref text,
  read_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  FOREIGN KEY (organization_id, notice_id) REFERENCES app.privacy_notice(organization_id, id) ON DELETE RESTRICT,
  FOREIGN KEY (organization_id, child_id) REFERENCES app.child(organization_id, id) ON DELETE RESTRICT
);

CREATE TABLE app.consent_record (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  child_id uuid NOT NULL,
  guardian_user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  consent_type text NOT NULL CHECK (consent_type IN ('SAGLIK', 'FOTO_VELI', 'FOTO_GRUP', 'FOTO_TANITIM', 'KONUM', 'AKIS_YETKILI')),
  decision text NOT NULL CHECK (decision IN ('ONAY', 'RED')),
  notice_id uuid NOT NULL,
  channel text NOT NULL CHECK (channel IN ('MOBIL', 'WEB', 'DIGER')),
  device_ref text,
  decision_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  FOREIGN KEY (organization_id, child_id, guardian_user_id)
    REFERENCES app.guardian_child(organization_id, child_id, guardian_user_id) ON DELETE RESTRICT,
  FOREIGN KEY (organization_id, notice_id) REFERENCES app.privacy_notice(organization_id, id) ON DELETE RESTRICT
);

CREATE TABLE app.consent_review (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  child_id uuid NOT NULL,
  guardian_user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  notice_id uuid NOT NULL,
  reviewed_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  FOREIGN KEY (organization_id, child_id, guardian_user_id)
    REFERENCES app.guardian_child(organization_id, child_id, guardian_user_id) ON DELETE RESTRICT,
  FOREIGN KEY (organization_id, notice_id) REFERENCES app.privacy_notice(organization_id, id) ON DELETE RESTRICT
);

CREATE TABLE app.health_profile (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  child_id uuid NOT NULL,
  content_ciphertext text NOT NULL,
  key_version smallint NOT NULL CHECK (key_version > 0),
  updated_by_user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (organization_id, child_id),
  FOREIGN KEY (organization_id, child_id) REFERENCES app.child(organization_id, id) ON DELETE RESTRICT
);

CREATE VIEW app.effective_consent WITH (security_invoker = true) AS
WITH latest_decision AS (
  SELECT DISTINCT ON (r.organization_id, r.child_id, r.guardian_user_id, r.consent_type)
    r.organization_id, r.child_id, r.guardian_user_id, r.consent_type, r.decision, r.decision_at, r.id
  FROM app.consent_record AS r
  ORDER BY r.organization_id, r.child_id, r.guardian_user_id, r.consent_type, r.decision_at DESC, r.id DESC
)
SELECT organization_id, child_id, consent_type,
       bool_or(decision = 'ONAY') AND NOT bool_or(decision = 'RED') AS is_enabled,
       max(decision_at) AS latest_decision_at
FROM latest_decision
GROUP BY organization_id, child_id, consent_type;

-- Meals, daily records, attendance and pickup
CREATE TABLE app.meal_menu (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  start_date date NOT NULL,
  end_date date NOT NULL,
  status text NOT NULL DEFAULT 'TASLAK' CHECK (status IN ('TASLAK', 'YAYINDA', 'ARSIV')),
  created_by_user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  published_at timestamptz,
  UNIQUE (organization_id, id),
  CHECK (end_date >= start_date)
);
CREATE INDEX meal_menu_organization_date_idx ON app.meal_menu (organization_id, start_date DESC, end_date);

CREATE TABLE app.meal_definition (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  menu_id uuid NOT NULL,
  date date NOT NULL,
  type text NOT NULL CHECK (type IN ('KAHVALTI', 'OGLE', 'IKINDI', 'DIGER')),
  name text NOT NULL,
  description text,
  sort_order smallint NOT NULL DEFAULT 0,
  UNIQUE (organization_id, id),
  UNIQUE (organization_id, menu_id, date, type),
  FOREIGN KEY (organization_id, menu_id) REFERENCES app.meal_menu(organization_id, id) ON DELETE RESTRICT
);
CREATE INDEX meal_definition_menu_idx ON app.meal_definition (organization_id, menu_id, date);

CREATE TABLE app.meal_allergen (
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  meal_definition_id uuid NOT NULL,
  allergen_code text NOT NULL CHECK (allergen_code ~ '^[A-Z0-9_]{2,40}$'),
  PRIMARY KEY (organization_id, meal_definition_id, allergen_code),
  FOREIGN KEY (organization_id, meal_definition_id) REFERENCES app.meal_definition(organization_id, id) ON DELETE RESTRICT
);

CREATE TABLE app.daily_record (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  child_id uuid NOT NULL,
  record_type text NOT NULL CHECK (record_type IN ('OGUN', 'UYKU', 'AKTIVITE', 'TUVALET', 'ILAC', 'OLAY', 'TESLIM')),
  data_category text NOT NULL CHECK (data_category IN ('GUNLUK', 'ILAC_OLAY', 'TESLIM')),
  date date NOT NULL,
  occurred_at timestamptz NOT NULL,
  received_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  time_source text NOT NULL DEFAULT 'SUNUCU' CHECK (time_source IN ('SUNUCU', 'CIHAZ')),
  is_late boolean NOT NULL DEFAULT false,
  details_json jsonb,
  details_ciphertext text,
  created_by_user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  corrected_record_id uuid,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (organization_id, id),
  FOREIGN KEY (organization_id, child_id) REFERENCES app.child(organization_id, id) ON DELETE RESTRICT,
  CHECK (details_json IS NULL OR jsonb_typeof(details_json) = 'object'),
  CHECK (
    (record_type IN ('ILAC', 'OLAY') AND details_ciphertext IS NOT NULL AND details_json IS NULL)
    OR (record_type NOT IN ('ILAC', 'OLAY') AND details_ciphertext IS NULL)
  ),
  CHECK (record_type NOT IN ('ILAC', 'OLAY') OR data_category = 'ILAC_OLAY')
);
ALTER TABLE app.daily_record
  ADD CONSTRAINT daily_record_correction_fk
  FOREIGN KEY (organization_id, corrected_record_id) REFERENCES app.daily_record(organization_id, id) ON DELETE RESTRICT;
CREATE INDEX daily_record_child_feed_idx ON app.daily_record (organization_id, child_id, date DESC, occurred_at DESC, id DESC);
CREATE INDEX daily_record_author_idx ON app.daily_record (organization_id, created_by_user_id, created_at DESC);

CREATE TABLE app.medication_instruction (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  child_id uuid NOT NULL,
  instruction_ciphertext text NOT NULL,
  key_version smallint NOT NULL CHECK (key_version > 0),
  starts_at timestamptz NOT NULL,
  ends_at timestamptz,
  recorded_by_user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  recorded_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  FOREIGN KEY (organization_id, child_id) REFERENCES app.child(organization_id, id) ON DELETE RESTRICT,
  CHECK (ends_at IS NULL OR ends_at > starts_at)
);
CREATE INDEX medication_instruction_child_idx ON app.medication_instruction (organization_id, child_id, starts_at DESC);

CREATE TABLE app.attendance (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  child_id uuid NOT NULL,
  date date NOT NULL,
  check_in_at timestamptz,
  check_out_at timestamptz,
  status text NOT NULL CHECK (status IN ('VAR', 'GELMEDI', 'GEC_GELDI', 'ERKEN_CIKTI')),
  recorded_by_user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (organization_id, child_id, date),
  FOREIGN KEY (organization_id, child_id) REFERENCES app.child(organization_id, id) ON DELETE RESTRICT,
  CHECK (check_out_at IS NULL OR check_in_at IS NULL OR check_out_at >= check_in_at)
);
CREATE INDEX attendance_date_idx ON app.attendance (organization_id, date DESC, child_id);

CREATE TABLE app.absence_notice (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  child_id uuid NOT NULL,
  notice_date date NOT NULL,
  reason_code text CHECK (reason_code IN ('HASTALIK', 'IZIN', 'DIGER')),
  description_ciphertext text,
  reported_by_user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (organization_id, child_id, notice_date),
  FOREIGN KEY (organization_id, child_id) REFERENCES app.child(organization_id, id) ON DELETE RESTRICT
);
CREATE INDEX absence_notice_date_idx ON app.absence_notice (organization_id, notice_date DESC);

CREATE TABLE app.authorized_person (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  added_by_user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  first_name_ciphertext text NOT NULL,
  phone_ciphertext text NOT NULL,
  phone_blind_index bytea NOT NULL CHECK (octet_length(phone_blind_index) = 32),
  relationship text NOT NULL,
  is_verified boolean NOT NULL DEFAULT false,
  approved_at timestamptz,
  valid_from timestamptz NOT NULL,
  valid_until timestamptz NOT NULL,
  remaining_uses smallint CHECK (remaining_uses >= 0),
  photo_media_id uuid,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (organization_id, id),
  CHECK (valid_until > valid_from)
);
CREATE INDEX authorized_person_creator_idx ON app.authorized_person (organization_id, added_by_user_id, valid_until);

CREATE TABLE app.authorized_user_child (
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  authorized_person_id uuid NOT NULL,
  child_id uuid NOT NULL,
  added_by_guardian_user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  feed_access_allowed boolean NOT NULL DEFAULT false,
  PRIMARY KEY (organization_id, authorized_person_id, child_id),
  FOREIGN KEY (organization_id, authorized_person_id) REFERENCES app.authorized_person(organization_id, id) ON DELETE RESTRICT,
  FOREIGN KEY (organization_id, child_id, added_by_guardian_user_id)
    REFERENCES app.guardian_child(organization_id, child_id, guardian_user_id) ON DELETE RESTRICT
);
CREATE INDEX authorized_user_child_child_idx ON app.authorized_user_child (organization_id, child_id, authorized_person_id);

CREATE TABLE app.emergency_contact (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  child_id uuid NOT NULL,
  added_by_user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  first_name_ciphertext text NOT NULL,
  phone_ciphertext text NOT NULL,
  phone_blind_index bytea NOT NULL CHECK (octet_length(phone_blind_index) = 32),
  relationship text NOT NULL,
  sort_order smallint NOT NULL DEFAULT 1 CHECK (sort_order > 0),
  is_active boolean NOT NULL DEFAULT true,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (organization_id, id),
  FOREIGN KEY (organization_id, child_id) REFERENCES app.child(organization_id, id) ON DELETE RESTRICT
);
CREATE UNIQUE INDEX emergency_contact_active_sort_uq
  ON app.emergency_contact (organization_id, child_id, sort_order) WHERE is_active;

CREATE TABLE app.pickup_request (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  child_id uuid NOT NULL,
  requested_by_user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  authorized_person_id uuid,
  recipient_name_ciphertext text NOT NULL,
  recipient_relationship text NOT NULL,
  estimated_arrival_at timestamptz,
  status text NOT NULL DEFAULT 'YOLDA' CHECK (status IN ('YOLDA', 'HAZIRLANIYOR', 'TESLIM_EDILDI', 'ENGELLENDI', 'IPTAL', 'SURESI_DOLDU')),
  pin_hash bytea CHECK (pin_hash IS NULL OR octet_length(pin_hash) = 32),
  pin_display_ciphertext text,
  failed_pin_attempts smallint NOT NULL DEFAULT 0 CHECK (failed_pin_attempts BETWEEN 0 AND 3),
  handed_over_by_user_id uuid REFERENCES app.user_account(id) ON DELETE RESTRICT,
  block_code text,
  closure_reason text,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  preparing_at timestamptz,
  completed_at timestamptz,
  updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (organization_id, id),
  FOREIGN KEY (organization_id, child_id) REFERENCES app.child(organization_id, id) ON DELETE RESTRICT,
  FOREIGN KEY (organization_id, authorized_person_id) REFERENCES app.authorized_person(organization_id, id) ON DELETE RESTRICT,
  CHECK ((status IN ('YOLDA', 'HAZIRLANIYOR')) OR (pin_display_ciphertext IS NULL AND pin_hash IS NULL)),
  CHECK (status <> 'TESLIM_EDILDI' OR (handed_over_by_user_id IS NOT NULL AND completed_at IS NOT NULL))
);
CREATE UNIQUE INDEX pickup_request_one_open_per_child_uq
  ON app.pickup_request (organization_id, child_id)
  WHERE status IN ('YOLDA', 'HAZIRLANIYOR', 'ENGELLENDI');
CREATE INDEX pickup_request_staff_queue_idx
  ON app.pickup_request (organization_id, status, estimated_arrival_at, created_at)
  WHERE status IN ('YOLDA', 'HAZIRLANIYOR', 'ENGELLENDI');
CREATE INDEX pickup_request_requester_idx ON app.pickup_request (organization_id, requested_by_user_id, created_at DESC);

-- Media, announcements, communication, calendar and notifications
CREATE TABLE app.media (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  uploaded_by_user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  type text NOT NULL CHECK (type IN ('FOTOGRAF', 'VIDEO', 'DOSYA')),
  status text NOT NULL DEFAULT 'KARANTINADA' CHECK (status IN ('KARANTINADA', 'ISLENIYOR', 'INCELEMEDE', 'YAYINDA', 'REDDEDILDI', 'SILINDI')),
  quarantine_object_key text NOT NULL,
  published_object_key text,
  sha256 bytea CHECK (sha256 IS NULL OR octet_length(sha256) = 32),
  size_bytes bigint CHECK (size_bytes IS NULL OR size_bytes >= 0),
  mime_type text,
  data_category text NOT NULL DEFAULT 'MEDYA',
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  published_at timestamptz,
  deleted_at timestamptz,
  UNIQUE (organization_id, id),
  UNIQUE (organization_id, quarantine_object_key)
);
CREATE INDEX media_organization_status_idx ON app.media (organization_id, status, created_at DESC);
CREATE INDEX media_uploader_idx ON app.media (organization_id, uploaded_by_user_id, created_at DESC);

CREATE TABLE app.media_tag (
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  media_id uuid NOT NULL,
  child_id uuid NOT NULL,
  face_boxes jsonb,
  visibility text NOT NULL DEFAULT 'GIZLI' CHECK (visibility IN ('VELIYE_OZEL', 'GRUP', 'TANITIM', 'GIZLI')),
  PRIMARY KEY (organization_id, media_id, child_id),
  FOREIGN KEY (organization_id, media_id) REFERENCES app.media(organization_id, id) ON DELETE RESTRICT,
  FOREIGN KEY (organization_id, child_id) REFERENCES app.child(organization_id, id) ON DELETE RESTRICT,
  CHECK (face_boxes IS NULL OR jsonb_typeof(face_boxes) = 'array')
);
CREATE INDEX media_tag_child_idx ON app.media_tag (organization_id, child_id, media_id);

CREATE TABLE app.media_derivative (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  media_id uuid NOT NULL,
  object_key text NOT NULL,
  type text NOT NULL CHECK (type IN ('KUCULTULMUS', 'BULANIKLASTIRILMIS', 'ONIZLEME', 'ARSIV')),
  version integer NOT NULL DEFAULT 1 CHECK (version > 0),
  sha256 bytea NOT NULL CHECK (octet_length(sha256) = 32),
  source_sha256 bytea NOT NULL CHECK (octet_length(source_sha256) = 32),
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  deleted_at timestamptz,
  UNIQUE (organization_id, id),
  UNIQUE (organization_id, media_id, type, version),
  UNIQUE (organization_id, object_key),
  FOREIGN KEY (organization_id, media_id) REFERENCES app.media(organization_id, id) ON DELETE RESTRICT
);
CREATE INDEX media_derivative_media_idx ON app.media_derivative (organization_id, media_id, type, version DESC);

ALTER TABLE app.child
  ADD CONSTRAINT child_profile_media_fk
  FOREIGN KEY (organization_id, profile_media_id) REFERENCES app.media(organization_id, id) ON DELETE RESTRICT;
ALTER TABLE app.authorized_person
  ADD CONSTRAINT authorized_person_photo_media_fk
  FOREIGN KEY (organization_id, photo_media_id) REFERENCES app.media(organization_id, id) ON DELETE RESTRICT;

CREATE TABLE app.announcement (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  created_by_user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  title text NOT NULL CHECK (length(btrim(title)) BETWEEN 1 AND 200),
  body text NOT NULL,
  status text NOT NULL DEFAULT 'TASLAK' CHECK (status IN ('TASLAK', 'ONAY_BEKLIYOR', 'YAYINDA', 'ARSIV')),
  approval_required boolean NOT NULL DEFAULT false,
  published_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (organization_id, id)
);
CREATE INDEX announcement_status_publication_idx ON app.announcement (organization_id, status, published_at DESC);

CREATE TABLE app.announcement_target (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  announcement_id uuid NOT NULL,
  target_type text NOT NULL CHECK (target_type IN ('KURUM', 'SINIF', 'VELI', 'COCUK')),
  class_id uuid,
  user_id uuid REFERENCES app.user_account(id) ON DELETE RESTRICT,
  child_id uuid,
  CHECK (
    (target_type = 'KURUM' AND class_id IS NULL AND user_id IS NULL AND child_id IS NULL) OR
    (target_type = 'SINIF' AND class_id IS NOT NULL AND user_id IS NULL AND child_id IS NULL) OR
    (target_type = 'VELI' AND class_id IS NULL AND user_id IS NOT NULL AND child_id IS NULL) OR
    (target_type = 'COCUK' AND class_id IS NULL AND user_id IS NULL AND child_id IS NOT NULL)
  ),
  FOREIGN KEY (organization_id, announcement_id) REFERENCES app.announcement(organization_id, id) ON DELETE RESTRICT,
  FOREIGN KEY (organization_id, class_id) REFERENCES app.classroom(organization_id, id) ON DELETE RESTRICT,
  FOREIGN KEY (organization_id, child_id) REFERENCES app.child(organization_id, id) ON DELETE RESTRICT
);
CREATE INDEX announcement_target_class_idx ON app.announcement_target (organization_id, class_id, announcement_id) WHERE class_id IS NOT NULL;
CREATE INDEX announcement_target_user_idx ON app.announcement_target (organization_id, user_id, announcement_id) WHERE user_id IS NOT NULL;
CREATE INDEX announcement_target_child_idx ON app.announcement_target (organization_id, child_id, announcement_id) WHERE child_id IS NOT NULL;

CREATE TABLE app.announcement_read (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  announcement_id uuid NOT NULL,
  user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  read_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (organization_id, announcement_id, user_id),
  FOREIGN KEY (organization_id, announcement_id) REFERENCES app.announcement(organization_id, id) ON DELETE RESTRICT
);
CREATE INDEX announcement_read_user_idx ON app.announcement_read (organization_id, user_id, read_at DESC);

CREATE TABLE app.announcement_approval (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  announcement_id uuid NOT NULL,
  user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  decision text NOT NULL CHECK (decision IN ('ONAY', 'RED')),
  decision_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  FOREIGN KEY (organization_id, announcement_id) REFERENCES app.announcement(organization_id, id) ON DELETE RESTRICT
);
CREATE INDEX announcement_approval_user_idx ON app.announcement_approval (organization_id, user_id, decision_at DESC);

CREATE TABLE app.conversation (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  type text NOT NULL CHECK (type IN ('VELI_OGRETMEN', 'VELI_YONETIM')),
  child_id uuid,
  created_by_user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  closed_at timestamptz,
  UNIQUE (organization_id, id),
  FOREIGN KEY (organization_id, child_id) REFERENCES app.child(organization_id, id) ON DELETE RESTRICT
);

CREATE TABLE app.conversation_participant (
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  conversation_id uuid NOT NULL,
  user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  added_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  left_at timestamptz,
  PRIMARY KEY (organization_id, conversation_id, user_id),
  FOREIGN KEY (organization_id, conversation_id) REFERENCES app.conversation(organization_id, id) ON DELETE RESTRICT
);
CREATE INDEX conversation_participant_user_idx ON app.conversation_participant (organization_id, user_id, conversation_id);

CREATE TABLE app.message (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  conversation_id uuid NOT NULL,
  sender_user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  body_ciphertext text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  deleted_at timestamptz,
  UNIQUE (organization_id, id),
  FOREIGN KEY (organization_id, conversation_id) REFERENCES app.conversation(organization_id, id) ON DELETE RESTRICT
);
CREATE INDEX message_conversation_idx ON app.message (organization_id, conversation_id, created_at DESC, id DESC);
CREATE INDEX message_sender_idx ON app.message (organization_id, sender_user_id, created_at DESC);

CREATE TABLE app.message_read (
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  message_id uuid NOT NULL,
  user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  read_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  PRIMARY KEY (organization_id, message_id, user_id),
  FOREIGN KEY (organization_id, message_id) REFERENCES app.message(organization_id, id) ON DELETE RESTRICT
);
CREATE INDEX message_read_user_idx ON app.message_read (organization_id, user_id, read_at DESC);

CREATE TABLE app.message_hidden (
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  message_id uuid NOT NULL,
  user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  hidden_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  PRIMARY KEY (organization_id, message_id, user_id),
  FOREIGN KEY (organization_id, message_id) REFERENCES app.message(organization_id, id) ON DELETE RESTRICT
);
CREATE INDEX message_hidden_user_idx ON app.message_hidden (organization_id, user_id, hidden_at DESC);

CREATE TABLE app.message_attachment (
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  message_id uuid NOT NULL,
  media_id uuid NOT NULL,
  PRIMARY KEY (organization_id, message_id, media_id),
  FOREIGN KEY (organization_id, message_id) REFERENCES app.message(organization_id, id) ON DELETE RESTRICT,
  FOREIGN KEY (organization_id, media_id) REFERENCES app.media(organization_id, id) ON DELETE RESTRICT
);

CREATE TABLE app.announcement_attachment (
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  announcement_id uuid NOT NULL,
  media_id uuid NOT NULL,
  PRIMARY KEY (organization_id, announcement_id, media_id),
  FOREIGN KEY (organization_id, announcement_id) REFERENCES app.announcement(organization_id, id) ON DELETE RESTRICT,
  FOREIGN KEY (organization_id, media_id) REFERENCES app.media(organization_id, id) ON DELETE RESTRICT
);

CREATE TABLE app.calendar_event (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  class_id uuid,
  title text NOT NULL CHECK (length(btrim(title)) BETWEEN 1 AND 200),
  description text,
  starts_at timestamptz NOT NULL,
  ends_at timestamptz NOT NULL,
  created_by_user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (organization_id, id),
  FOREIGN KEY (organization_id, class_id) REFERENCES app.classroom(organization_id, id) ON DELETE RESTRICT,
  CHECK (ends_at >= starts_at)
);

CREATE TABLE app.event_participation (
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  calendar_event_id uuid NOT NULL,
  child_id uuid NOT NULL,
  participation_status text NOT NULL DEFAULT 'BEKLIYOR' CHECK (participation_status IN ('BEKLIYOR', 'KATILACAK', 'KATILMAYACAK')),
  responded_by_user_id uuid REFERENCES app.user_account(id) ON DELETE RESTRICT,
  responded_at timestamptz,
  PRIMARY KEY (organization_id, calendar_event_id, child_id),
  FOREIGN KEY (organization_id, calendar_event_id) REFERENCES app.calendar_event(organization_id, id) ON DELETE RESTRICT,
  FOREIGN KEY (organization_id, child_id) REFERENCES app.child(organization_id, id) ON DELETE RESTRICT
);
CREATE INDEX event_participation_child_idx ON app.event_participation (organization_id, child_id, calendar_event_id);

CREATE TABLE app.notification (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid REFERENCES app.organization(id) ON DELETE RESTRICT,
  user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  type text NOT NULL,
  source_type text,
  source_id uuid,
  read_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE INDEX notification_user_unread_idx ON app.notification (user_id, created_at DESC) WHERE read_at IS NULL;
CREATE INDEX notification_organization_user_idx ON app.notification (organization_id, user_id, created_at DESC) WHERE organization_id IS NOT NULL;

CREATE TABLE app.notification_preference (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  channel text NOT NULL CHECK (channel IN ('PUSH', 'EPOSTA', 'SMS')),
  category text NOT NULL,
  is_enabled boolean NOT NULL DEFAULT true,
  updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (user_id, channel, category)
);

-- Sessions, privacy requests, retention, audit and background jobs
CREATE TABLE app.session (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  family_id uuid NOT NULL,
  device_id uuid,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  last_activity_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  ip_hash bytea,
  revoked_at timestamptz,
  revocation_reason text,
  UNIQUE (id, user_id)
);

CREATE TABLE app.refresh_token (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  session_id uuid NOT NULL,
  user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  token_hash bytea NOT NULL UNIQUE CHECK (octet_length(token_hash) = 32),
  previous_token_id uuid REFERENCES app.refresh_token(id) ON DELETE RESTRICT,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  expires_at timestamptz NOT NULL,
  consumed_at timestamptz,
  FOREIGN KEY (session_id, user_id) REFERENCES app.session(id, user_id) ON DELETE RESTRICT
);

CREATE TABLE app.device (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  platform text NOT NULL CHECK (platform IN ('IOS', 'ANDROID', 'WEB')),
  device_hash bytea NOT NULL CHECK (octet_length(device_hash) = 32),
  push_token_ciphertext text,
  push_provider text CHECK (push_provider IN ('FCM', 'APNS')),
  environment text CHECK (environment IN ('GELISTIRME', 'URETIM')),
  recorded_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  last_seen_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  revoked_at timestamptz,
  UNIQUE (user_id, platform, device_hash),
  UNIQUE (id, user_id),
  CHECK ((push_token_ciphertext IS NULL) = (push_provider IS NULL))
);
ALTER TABLE app.session
  ADD CONSTRAINT session_device_user_fk
  FOREIGN KEY (device_id, user_id) REFERENCES app.device(id, user_id) ON DELETE RESTRICT;
CREATE INDEX device_user_active_idx ON app.device (user_id, last_seen_at DESC) WHERE revoked_at IS NULL;

CREATE TABLE app.privacy_request (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  type text NOT NULL CHECK (type IN ('BILGI', 'DUZELTME', 'SILME', 'ITIRAZ', 'DIGER')),
  description_ciphertext text NOT NULL,
  status text NOT NULL DEFAULT 'ACIK' CHECK (status IN ('ACIK', 'INCELEMEDE', 'YANITLANDI', 'KAPATILDI')),
  operational_target_at timestamptz NOT NULL,
  legal_deadline_at timestamptz NOT NULL,
  response_ciphertext text,
  reason_ciphertext text,
  assigned_to_user_id uuid REFERENCES app.user_account(id) ON DELETE RESTRICT,
  responded_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (organization_id, id),
  CHECK (legal_deadline_at >= operational_target_at)
);
CREATE INDEX privacy_request_due_idx ON app.privacy_request (organization_id, status, legal_deadline_at);

CREATE TABLE app.data_export_request (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  status text NOT NULL DEFAULT 'BEKLIYOR' CHECK (status IN ('BEKLIYOR', 'HAZIRLANIYOR', 'HAZIR', 'BASARISIZ', 'SILINDI')),
  object_key text,
  ready_at timestamptz,
  access_expires_at timestamptz,
  deleted_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (organization_id, id),
  CHECK (access_expires_at IS NULL OR ready_at IS NULL OR access_expires_at <= ready_at + interval '24 hours')
);
CREATE INDEX data_export_request_user_idx ON app.data_export_request (organization_id, user_id, created_at DESC);

CREATE TABLE app.retention_policy (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  data_category text NOT NULL,
  retention_interval interval NOT NULL CHECK (retention_interval >= interval '0 seconds'),
  retention_start_event text NOT NULL,
  expiry_action text NOT NULL CHECK (expiry_action IN ('SIL', 'ANONIMLESTIR')),
  legal_hold_required boolean NOT NULL DEFAULT false,
  is_enabled boolean NOT NULL DEFAULT true,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (organization_id, data_category)
);

CREATE TABLE app.destruction_record (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  data_category text NOT NULL,
  execution_id uuid NOT NULL,
  starts_at timestamptz NOT NULL,
  ends_at timestamptz NOT NULL,
  record_count bigint NOT NULL CHECK (record_count >= 0),
  method text NOT NULL CHECK (method IN ('SILME', 'ANONIMLESTIRME', 'KRIPTOGRAFIK_IMHA')),
  result text NOT NULL CHECK (result IN ('BASARILI', 'KISMI', 'BASARISIZ')),
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  CHECK (ends_at >= starts_at)
);
CREATE INDEX destruction_record_category_idx ON app.destruction_record (organization_id, data_category, ends_at DESC);

CREATE TABLE app.audit_chain_head (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid REFERENCES app.organization(id) ON DELETE RESTRICT,
  chain_type text NOT NULL CHECK (chain_type IN ('KURUM', 'PLATFORM')),
  last_record_id uuid,
  last_hash bytea CHECK (last_hash IS NULL OR octet_length(last_hash) = 32),
  last_event_at timestamptz,
  UNIQUE (id, organization_id),
  UNIQUE (id, chain_type),
  CHECK ((chain_type = 'KURUM' AND organization_id IS NOT NULL) OR (chain_type = 'PLATFORM' AND organization_id IS NULL))
);
CREATE UNIQUE INDEX audit_chain_organization_uq ON app.audit_chain_head (organization_id) WHERE organization_id IS NOT NULL;
CREATE UNIQUE INDEX audit_chain_platform_uq ON app.audit_chain_head (chain_type) WHERE organization_id IS NULL;

CREATE TABLE app.audit_record (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  chain_id uuid NOT NULL,
  actor_user_id uuid REFERENCES app.user_account(id) ON DELETE RESTRICT,
  action text NOT NULL CHECK (length(btrim(action)) BETWEEN 1 AND 100),
  target_type text NOT NULL,
  target_id uuid,
  occurred_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  previous_hash bytea CHECK (previous_hash IS NULL OR octet_length(previous_hash) = 32),
  record_hash bytea NOT NULL CHECK (octet_length(record_hash) = 32),
  details_ciphertext text,
  UNIQUE (id, organization_id),
  FOREIGN KEY (chain_id, organization_id) REFERENCES app.audit_chain_head(id, organization_id) ON DELETE RESTRICT
);
CREATE INDEX audit_record_organization_time_idx ON app.audit_record (organization_id, occurred_at DESC, id DESC);
CREATE INDEX audit_record_target_idx ON app.audit_record (organization_id, target_type, target_id, occurred_at DESC);

CREATE TABLE app.platform_audit_record (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  chain_id uuid NOT NULL,
  chain_type text NOT NULL DEFAULT 'PLATFORM' CHECK (chain_type = 'PLATFORM'),
  actor_user_id uuid REFERENCES app.user_account(id) ON DELETE RESTRICT,
  action text NOT NULL CHECK (length(btrim(action)) BETWEEN 1 AND 100),
  target_type text NOT NULL,
  target_id uuid,
  occurred_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  previous_hash bytea CHECK (previous_hash IS NULL OR octet_length(previous_hash) = 32),
  record_hash bytea NOT NULL CHECK (octet_length(record_hash) = 32),
  details_ciphertext text,
  FOREIGN KEY (chain_id, chain_type) REFERENCES app.audit_chain_head(id, chain_type) ON DELETE RESTRICT
);

CREATE TABLE app.idempotency_key (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid REFERENCES app.organization(id) ON DELETE RESTRICT,
  user_id uuid REFERENCES app.user_account(id) ON DELETE RESTRICT,
  http_method text NOT NULL CHECK (http_method IN ('POST', 'PUT', 'PATCH', 'DELETE')),
  route_template text NOT NULL,
  idempotency_key uuid NOT NULL,
  request_sha256 bytea NOT NULL CHECK (octet_length(request_sha256) = 32),
  status text NOT NULL DEFAULT 'BEKLIYOR' CHECK (status IN ('BEKLIYOR', 'TAMAMLANDI')),
  response_status smallint CHECK (response_status BETWEEN 200 AND 299),
  response_ciphertext text,
  expires_at timestamptz NOT NULL,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE NULLS NOT DISTINCT (organization_id, user_id, http_method, route_template, idempotency_key),
  CHECK (organization_id IS NOT NULL OR user_id IS NOT NULL),
  CHECK ((status = 'TAMAMLANDI') = (response_status IS NOT NULL))
);
CREATE INDEX idempotency_expiry_idx ON app.idempotency_key (expires_at);

CREATE TABLE app.job_queue (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid REFERENCES app.organization(id) ON DELETE RESTRICT,
  job_type text NOT NULL,
  source_type text,
  source_id uuid,
  payload jsonb NOT NULL DEFAULT '{}'::jsonb,
  status text NOT NULL DEFAULT 'BEKLIYOR' CHECK (status IN ('BEKLIYOR', 'ISLENIYOR', 'YENIDEN_DENE', 'TAMAMLANDI', 'BASARISIZ', 'IPTAL')),
  attempt_count integer NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
  max_attempts integer NOT NULL DEFAULT 8 CHECK (max_attempts > 0),
  retry_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  lease_owner uuid,
  lease_until timestamptz,
  last_error_code text,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  completed_at timestamptz,
  CHECK (jsonb_typeof(payload) = 'object')
);
CREATE INDEX job_queue_claim_idx ON app.job_queue (retry_at, created_at) WHERE status IN ('BEKLIYOR', 'YENIDEN_DENE');
CREATE INDEX job_queue_lease_idx ON app.job_queue (lease_until) WHERE status = 'ISLENIYOR';

CREATE TABLE app.domain_event_outbox (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid REFERENCES app.organization(id) ON DELETE RESTRICT,
  event_code text NOT NULL,
  source_type text NOT NULL,
  source_id uuid NOT NULL,
  source_version bigint NOT NULL CHECK (source_version > 0),
  payload jsonb NOT NULL DEFAULT '{}'::jsonb,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  published_at timestamptz,
  UNIQUE (event_code, source_id, source_version),
  CHECK (jsonb_typeof(payload) = 'object')
);
CREATE INDEX domain_event_outbox_pending_idx ON app.domain_event_outbox (created_at, id) WHERE published_at IS NULL;

CREATE TABLE app.scheduled_job (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  job_name text NOT NULL UNIQUE,
  timezone text NOT NULL DEFAULT 'Europe/Istanbul',
  cron_expression text NOT NULL,
  next_run_at timestamptz NOT NULL,
  lease_owner uuid,
  lease_until timestamptz,
  is_enabled boolean NOT NULL DEFAULT true,
  updated_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE INDEX scheduled_job_due_idx ON app.scheduled_job (next_run_at) WHERE is_enabled;

CREATE TABLE app.scheduled_job_run (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  scheduled_job_id uuid NOT NULL REFERENCES app.scheduled_job(id) ON DELETE RESTRICT,
  scheduled_for timestamptz NOT NULL,
  starts_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  ends_at timestamptz,
  result text NOT NULL DEFAULT 'CALISIYOR' CHECK (result IN ('CALISIYOR', 'BASARILI', 'BASARISIZ', 'ATLANDI')),
  error_code text,
  UNIQUE (scheduled_job_id, scheduled_for),
  CHECK (ends_at IS NULL OR ends_at >= starts_at)
);
CREATE INDEX scheduled_job_run_recent_idx ON app.scheduled_job_run (scheduled_job_id, starts_at DESC);

CREATE TABLE app.data_import (
  id uuid PRIMARY KEY DEFAULT uuidv7(),
  organization_id uuid NOT NULL REFERENCES app.organization(id) ON DELETE RESTRICT,
  started_by_user_id uuid NOT NULL REFERENCES app.user_account(id) ON DELETE RESTRICT,
  file_object_key text NOT NULL,
  file_sha256 bytea NOT NULL CHECK (octet_length(file_sha256) = 32),
  status text NOT NULL DEFAULT 'YUKLENDI' CHECK (status IN ('YUKLENDI', 'TARANIYOR', 'ONIZLEME', 'UYGULANIYOR', 'TAMAMLANDI', 'BASARISIZ', 'SILINDI')),
  result_summary jsonb,
  cleanup_deadline timestamptz NOT NULL,
  created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
  UNIQUE (organization_id, id),
  CHECK (result_summary IS NULL OR jsonb_typeof(result_summary) = 'object')
);
CREATE INDEX data_import_cleanup_idx ON app.data_import (cleanup_deadline) WHERE status <> 'SILINDI';

-- Remaining access paths and tenant integrity
CREATE INDEX branch_organization_idx ON app.branch (organization_id);
CREATE INDEX school_year_organization_date_idx ON app.school_year (organization_id, start_date DESC);
CREATE INDEX classroom_organization_year_idx ON app.classroom (organization_id, school_year_id, is_active);
CREATE INDEX child_class_status_idx ON app.child (organization_id, class_id, status, id);
CREATE INDEX privacy_notice_effective_idx ON app.privacy_notice (organization_id, audience, effective_at DESC);
CREATE INDEX privacy_notice_read_user_idx ON app.privacy_notice_read (organization_id, user_id, read_at DESC);
CREATE INDEX privacy_notice_read_child_idx ON app.privacy_notice_read (organization_id, child_id, notice_id) WHERE child_id IS NOT NULL;
CREATE INDEX consent_record_latest_decision_idx ON app.consent_record (organization_id, child_id, guardian_user_id, consent_type, decision_at DESC, id DESC);
CREATE INDEX consent_record_guardian_idx ON app.consent_record (organization_id, guardian_user_id, decision_at DESC);
CREATE INDEX consent_review_child_idx ON app.consent_review (organization_id, child_id, guardian_user_id, reviewed_at DESC);
CREATE INDEX meal_allergen_lookup_idx ON app.meal_allergen (organization_id, allergen_code, meal_definition_id);

-- Tenant tables use one transaction-local organization context.
CREATE FUNCTION app.has_active_membership(p_organization_id uuid)
RETURNS boolean
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = pg_catalog, app
AS $function$
  SELECT EXISTS (
    SELECT 1
    FROM app.membership AS membership
    WHERE membership.organization_id = p_organization_id
      AND membership.user_id = (SELECT NULLIF(current_setting('app.user_id', true), '')::uuid)
      AND membership.status = 'AKTIF'
  );
$function$;

CREATE FUNCTION app.list_user_memberships()
RETURNS TABLE (organization_id uuid, membership_id uuid, organization_name text, role text, membership_status text)
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = pg_catalog, app
AS $function$
  SELECT membership.organization_id, membership.id, organization.name, membership.role, membership.status
  FROM app.membership AS membership
  JOIN app.organization AS organization ON organization.id = membership.organization_id
  WHERE membership.user_id = (SELECT NULLIF(current_setting('app.user_id', true), '')::uuid)
  ORDER BY membership.starts_at DESC;
$function$;

CREATE FUNCTION app.find_user_id_by_blind_index(p_type text, p_version smallint, p_blind_index bytea)
RETURNS uuid
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = pg_catalog, app
AS $function$
  SELECT contact.user_id
  FROM app.user_contact AS contact
  JOIN app.user_account AS account ON account.id = contact.user_id
  WHERE contact.type = p_type
    AND contact.blind_index_version = p_version
    AND contact.blind_index = p_blind_index
    AND contact.verified_at IS NOT NULL
    AND account.status = 'AKTIF'
  LIMIT 1;
$function$;

-- RLS is forced even for table owners; all policies explicitly name the intended roles.
DO $tenant_rls$
DECLARE table_name text;
BEGIN
  FOREACH table_name IN ARRAY ARRAY[
    'branch', 'school_year', 'classroom', 'invitation', 'invitation_scope', 'membership', 'membership_class',
    'child', 'guardian_child', 'access_restriction', 'organization_settings', 'organization_module', 'activity_type',
    'privacy_notice', 'privacy_notice_read', 'consent_record', 'consent_review', 'health_profile',
    'meal_menu', 'meal_definition', 'meal_allergen', 'daily_record', 'medication_instruction', 'attendance',
    'absence_notice', 'authorized_person', 'authorized_user_child', 'emergency_contact', 'pickup_request',
    'media', 'media_tag', 'media_derivative', 'announcement', 'announcement_target', 'announcement_read',
    'announcement_approval', 'conversation', 'conversation_participant', 'message', 'message_read', 'message_hidden',
    'message_attachment', 'announcement_attachment', 'calendar_event', 'event_participation', 'privacy_request',
    'data_export_request', 'retention_policy', 'destruction_record', 'audit_chain_head', 'audit_record',
    'domain_event_outbox', 'data_import'
  ] LOOP
    EXECUTE format('ALTER TABLE app.%I ENABLE ROW LEVEL SECURITY', table_name);
    EXECUTE format('ALTER TABLE app.%I FORCE ROW LEVEL SECURITY', table_name);
    EXECUTE format(
      'CREATE POLICY tenant_scope ON app.%I FOR ALL TO okuldayim_app USING (organization_id = (SELECT NULLIF(current_setting(''app.organization_id'', true), '''')::uuid)) WITH CHECK (organization_id = (SELECT NULLIF(current_setting(''app.organization_id'', true), '''')::uuid))',
      table_name
    );
    EXECUTE format('CREATE POLICY owner_access ON app.%I FOR ALL TO okuldayim_owner USING (true) WITH CHECK (true)', table_name);
  END LOOP;
END
$tenant_rls$;

DO $user_rls$
DECLARE table_name text;
BEGIN
  FOREACH table_name IN ARRAY ARRAY['user_contact', 'mfa_record'] LOOP
    EXECUTE format('ALTER TABLE app.%I ENABLE ROW LEVEL SECURITY', table_name);
    EXECUTE format('ALTER TABLE app.%I FORCE ROW LEVEL SECURITY', table_name);
    EXECUTE format(
      'CREATE POLICY user_self ON app.%I FOR ALL TO okuldayim_app USING (user_id = (SELECT NULLIF(current_setting(''app.user_id'', true), '''')::uuid)) WITH CHECK (user_id = (SELECT NULLIF(current_setting(''app.user_id'', true), '''')::uuid))',
      table_name
    );
    EXECUTE format('CREATE POLICY owner_access ON app.%I FOR ALL TO okuldayim_owner USING (true) WITH CHECK (true)', table_name);
  END LOOP;
END
$user_rls$;

ALTER TABLE app.user_account ENABLE ROW LEVEL SECURITY;
ALTER TABLE app.user_account FORCE ROW LEVEL SECURITY;
CREATE POLICY user_self ON app.user_account FOR ALL TO okuldayim_app
  USING (id = (SELECT NULLIF(current_setting('app.user_id', true), '')::uuid))
  WITH CHECK (id = (SELECT NULLIF(current_setting('app.user_id', true), '')::uuid));
CREATE POLICY owner_access ON app.user_account FOR ALL TO okuldayim_owner USING (true) WITH CHECK (true);

DO $session_rls$
DECLARE table_name text;
BEGIN
  FOREACH table_name IN ARRAY ARRAY['session', 'refresh_token', 'device', 'notification_preference'] LOOP
    EXECUTE format('ALTER TABLE app.%I ENABLE ROW LEVEL SECURITY', table_name);
    EXECUTE format('ALTER TABLE app.%I FORCE ROW LEVEL SECURITY', table_name);
    EXECUTE format(
      'CREATE POLICY user_self ON app.%I FOR ALL TO okuldayim_app USING (user_id = (SELECT NULLIF(current_setting(''app.user_id'', true), '''')::uuid)) WITH CHECK (user_id = (SELECT NULLIF(current_setting(''app.user_id'', true), '''')::uuid))',
      table_name
    );
    EXECUTE format('CREATE POLICY owner_access ON app.%I FOR ALL TO okuldayim_owner USING (true) WITH CHECK (true)', table_name);
  END LOOP;
END
$session_rls$;

ALTER TABLE app.mfa_recovery_code ENABLE ROW LEVEL SECURITY;
ALTER TABLE app.mfa_recovery_code FORCE ROW LEVEL SECURITY;
CREATE POLICY user_self ON app.mfa_recovery_code FOR ALL TO okuldayim_app
  USING (EXISTS (
    SELECT 1 FROM app.mfa_record AS mfa
    WHERE mfa.id = mfa_record_id
      AND mfa.user_id = (SELECT NULLIF(current_setting('app.user_id', true), '')::uuid)
  ))
  WITH CHECK (EXISTS (
    SELECT 1 FROM app.mfa_record AS mfa
    WHERE mfa.id = mfa_record_id
      AND mfa.user_id = (SELECT NULLIF(current_setting('app.user_id', true), '')::uuid)
  ));
CREATE POLICY owner_access ON app.mfa_recovery_code FOR ALL TO okuldayim_owner USING (true) WITH CHECK (true);

ALTER TABLE app.notification ENABLE ROW LEVEL SECURITY;
ALTER TABLE app.notification FORCE ROW LEVEL SECURITY;
CREATE POLICY notification_self ON app.notification FOR ALL TO okuldayim_app
  USING (user_id = (SELECT NULLIF(current_setting('app.user_id', true), '')::uuid)
    AND (organization_id IS NULL OR organization_id = (SELECT NULLIF(current_setting('app.organization_id', true), '')::uuid)))
  WITH CHECK (user_id = (SELECT NULLIF(current_setting('app.user_id', true), '')::uuid)
    AND (organization_id IS NULL OR organization_id = (SELECT NULLIF(current_setting('app.organization_id', true), '')::uuid)));
CREATE POLICY owner_access ON app.notification FOR ALL TO okuldayim_owner USING (true) WITH CHECK (true);

ALTER TABLE app.idempotency_key ENABLE ROW LEVEL SECURITY;
ALTER TABLE app.idempotency_key FORCE ROW LEVEL SECURITY;
CREATE POLICY idempotency_owner_scope ON app.idempotency_key FOR ALL TO okuldayim_app
  USING (user_id = (SELECT NULLIF(current_setting('app.user_id', true), '')::uuid)
    AND (organization_id IS NULL OR organization_id = (SELECT NULLIF(current_setting('app.organization_id', true), '')::uuid)))
  WITH CHECK (user_id = (SELECT NULLIF(current_setting('app.user_id', true), '')::uuid)
    AND (organization_id IS NULL OR organization_id = (SELECT NULLIF(current_setting('app.organization_id', true), '')::uuid)));
CREATE POLICY owner_access ON app.idempotency_key FOR ALL TO okuldayim_owner USING (true) WITH CHECK (true);

ALTER TABLE app.organization ENABLE ROW LEVEL SECURITY;
ALTER TABLE app.organization FORCE ROW LEVEL SECURITY;
CREATE POLICY organization_member ON app.organization FOR SELECT TO okuldayim_app
  USING (id = (SELECT NULLIF(current_setting('app.organization_id', true), '')::uuid) OR app.has_active_membership(id));
CREATE POLICY owner_access ON app.organization FOR ALL TO okuldayim_owner USING (true) WITH CHECK (true);

DO $internal_rls$
DECLARE table_name text;
BEGIN
  FOREACH table_name IN ARRAY ARRAY['job_queue', 'scheduled_job', 'scheduled_job_run', 'platform_audit_record'] LOOP
    EXECUTE format('ALTER TABLE app.%I ENABLE ROW LEVEL SECURITY', table_name);
    EXECUTE format('ALTER TABLE app.%I FORCE ROW LEVEL SECURITY', table_name);
    EXECUTE format('CREATE POLICY owner_access ON app.%I FOR ALL TO okuldayim_owner USING (true) WITH CHECK (true)', table_name);
  END LOOP;
END
$internal_rls$;

REVOKE ALL ON FUNCTION app.has_active_membership(uuid) FROM PUBLIC;
REVOKE ALL ON FUNCTION app.list_user_memberships() FROM PUBLIC;
REVOKE ALL ON FUNCTION app.find_user_id_by_blind_index(text, smallint, bytea) FROM PUBLIC;

REVOKE ALL ON ALL TABLES IN SCHEMA app FROM PUBLIC, okuldayim_app;
GRANT USAGE ON SCHEMA app TO okuldayim_app;
GRANT SELECT, INSERT, UPDATE ON ALL TABLES IN SCHEMA app TO okuldayim_app;
REVOKE ALL ON app.schema_migrations, app.job_queue, app.scheduled_job, app.scheduled_job_run,
  app.platform_audit_record FROM okuldayim_app;
REVOKE INSERT, UPDATE, DELETE ON app.organization FROM okuldayim_app;
GRANT SELECT ON app.organization TO okuldayim_app;
REVOKE UPDATE ON app.consent_record, app.consent_review, app.privacy_notice_read, app.access_restriction,
  app.audit_record, app.destruction_record, app.domain_event_outbox, app.announcement_read,
  app.announcement_approval, app.message_read, app.message_hidden FROM okuldayim_app;
GRANT SELECT ON app.effective_consent TO okuldayim_app;
REVOKE EXECUTE ON ALL FUNCTIONS IN SCHEMA app FROM PUBLIC, okuldayim_app;
GRANT EXECUTE ON FUNCTION app.has_active_membership(uuid) TO okuldayim_app;
GRANT EXECUTE ON FUNCTION app.list_user_memberships() TO okuldayim_app;
GRANT EXECUTE ON FUNCTION app.find_user_id_by_blind_index(text, smallint, bytea) TO okuldayim_app;

COMMENT ON TABLE app.daily_record IS 'Medication and incident details are stored as Vault Transit ciphertext.';
COMMENT ON TABLE app.job_queue IS 'Queue payloads must contain identifiers and metadata only; runtime access belongs in scoped worker functions.';
COMMENT ON TABLE app.consent_record IS 'Append-only consent decision history; correction or withdrawal is a new row.';
COMMENT ON TABLE app.audit_record IS 'Append-only tenant hash chain; serialize writes by locking the chain head in the transaction.';
COMMENT ON TABLE app.pickup_request IS 'A partial unique index enforces one open pickup per child; terminal transitions erase PIN material.';

INSERT INTO app.schema_migrations (version, description)
VALUES ('0001_initial_schema', 'Normalized application schema, tenant RLS, least-privilege grants, and access indexes');
COMMIT;

