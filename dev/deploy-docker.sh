#!/usr/bin/env bash
set -e

GITHUB_TOKEN="${GITHUB_TOKEN?}"
GITHUB_ACTOR="${GITHUB_ACTOR?}"
REPO="${REPO?}"
IMAGE_TAG="${IMAGE_TAG:-latest}"

IMAGE="ghcr.io/${REPO}:${IMAGE_TAG}"
PROJECT_DIR="${PROJECT_DIR:-$HOME/observatory}"

cd "${PROJECT_DIR}"
git fetch --all
git reset --hard origin/master

echo "${GITHUB_TOKEN}" | docker login ghcr.io -u "${GITHUB_ACTOR}" --password-stdin

docker compose down || true
docker pull "${IMAGE}"

cat > docker-compose.override.yaml << EOF
services:
  observatory:
    image: ${IMAGE}
EOF

docker compose up -d
docker compose ps
docker compose logs --tail=30