import { create } from 'zustand';

import type { ChildSummary, Me, Membership, Role } from '../api/types';

export type SessionStatus = 'booting' | 'signedOut' | 'signedIn';
export type SignedOutReason = 'expired' | null;
export type OnboardingStep = 'notice' | 'consents' | null;

interface SessionState {
  status: SessionStatus;
  me: Me | null;
  /** Oturum bilgisi ağdan değil, cihazdaki şifreli önbellekten yüklendi. */
  offline: boolean;
  activeMembershipId: string | null;
  activeChildId: string | null;
  signedOutReason: SignedOutReason;
}

const initialState: SessionState = {
  status: 'booting',
  me: null,
  offline: false,
  activeMembershipId: null,
  activeChildId: null,
  signedOutReason: null,
};

export const useSessionStore = create<SessionState>()(() => initialState);

const ROLE_PRIORITY: readonly Role[] = ['OGRETMEN', 'VELI', 'KURUM_YONETICISI', 'YETKILI_TESLIM_ALAN'];

function defaultMembership(me: Me): Membership | undefined {
  return [...me.memberships].sort((a, b) => ROLE_PRIORITY.indexOf(a.role) - ROLE_PRIORITY.indexOf(b.role))[0];
}

export const sessionActions = {
  setMe(me: Me, options: { offline?: boolean } = {}) {
    useSessionStore.setState((state) => {
      const membership =
        me.memberships.find((m) => m.id === state.activeMembershipId) ?? defaultMembership(me) ?? null;
      const children = membership?.children ?? [];
      const activeChildId = children.some((c) => c.id === state.activeChildId)
        ? state.activeChildId
        : (children[0]?.id ?? null);
      return {
        status: 'signedIn',
        me,
        offline: options.offline ?? false,
        activeMembershipId: membership?.id ?? null,
        activeChildId,
        signedOutReason: null,
      };
    });
  },

  setActiveMembership(membershipId: string) {
    useSessionStore.setState((state) => {
      const membership = state.me?.memberships.find((m) => m.id === membershipId);
      if (!membership) return state;
      return { activeMembershipId: membership.id, activeChildId: membership.children[0]?.id ?? null };
    });
  },

  setActiveChild(childId: string) {
    useSessionStore.setState({ activeChildId: childId });
  },

  signedOut(reason: SignedOutReason = null) {
    useSessionStore.setState({ ...initialState, status: 'signedOut', signedOutReason: reason });
  },
};

export function selectActiveMembership(state: SessionState): Membership | null {
  return state.me?.memberships.find((m) => m.id === state.activeMembershipId) ?? null;
}

export function selectOnboardingStep(state: SessionState): OnboardingStep {
  if (!state.me) return null;
  if (state.me.pendingNoticeKurumIds.length > 0) return 'notice';
  if (state.me.consentReviewChildIds.length > 0) return 'consents';
  return null;
}

export const useActiveMembership = () => useSessionStore(selectActiveMembership);
export const useActiveRole = () => useSessionStore((state) => selectActiveMembership(state)?.role ?? null);
export const useOnboardingStep = () => useSessionStore(selectOnboardingStep);

export function useActiveChild(): ChildSummary | null {
  return useSessionStore(
    (state) => selectActiveMembership(state)?.children.find((c) => c.id === state.activeChildId) ?? null,
  );
}

export function findChild(me: Me | null, childId: string): ChildSummary | undefined {
  return me?.memberships.flatMap((m) => m.children).find((c) => c.id === childId);
}
