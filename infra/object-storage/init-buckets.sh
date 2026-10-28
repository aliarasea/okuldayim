#!/bin/sh
# Medya bucket'larını ve yaşam döngüsü kurallarını oluşturur. İdempotenttir.
#
#   okuldayim-media-quarantine  Mobil uygulamanın imzalı URL ile yüklediği ham dosyalar. EXIF silme,
#                               bulanıklaştırma ve tarama bitmeden yayınlanmaz. Kesin silmeyi worker yapar;
#                               1 günlük lifecycle asenkron güvenlik ağıdır.
#   okuldayim-media             İşlenmiş, yayınlanan medya. İstemciye ham S3 GET imzası verilmez; okuma her
#                               istekte yetki denetleyen medya geçidinden yapılır.
#   okuldayim-exports           KVKK-06 veri dökümü paketleri; 24 saatlik silmeyi worker yapar, 1 günlük
#                               lifecycle güvenlik ağıdır.
set -eu

mkdir -p "$HOME/.aws"
printf '[default]\ns3 =\n    addressing_style = path\n' > "$HOME/.aws/config"

s3api() { aws --endpoint-url "$S3_ENDPOINT" s3api "$@"; }

for bucket in okuldayim-media-quarantine okuldayim-media okuldayim-exports; do
  if s3api head-bucket --bucket "$bucket" >/dev/null 2>&1; then
    echo "[storage-init] $bucket zaten var"
  else
    s3api create-bucket --bucket "$bucket" >/dev/null
    echo "[storage-init] $bucket oluşturuldu"
  fi
done

for bucket in okuldayim-media-quarantine okuldayim-exports; do
  s3api put-bucket-lifecycle-configuration \
    --bucket "$bucket" \
    --lifecycle-configuration file:///scripts/lifecycle-1-day.json \
    || echo "[storage-init] UYARI: $bucket için yaşam döngüsü kuralı uygulanamadı"
done

echo "[storage-init] Hazır"
