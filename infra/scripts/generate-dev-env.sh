#!/bin/sh
# infra/.env dosyasını rastgele geliştirme parolalarıyla oluşturur. Mevcut dosyanın üzerine yazmaz.
set -eu

cd "$(dirname "$0")/.."

if [ -f .env ]; then
  echo ".env zaten var; değiştirilmedi."
  exit 0
fi

rand() { LC_ALL=C tr -dc 'A-Za-z0-9' </dev/urandom | head -c "${1:-32}"; }

umask 077
sed \
  -e "s/^POSTGRES_SUPERUSER_PASSWORD=.*/POSTGRES_SUPERUSER_PASSWORD=$(rand)/" \
  -e "s/^OKULDAYIM_MIGRATOR_PASSWORD=.*/OKULDAYIM_MIGRATOR_PASSWORD=$(rand)/" \
  -e "s/^VAULT_DB_ADMIN_PASSWORD=.*/VAULT_DB_ADMIN_PASSWORD=$(rand)/" \
  -e "s/^REDIS_APP_PASSWORD=.*/REDIS_APP_PASSWORD=$(rand)/" \
  -e "s/^S3_ACCESS_KEY=.*/S3_ACCESS_KEY=okuldayim-$(rand 12)/" \
  -e "s/^S3_SECRET_KEY=.*/S3_SECRET_KEY=$(rand 40)/" \
  .env.example > .env

echo ".env oluşturuldu (izinler: 600)."
