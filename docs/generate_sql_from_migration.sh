#!/usr/bin/env bash
set -euo pipefail

# ─── CONFIG ────────────────────────────────────────────────────────────────
# Generates the latest DBM (pgModeler) and SQL init script from playing through all sea-orm migrations
CONTAINER="pg_migrate_tmp"
IMAGE="postgres:15-alpine"
DB_USER="postgres"
DB_PASS="postgres"
DB_NAME="postgres"
HOST_PORT=54321
TMPFS_SIZE="200m"
# ───────────────────────────────────────────────────────────────────────────────

cleanup() {
  docker rm -f "${CONTAINER}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

# 1) Start in-memory Postgres
echo "▶ Starting Postgres container…"
docker run -d --name "${CONTAINER}" \
  -e POSTGRES_USER="${DB_USER}" \
  -e POSTGRES_PASSWORD="${DB_PASS}" \
  -e POSTGRES_DB="${DB_NAME}" \
  --tmpfs /var/lib/postgresql/data:rw,size=${TMPFS_SIZE} \
  -p "${HOST_PORT}":5432 "${IMAGE}" >/dev/null

# 2) Wait for Postgres
echo -n "⏳ Waiting for Postgres… "
for i in {1..30}; do
  if docker exec "${CONTAINER}" pg_isready -U "${DB_USER}" &>/dev/null; then
    echo " ready"
    break
  fi
  echo -n "."; sleep 1
done

# 3) Apply SeaORM (Rust) migrations from ../migrations
echo "▶ Applying SeaORM migrations…"
DSN="postgres://${DB_USER}:${DB_PASS}@localhost:${HOST_PORT}/${DB_NAME}?sslmode=disable"
sea-orm-cli migrate fresh \
  -d ../migration \
  -u "${DSN}"

# 4) Import into pgModeler DBM
echo "▶ Importing schema into spice.dbm…"
pgmodeler-cli --platform offscreen --import-db \
  --input-db "${DB_NAME}" --host localhost --port "${HOST_PORT}" \
  --user "${DB_USER}" --passwd "${DB_PASS}" --output spice.dbm

# 5) Dump schema-only SQL
echo "▶ Dumping schema-only SQL to spice.sql…"
docker exec "${CONTAINER}" pg_dump \
  --username="${DB_USER}" --schema-only "${DB_NAME}" \
  > spice.sql

echo "✔ Done: spice.dbm, spice.sql"
