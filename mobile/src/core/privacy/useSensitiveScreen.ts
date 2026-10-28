import { usePreventScreenCapture, useScreenshotListener } from 'expo-screen-capture';

import { outboxPaths } from '../api/endpoints';
import { enqueue, syncOutbox } from '../offline';
import { logger } from './logger';

export type SensitiveScreen = 'teslim-kodu' | 'teslim-dogrulama';

/**
 * KVKK-15: Ekran açıkken Android'de ekran görüntüsü ve kayıt engellenir (FLAG_SECURE). iOS'ta ekran
 * görüntüsü engellenemez; alındığında denetim kaydına işlenir.
 */
export function useSensitiveScreen(screen: SensitiveScreen): void {
  usePreventScreenCapture(screen);
  useScreenshotListener(() => {
    void recordScreenshot(screen);
  });
}

async function recordScreenshot(screen: SensitiveScreen): Promise<void> {
  try {
    await enqueue('POST', outboxPaths.auditEvents, { type: 'EKRAN_GORUNTUSU', screen, at: new Date().toISOString() });
    void syncOutbox(true);
  } catch (error) {
    logger.warn('Ekran görüntüsü olayı kaydedilemedi', error);
  }
}
