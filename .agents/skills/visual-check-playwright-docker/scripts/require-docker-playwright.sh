#!/usr/bin/env bash
set -euo pipefail

container_name="${1:-symphony-playwright}"

if ! command -v docker >/dev/null 2>&1; then
  echo "Docker CLI is required." >&2
  exit 1
fi

if ! docker info >/dev/null 2>&1; then
  echo "Docker daemon is not running. Start Docker Desktop and retry." >&2
  exit 1
fi

if ! docker inspect "$container_name" >/dev/null 2>&1; then
  echo "Required container '$container_name' does not exist." >&2
  exit 1
fi

running="$(docker inspect --format '{{.State.Running}}' "$container_name")"
if [[ "$running" != "true" ]]; then
  echo "Required container '$container_name' is not running." >&2
  exit 1
fi

health="$(docker inspect --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}no-healthcheck{{end}}' "$container_name")"
if [[ "$health" != "healthy" && "$health" != "no-healthcheck" ]]; then
  echo "Required container '$container_name' is not healthy: $health" >&2
  exit 1
fi

docker exec "$container_name" node -e '
const { chromium } = require("/usr/local/lib/node_modules/playwright");
(async () => {
  const browser = await chromium.connect("ws://127.0.0.1:3000/");
  await browser.close();
})().catch((error) => {
  console.error(error.message);
  process.exit(1);
});
' >/dev/null

echo "Docker Playwright is ready: $container_name"
