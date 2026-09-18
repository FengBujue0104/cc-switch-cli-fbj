#!/usr/bin/env bash
# Build local binaries and publish a GitHub Release. No GitHub Actions.
set -Eeuo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO="${CC_SWITCH_REPO:-FengBujue0104/cc-switch-cli-fbj}"
TAG="${1:-}"
if [[ -z "${TAG}" ]]; then
  echo "usage: $0 <tag>   example: $0 v5.10.5-fbj.2" >&2
  exit 1
fi
[[ "${TAG}" =~ ^v ]] || TAG="v${TAG}"

cd "${ROOT}"
if ! command -v gh >/dev/null 2>&1 && [[ ! -x "/mnt/c/Program Files/GitHub CLI/gh.exe" ]]; then
  echo "gh is required to upload the release." >&2
  exit 1
fi
GH="${GH:-gh}"
if ! command -v gh >/dev/null 2>&1; then
  GH="/mnt/c/Program Files/GitHub CLI/gh.exe"
fi

need_linux=1
need_windows="${PUBLISH_WINDOWS:-1}"

OUT="${ROOT}/dist-release"
rm -rf "${OUT}"
mkdir -p "${OUT}/linux"

echo "==> Linux release binary"
(
  cd "${ROOT}/src-tauri"
  cargo build --release
)
cp "${ROOT}/src-tauri/target/release/cc-switch" "${OUT}/linux/cc-switch"
chmod 755 "${OUT}/linux/cc-switch"
tar -czf "${OUT}/cc-switch-cli-${TAG}-linux-x64.tar.gz" -C "${OUT}/linux" cc-switch

if [[ "${need_windows}" == "1" ]] && command -v cargo.exe >/dev/null 2>&1 || [[ -x /mnt/c/Users/zcy/.cargo/bin/cargo.exe ]]; then
  echo "==> Windows release binary"
  WIN_CARGO="${WIN_CARGO:-/mnt/c/Users/zcy/.cargo/bin/cargo.exe}"
  (
    cd "${ROOT}/src-tauri"
    RUSTUP_TOOLCHAIN=stable "${WIN_CARGO}" build --release --target x86_64-pc-windows-msvc
  )
  WIN_EXE="${ROOT}/src-tauri/target/x86_64-pc-windows-msvc/release/cc-switch.exe"
  python3 - "${WIN_EXE}" "${OUT}/cc-switch-cli-${TAG}-windows-x64.zip" <<'PY'
import sys, zipfile, pathlib
exe, out = map(pathlib.Path, sys.argv[1:])
with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
    z.write(exe, "cc-switch.exe")
print("wrote", out)
PY
fi

cp "${ROOT}/install.sh" "${OUT}/install.sh"
cp "${ROOT}/install.ps1" "${OUT}/install.ps1"
(
  cd "${OUT}"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum cc-switch-cli-${TAG}-*.tar.gz cc-switch-cli-${TAG}-*.zip install.sh install.ps1 > checksums.txt 2>/dev/null || true
  fi
)

NOTES="${OUT}/notes.md"
cat > "${NOTES}" <<EOF
cc-switch ${TAG}

Published from a local machine with scripts/publish-release.sh (no GitHub Actions).

Linux:
  curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install.sh | bash

Windows:
  irm https://raw.githubusercontent.com/${REPO}/main/install.ps1 | iex
EOF

echo "==> GitHub release ${TAG}"
if "${GH}" release view "${TAG}" --repo "${REPO}" >/dev/null 2>&1; then
  "${GH}" release upload "${TAG}" --repo "${REPO}" --clobber \
    "${OUT}/cc-switch-cli-${TAG}-linux-x64.tar.gz" \
    "${OUT}/install.sh" \
    ${OUT}/cc-switch-cli-${TAG}-windows-x64.zip \
    "${OUT}/install.ps1" \
    "${OUT}/checksums.txt" || true
else
  "${GH}" release create "${TAG}" --repo "${REPO}" --title "CC Switch CLI ${TAG}" --notes-file "${NOTES}" \
    "${OUT}/cc-switch-cli-${TAG}-linux-x64.tar.gz" \
    "${OUT}/install.sh" \
    ${OUT}/cc-switch-cli-${TAG}-windows-x64.zip \
    "${OUT}/install.ps1" \
    "${OUT}/checksums.txt"
fi

echo "Published ${TAG}"
echo "https://github.com/${REPO}/releases/tag/${TAG}"
