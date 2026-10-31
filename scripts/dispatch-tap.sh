#!/usr/bin/env bash
set -euo pipefail

tag="${1:?release tag is required}"
: "${TAP_AUTOMATION_TOKEN:?TAP_AUTOMATION_TOKEN must be configured in the protected tap-dispatch environment}"

GH_TOKEN="$TAP_AUTOMATION_TOKEN" gh workflow run publish-skills-manager.yml \
  --repo jboczek/homebrew-tap \
  --ref main \
  --field "tag=$tag"
echo "dispatched tap publication for $tag"
