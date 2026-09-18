#!/usr/bin/env bash
# Build local binaries and publish a GitHub Release. No GitHub Actions.
set -Eeuo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO="${CC_SWITCH_REPO:-FengBujue0104/cc-switch-cli-fbj}"
TAG="${1:-}"
if [[ -z "${TAG}" ]]; then
  echo "usage: $0 <tag>   example: $0 v5.10.6" >&2
  exit 1
fi
[[ "${TAG}" =~ ^v ]] || TAG="v${TAG}"

cd "${ROOT}"
GH="${GH:-gh}"
if ! command -v gh >/dev/null 2>&1; then
  GH="/mnt/c/Program Files/GitHub CLI/gh.exe"
fi
if [[ ! -x "${GH}" ]] && ! command -v "${GH}" >/dev/null 2>&1; then
  echo "gh is required to upload the release." >&2
  exit 1
fi

WIN_CARGO="${WIN_CARGO:-/mnt/c/Users/zcy/.cargo/bin/cargo.exe}"
need_windows="${PUBLISH_WINDOWS:-1}"

OUT="${ROOT}/dist-release"
rm -rf "${OUT}"
mkdir -p "${OUT}/linux"

echo "==> Linux musl static binary"
(
  cd "${ROOT}/src-tauri"
  export CC_x86_64_unknown_linux_musl="${CC_x86_64_unknown_linux_musl:-musl-gcc}"
  export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER="${CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER:-musl-gcc}"
  cargo build --release --target x86_64-unknown-linux-musl
)
cp "${ROOT}/src-tauri/target/x86_64-unknown-linux-musl/release/cc-switch" "${OUT}/linux/cc-switch"
chmod 755 "${OUT}/linux/cc-switch"
tar -czf "${OUT}/cc-switch-cli-${TAG}-linux-x64.tar.gz" -C "${OUT}/linux" cc-switch

if [[ "${need_windows}" == "1" ]]; then
  if command -v cargo.exe >/dev/null 2>&1; then
    WIN_CARGO="$(command -v cargo.exe)"
  elif [[ ! -x "${WIN_CARGO}" ]]; then
    echo "Windows cargo not found; set WIN_CARGO or PUBLISH_WINDOWS=0." >&2
    exit 1
  fi
  echo "==> Windows release binary"
  (
    cd "${ROOT}/src-tauri"
    RUSTUP_TOOLCHAIN=stable "${WIN_CARGO}" build --release --target x86_64-pc-windows-msvc
  )
  WIN_EXE="${ROOT}/src-tauri/target/x86_64-pc-windows-msvc/release/cc-switch.exe"
  if [[ ! -f "${WIN_EXE}" ]]; then
    echo "Windows build did not produce ${WIN_EXE}" >&2
    exit 1
  fi
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
  sha256sum cc-switch-cli-"${TAG}"-linux-x64.tar.gz > checksums.txt
  if [[ -f cc-switch-cli-"${TAG}"-windows-x64.zip ]]; then
    sha256sum cc-switch-cli-"${TAG}"-windows-x64.zip >> checksums.txt
  fi
  sha256sum install.sh install.ps1 >> checksums.txt
)

NOTES="${OUT}/notes.md"
cat > "${NOTES}" <<EOF
cc-switch ${TAG}

Published with scripts/publish-release.sh (no GitHub Actions).
Linux x86_64 is a static musl binary.

Linux:
  curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install.sh | bash

Windows:
  irm https://raw.githubusercontent.com/${REPO}/main/install.ps1 | iex
EOF

assets=(
  "${OUT}/cc-switch-cli-${TAG}-linux-x64.tar.gz"
  "${OUT}/install.sh"
  "${OUT}/install.ps1"
  "${OUT}/checksums.txt"
)
if [[ -f "${OUT}/cc-switch-cli-${TAG}-windows-x64.zip" ]]; then
  assets+=("${OUT}/cc-switch-cli-${TAG}-windows-x64.zip")
fi

echo "==> GitHub release ${TAG}"
if "${GH}" release view "${TAG}" --repo "${REPO}" >/dev/null 2>&1; then
  "${GH}" release upload "${TAG}" --repo "${REPO}" --clobber "${assets[@]}"
else
  "${GH}" release create "${TAG}" --repo "${REPO}" --title "CC Switch CLI ${TAG}" --notes-file "${NOTES}" "${assets[@]}"
fi

echo "Published ${TAG}"
echo "https://github.com/${REPO}/releases/tag/${TAG}"
