-- Phone registration remains invitation-only. These two SECURITY DEFINER functions expose
-- only invitation existence and atomically consume an eligible phone invitation.
BEGIN;
SET LOCAL ROLE okuldayim_owner;
SET LOCAL lock_timeout = '2s';
SET LOCAL statement_timeout = '15s';
SET LOCAL search_path = app, pg_catalog;

CREATE FUNCTION app.has_pending_phone_invitation(
  p_version smallint,
  p_blind_index bytea
)
RETURNS boolean
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = pg_catalog, app
AS $function$
  SELECT EXISTS (
    SELECT 1
    FROM app.invitation AS invitation
    WHERE invitation.target_type = 'TELEFON'
      AND invitation.blind_index_version = p_version
      AND invitation.target_blind_index = p_blind_index
      AND invitation.status = 'BEKLIYOR'
      AND invitation.expires_at > clock_timestamp()
      AND invitation.role <> 'YETKILI_TESLIM_ALAN'
      AND (
        invitation.role <> 'VELI'
        OR EXISTS (
          SELECT 1 FROM app.invitation_scope AS scope
          WHERE scope.organization_id = invitation.organization_id
            AND scope.invitation_id = invitation.id AND scope.child_id IS NOT NULL
        )
      )
      AND (
        invitation.role <> 'OGRETMEN'
        OR EXISTS (
          SELECT 1 FROM app.invitation_scope AS scope
          WHERE scope.organization_id = invitation.organization_id
            AND scope.invitation_id = invitation.id AND scope.class_id IS NOT NULL
        )
      )
  );
$function$;

CREATE FUNCTION app.register_user_from_phone_invitation(
  p_user_id uuid,
  p_phone_ciphertext text,
  p_version smallint,
  p_blind_index bytea
)
RETURNS uuid
LANGUAGE plpgsql
VOLATILE
SECURITY DEFINER
SET search_path = pg_catalog, app
AS $function$
DECLARE
  invitation_row record;
  active_membership_id uuid;
  active_membership_role text;
BEGIN
  IF NOT app.has_pending_phone_invitation(p_version, p_blind_index) THEN
    RETURN NULL;
  END IF;

  INSERT INTO app.user_account (id) VALUES (p_user_id);
  INSERT INTO app.user_contact (
    user_id, type, value_ciphertext, blind_index, blind_index_version,
    verified_at, is_primary
  ) VALUES (
    p_user_id, 'TELEFON', p_phone_ciphertext, p_blind_index, p_version,
    clock_timestamp(), true
  );

  FOR invitation_row IN
    SELECT invitation.id, invitation.organization_id, invitation.role
    FROM app.invitation AS invitation
    WHERE invitation.target_type = 'TELEFON'
      AND invitation.blind_index_version = p_version
      AND invitation.target_blind_index = p_blind_index
      AND invitation.status = 'BEKLIYOR'
      AND invitation.expires_at > clock_timestamp()
      AND invitation.role <> 'YETKILI_TESLIM_ALAN'
      AND (
        invitation.role <> 'VELI'
        OR EXISTS (
          SELECT 1 FROM app.invitation_scope AS scope
          WHERE scope.organization_id = invitation.organization_id
            AND scope.invitation_id = invitation.id AND scope.child_id IS NOT NULL
        )
      )
      AND (
        invitation.role <> 'OGRETMEN'
        OR EXISTS (
          SELECT 1 FROM app.invitation_scope AS scope
          WHERE scope.organization_id = invitation.organization_id
            AND scope.invitation_id = invitation.id AND scope.class_id IS NOT NULL
        )
      )
    ORDER BY invitation.organization_id, invitation.created_at, invitation.id
    FOR UPDATE
  LOOP
    SELECT membership.id, membership.role
    INTO active_membership_id, active_membership_role
    FROM app.membership AS membership
    WHERE membership.organization_id = invitation_row.organization_id
      AND membership.user_id = p_user_id AND membership.status = 'AKTIF'
    LIMIT 1;

    IF active_membership_id IS NULL THEN
      INSERT INTO app.membership (organization_id, user_id, role, status)
      VALUES (invitation_row.organization_id, p_user_id, invitation_row.role, 'AKTIF')
      RETURNING id INTO active_membership_id;
      active_membership_role := invitation_row.role;
    END IF;

    IF active_membership_role <> invitation_row.role THEN
      CONTINUE;
    END IF;

    UPDATE app.invitation
    SET status = 'TUKETILDI', consumed_at = clock_timestamp()
    WHERE organization_id = invitation_row.organization_id
      AND id = invitation_row.id AND status = 'BEKLIYOR';

    IF invitation_row.role = 'OGRETMEN' THEN
      INSERT INTO app.membership_class (organization_id, membership_id, class_id)
      SELECT scope.organization_id, active_membership_id, scope.class_id
      FROM app.invitation_scope AS scope
      WHERE scope.organization_id = invitation_row.organization_id
        AND scope.invitation_id = invitation_row.id AND scope.class_id IS NOT NULL
      ON CONFLICT DO NOTHING;
    ELSIF invitation_row.role = 'VELI' THEN
      INSERT INTO app.guardian_child (
        organization_id, guardian_user_id, child_id, relationship
      )
      SELECT scope.organization_id, p_user_id, scope.child_id, 'DIGER'
      FROM app.invitation_scope AS scope
      WHERE scope.organization_id = invitation_row.organization_id
        AND scope.invitation_id = invitation_row.id AND scope.child_id IS NOT NULL
      ON CONFLICT (organization_id, child_id, guardian_user_id) DO NOTHING;
    END IF;

    active_membership_id := NULL;
    active_membership_role := NULL;
  END LOOP;

  IF NOT EXISTS (
    SELECT 1 FROM app.membership
    WHERE user_id = p_user_id AND status = 'AKTIF'
  ) THEN
    DELETE FROM app.user_contact WHERE user_id = p_user_id;
    DELETE FROM app.user_account WHERE id = p_user_id;
    RETURN NULL;
  END IF;
  RETURN p_user_id;
END;
$function$;

REVOKE ALL ON FUNCTION app.has_pending_phone_invitation(smallint, bytea) FROM PUBLIC;
REVOKE ALL ON FUNCTION app.register_user_from_phone_invitation(uuid, text, smallint, bytea) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION app.has_pending_phone_invitation(smallint, bytea) TO okuldayim_app;
GRANT EXECUTE ON FUNCTION app.register_user_from_phone_invitation(uuid, text, smallint, bytea) TO okuldayim_app;

INSERT INTO app.schema_migrations (version, description)
VALUES ('0003_invited_phone_registration', 'Invitation-only phone registration under RLS');
COMMIT;
