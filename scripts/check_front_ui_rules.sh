#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "[ui-rule-check] running mono detail alignment e2e"
npm --prefix assets/web run test:e2e:design-rules
