# Okuldayım

Kreş, anaokulu ve ilkokul için güvenli okul–veli iletişim uygulaması.

| Klasör / dosya | İçerik |
| --- | --- |
| [Ürün gereksinimleri](school-parent-communication-requirements.md) | Sürüm 1.3: KVKK revizyonu ve backend/sistemlerle birleşik rafinasyon; değişiklikler Ek A ve Ek B'de |
| [Backend ihtiyaç analizi](docs/backend-requirements-analysis.md) | Sürüm 1.2: Rust backend tasarımı, teslim durum makinesi, eksik MVP sözleşmeleri ve uygulanacak kabul koşulları; değişiklikler §14'te |
| [Sistem ihtiyaç analizleri](docs/systems/README.md) | Sürüm 1.2: 15 ayrı sistem analizi; mevcut durum, bağımlılıklar, ortak süreler ve C01–C13 kabul matrisi |
| `docs/archive/` | Gereksinim analizinin orijinal sürümü (1.0) |
| `backend/` | Rust API runtime'ı; Vault, PostgreSQL ve Redis entegrasyonları; ürün uç noktaları henüz tamamlanmadı |
| `infra/` | Yerel geliştirme altyapısı (Docker Compose): PostgreSQL, Redis, Vault, API, S3 uyumlu depolama ve Mailpit |
| `mobile/` | React Native (Expo SDK 58) mobil uygulama |

## Hızlı başlangıç

```sh
# Altyapı (backend geliştirmesi için)
cd infra && ./scripts/generate-dev-env.sh && docker compose up -d --build

# Mobil uygulama (mock sunucuyla, backend gerekmez)
cd mobile && npm install && npm run ios
```

Ayrıntılar: [`backend/README.md`](backend/README.md), [`infra/README.md`](infra/README.md), [`mobile/README.md`](mobile/README.md).

Backend'de şimdilik Vault AppRole/DB lease ve pool rotasyonu, PostgreSQL tenant transaction temeli, Redis readiness kontrolü ve `/healthz`, `/readyz`, `/openapi.json` operasyon uçları vardır. Ürün API akışları tamamlanana kadar mobil uygulama mock kullanır. Compose yerel servisleri ve başlangıç işlerini tanımlar; üretim HA, izleme, yedekleme ve dış sağlayıcı kurulumları [sistem analizlerinde](docs/systems/README.md) ayrı gereksinimlerdir.
