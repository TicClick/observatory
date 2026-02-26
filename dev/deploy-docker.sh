#!/usr/bin/env bash
set -e

GITHUB_TOKEN="${GITHUB_TOKEN?}"
GITHUB_ACTOR="${GITHUB_ACTOR?}"
REPO="${REPO?}"
IMAGE_TAG="${IMAGE_TAG:-latest}"

IMAGE="ghcr.io/${REPO}:${IMAGE_TAG}"
PROJECT_DIR="${PROJECT_DIR:-$HOME/apps/observatory}"

mkdir -p "${PROJECT_DIR}"
cd "${PROJECT_DIR}"

cat > docker-compose.yaml << EOF
services:
  observatory:
    image: ${IMAGE}
    ports:
      - "127.0.0.1:3000:3000"
    volumes:
      - ./runtime:/app/runtime
    command: ["-c", "/app/runtime/config.yaml"]
    restart: on-failure
EOF

echo "${GITHUB_TOKEN}" | docker login ghcr.io -u "${GITHUB_ACTOR}" --password-stdin

docker compose down || true
docker pull "${IMAGE}"

docker compose up -d
docker compose ps
docker compose logs --tail=30
