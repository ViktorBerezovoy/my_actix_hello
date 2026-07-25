#!/usr/bin/env bash
set -x
set -eo pipefail

if ! [ -x "$(command -v sqlx)" ]; then
    echo >&2 "Error: `sqlx` is not installed."
    echo >&2 "Use:"
    echo >&2 "cargo install sqlx-cli --no-default-features --features native-tls,postgres"
    echo >&2 "to install it."
    exit 1
fi

DB_USER=${POSTGRES_USER:=postgres}
DB_PASSWORD="${POSTGRES_PASSWORD:=password}"
DB_NAME="${POSTGRES_DB:=newsletter}"
DB_PORT="${POSTGRES_PORT:=5432}"

# Добавляем флаг --name newsletter_db

if [[ -z "${SKIP_PODMAN}" ]]
then
podman run \
  --name newsletter_db \
  -e POSTGRES_USER=${DB_USER} \
  -e POSTGRES_PASSWORD=${DB_PASSWORD} \
  -e POSTGRES_DB=${DB_NAME} \
  -p "${DB_PORT}":5432 \
  -d docker.io/library/postgres \
  postgres -N 1000
fi

# Выполняем проверку внутри самого контейнера через podman exec
until podman exec newsletter_db pg_isready -h "localhost" -U "${DB_USER}" -p "5432"; do
  >&2 echo "Postgres is still unavailable - sleeping"
  sleep 1
done

>&2 echo "Postgres is up and running on port ${DB_PORT}!"

export DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@localhost:${DB_PORT}/${DB_NAME}
sqlx database create
sqlx migrate run
