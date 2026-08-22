#!/usr/bin/env bash
set -x
set -eo pipefail

CONTAINER_NAME="valkey"

# Проверяем, запущен ли уже контейнер
RUNNING_CONTAINER=$(podman ps --filter "name=${CONTAINER_NAME}" --format '{{.ID}}')
if [[ -n $RUNNING_CONTAINER ]]; then
  echo >&2 "Valkey is already running."
  exit 0
fi

# Проверяем, существует ли остановленный контейнер с таким именем
STOPPED_CONTAINER=$(podman ps -a --filter "name=${CONTAINER_NAME}" --format '{{.ID}}')
if [[ -n $STOPPED_CONTAINER ]]; then
  echo >&2 "Starting existing Valkey container..."
  podman start ${CONTAINER_NAME}
  exit 0
fi

# Если контейнера нет, создаем новый
echo >&2 "Creating and starting a new Valkey container..."
podman run \
  -p "6379:6379" \
  -d \
  --name "${CONTAINER_NAME}" \
  docker.io/valkey/valkey:9.1.1-alpine

>&2 echo "Valkey is ready to go!"
