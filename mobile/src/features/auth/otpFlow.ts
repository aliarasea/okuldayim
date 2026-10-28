import { create } from 'zustand';

import type { OtpChallenge } from '@/core/api/types';

interface OtpFlowState {
  phone: string | null;
  challengeId: string | null;
  retryAfterSec: number;
}

const empty: OtpFlowState = { phone: null, challengeId: null, retryAfterSec: 0 };

// Telefon numarası yalnızca bellekte, giriş akışı süresince tutulur; URL'ye veya diske yazılmaz.
export const useOtpFlow = create<OtpFlowState>()(() => empty);

export const otpFlowActions = {
  start(phone: string, challenge: OtpChallenge) {
    useOtpFlow.setState({ phone, challengeId: challenge.challengeId, retryAfterSec: challenge.retryAfterSec });
  },
  clear() {
    useOtpFlow.setState(empty);
  },
};
