# Okuldayım

Kreş, anaokulu ve ilkokul için güvenli okul–veli iletişim uygulaması.

| Klasör / dosya | İçerik |
| --- | --- |
| [Ürün gereksinimleri](school-parent-communication-requirements.md) | Sürüm 1.3: KVKK revizyonu ve backend/sistemlerle birleşik rafinasyon; değişiklikler Ek A ve Ek B'de |
| [Backend ihtiyaç analizi](docs/backend-requirements-analysis.md) | Sürüm 1.2: Rust backend tasarımı, teslim durum makinesi, eksik MVP sözleşmeleri ve uygulanacak kabul koşulları; değişiklikler §14'te |
| [Sistem ihtiyaç analizleri](docs/systems/README.md) | Sürüm 1.1: 15 ayrı sistem analizi; mevcut durum, bağımlılıklar, ortak süreler ve C01–C13 kabul matrisi |
| `docs/archive/` | Gereksinim analizinin orijinal sürümü (1.0) |
| `infra/` | Yerel geliştirme altyapısı (Docker Compose): PostgreSQL, Redis, Vault, S3 uyumlu depolama, Mailpit |
| `mobile/` | React Native (Expo SDK 58) mobil uygulama |

## Hızlı başlangıç

```sh
# Altyapı (backend geliştirmesi için)
cd infra && ./scripts/generate-dev-env.sh && docker compose up -d

# Mobil uygulama (mock sunucuyla, backend gerekmez)
cd mobile && npm install && npm run ios
```

Ayrıntılar: [`infra/README.md`](infra/README.md), [`mobile/README.md`](mobile/README.md).

Rust backend henüz uygulanmadı; mobil uygulama mock kullanır. Compose beş geliştirme servisi ve başlangıç işlerini tanımlar. Üretim HA, izleme, yedekleme ve dış sağlayıcı kurulumları [sistem analizlerinde](docs/systems/README.md) ayrı gereksinimlerdir. Analizlerdeki backend çalıştırma/test komutları iskelet ve test harness'i tamamlandıktan sonra kullanılabilir.
