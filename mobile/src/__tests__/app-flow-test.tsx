// Gerçek src/app rotaları + mock sunucu ile uçtan uca akış: giriş → aydınlatma → rıza → rol ana ekranı.
import { act, fireEvent, renderRouter, screen, waitFor } from 'expo-router/testing-library';

import i18n from '@/core/i18n';
import { signOut } from '@/core/session/sessionService';

jest.mock('expo-secure-store', () => {
  const store = new Map<string, string>();
  return {
    WHEN_UNLOCKED_THIS_DEVICE_ONLY: 0,
    getItemAsync: async (key: string) => store.get(key) ?? null,
    setItemAsync: async (key: string, value: string) => {
      store.set(key, value);
    },
    deleteItemAsync: async (key: string) => {
      store.delete(key);
    },
  };
});

jest.mock('expo-crypto', () => ({
  randomUUID: () => `uuid-${Math.random().toString(36).slice(2)}`,
  getRandomBytes: (count: number) => new Uint8Array(count),
}));

jest.mock('expo-network', () => ({
  useNetworkState: () => ({ isConnected: true }),
  addNetworkStateListener: () => ({ remove: () => undefined }),
}));

jest.mock('expo-screen-capture', () => ({
  usePreventScreenCapture: () => undefined,
  useScreenshotListener: () => undefined,
  enableAppSwitcherProtectionAsync: async () => undefined,
}));

jest.mock('expo-splash-screen', () => ({
  preventAutoHideAsync: async () => true,
  hide: () => undefined,
}));

// SQLCipher yerine bellek içi ayarlar ve kuyruk.
jest.mock('@/core/offline/database', () => {
  const settings = new Map<string, string>();
  return {
    getDatabase: async () => {
      throw new Error('testte kullanılmaz');
    },
    getSetting: async (key: string) => settings.get(key) ?? null,
    setSetting: async (key: string, value: string) => {
      settings.set(key, value);
    },
    wipeDatabase: async () => {
      settings.clear();
    },
  };
});

jest.mock('@/core/offline/sqliteOutboxStore', () => {
  const { MemoryOutboxStore } = jest.requireActual<typeof import('@/core/offline/memoryOutboxStore')>(
    '@/core/offline/memoryOutboxStore',
  );
  return { createSqliteOutboxStore: () => new MemoryOutboxStore() };
});

jest.mock('@/core/api/mock/mockServer', () => {
  const actual = jest.requireActual<typeof import('@/core/api/mock/mockServer')>('@/core/api/mock/mockServer');
  return { createMockTransport: () => actual.createMockTransport({ latencyMs: 0 }) };
});

async function signInWith(phone: string) {
  await fireEvent.changeText(await screen.findByLabelText('Cep telefonu'), phone);
  await fireEvent.press(screen.getByRole('button', { name: 'Doğrulama kodu gönder' }));
  await fireEvent.changeText(await screen.findByLabelText('Doğrulama kodu'), '123456');
}

describe('uygulama akışı', () => {
  beforeAll(async () => {
    await i18n.changeLanguage('tr');
  });

  afterEach(async () => {
    await act(async () => {
      await signOut();
    });
  });

  it('veli: giriş → aydınlatma → her çocuk için rıza → günlük akış', async () => {
    await renderRouter('./src/app', { initialUrl: '/' });
    await waitFor(() => expect(screen).toHavePathname('/login'));

    await signInWith('5550000001');

    // Aydınlatma, rızadan ayrı ve önce gösterilir.
    await screen.findByText('Kişisel verilerinizin işlenmesi hakkında');
    expect(screen.getByText(/bilgilendirme amaçlıdır, onay \(rıza\) değildir/)).toBeOnTheScreen();
    await fireEvent.press(screen.getByRole('button', { name: 'Okudum, devam et' }));

    // Ada: baba sağlık ve fotoğraf rızası verdi, grup fotoğrafına itiraz etti (en kısıtlayıcı rıza).
    await screen.findByText(/Ada için aşağıdaki işlemlere/);
    await waitFor(() => expect(screen).toHavePathname('/consents'));
    expect(screen.getAllByText('Uygulanıyor')).toHaveLength(2);
    expect(screen.getByText('Diğer veli onaylamadığı için uygulanmıyor')).toBeOnTheScreen();
    // Hiçbir rıza seçeneği önceden işaretli gelmez.
    expect(screen.queryAllByRole('radio', { checked: true })).toHaveLength(0);

    // Hiçbir rıza vermeden devam edilebilir.
    await fireEvent.press(screen.getByRole('button', { name: 'Kaydet ve devam et' }));
    await screen.findByText(/Can için aşağıdaki işlemlere/);
    await fireEvent.press(screen.getByRole('button', { name: 'Kaydet ve devam et' }));

    await waitFor(() => expect(screen).toHavePathname('/feed'));
  });

  it('öğretmen: rıza adımı olmadan sınıfın öğün ekranına gelir', async () => {
    await renderRouter('./src/app', { initialUrl: '/' });
    await signInWith('5550000002');

    await screen.findByText('Kişisel verilerinizin işlenmesi hakkında');
    await fireEvent.press(screen.getByRole('button', { name: 'Okudum, devam et' }));

    await waitFor(() => expect(screen).toHavePathname('/meals'));
    await screen.findByText('Kelebekler (4 yaş)');
    // Velisi sağlık rızası vermeyen çocukların (Deniz, Mert) alerji bilgisi öğretmene gelmez.
    expect(screen.getAllByText('Sağlık rızası yok, alerji bilgisi görüntülenemiyor')).toHaveLength(2);
  });
});
