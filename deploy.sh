#!/usr/bin/env bash

GITHUB_TOKEN="${GITHUB_TOKEN?}"
REPO="${GITHUB_REPO?}"
TAG="${GITHUB_TAG?}"

ARTIFACT_NAME="observatory-x86_64-unknown-linux-gnu.tar.gz"
SYSTEMD_SERVICE="observatory"
BINARY_NAME="observatory"

./${BINARY_NAME} --version
systemctl --user status ${SYSTEMD_SERVICE}

asset_url=$(
    curl -L \
        -H "Authorization: Bearer ${GITHUB_TOKEN}" \
        -H "Accept: application/vnd.github+json" \
        "https://api.github.com/repos/${REPO}/releases/tags/${TAG}" \
        | jq -r ".assets | .[] | select(.name==\"$ARTIFACT_NAME\") | .url"
)
echo "asset URL: ${asset_url}"
curl -L \
    -H "Authorization: token ${GITHUB_TOKEN}" \
    -H "Accept: application/octet-stream" \
    "${asset_url}" \
    -o "${ARTIFACT_NAME}" || exit 1

tar --extract --file ${ARTIFACT_NAME} ${BINARY_NAME} && \
    rm ${ARTIFACT_NAME} && \
    systemctl --user restart ${SYSTEMD_SERVICE} || exit 1

systemctl --user status ${SYSTEMD_SERVICE} || exit 1
