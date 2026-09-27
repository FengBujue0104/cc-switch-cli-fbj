#!/usr/bin/env bash
# Build local binaries and publish a GitHub Release. No GitHub Actions.
#
# Windows zip is always x86_64-pc-windows-msvc → cc-switch-cli-${TAG}-windows-x64.zip
# with zip entry cc-switch.exe.
#
# Env:
#   PUBLISH_LINUX=1|0              default 1. Skip the musl Linux tarball when 0.
#   PUBLISH_WINDOWS=1|0            default 1. Skip the Windows zip when 0.
#   PUBLISH_UPLOAD=1|0             default 1. Skip gh release create/upload when 0.
#   CC_SWITCH_WIN_BUILDER=xwin|cargo.exe
#                                  default auto: cargo-xwin when present, else
#                                  WIN_CARGO / cargo.exe (WSL).
#   WIN_CARGO                      cargo.exe path for the WSL/Windows fallback.
#   GH                             gh binary (or gh.exe under WSL).
#
# Linux host Windows path (cargo-xwin):
#   rustup target add x86_64-pc-windows-msvc --toolchain 1.91.1
#   cargo install --locked cargo-xwin
#   clang, lld, llvm, and clang-cl on PATH
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

need_linux="${PUBLISH_LINUX:-1}"
need_windows="${PUBLISH_WINDOWS:-1}"
need_upload="${PUBLISH_UPLOAD:-1}"

if [[ "${need_linux}" != "1" && "${need_windows}" != "1" ]]; then
  echo "Nothing to build; set PUBLISH_LINUX=1 and/or PUBLISH_WINDOWS=1." >&2
  exit 1
fi

if [[ "${need_upload}" == "1" ]]; then
  GH="${GH:-gh}"
  if ! command -v gh >/dev/null 2>&1; then
    GH="/mnt/c/Program Files/GitHub CLI/gh.exe"
  fi
  if [[ ! -x "${GH}" ]] && ! command -v "${GH}" >/dev/null 2>&1; then
    echo "gh is required to upload the release." >&2
    exit 1
  fi
fi

WIN_CARGO="${WIN_CARGO:-}"

OUT="${ROOT}/dist-release"
rm -rf "${OUT}"
mkdir -p "${OUT}"

if [[ "${need_linux}" == "1" ]]; then
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
fi

resolve_win_cargo() {
  if [[ -z "${WIN_CARGO}" ]] && command -v cargo.exe >/dev/null 2>&1; then
    WIN_CARGO="$(command -v cargo.exe)"
  fi
  if [[ -z "${WIN_CARGO}" ]]; then
    WIN_CARGO="cargo.exe"
  fi
  if ! command -v "${WIN_CARGO}" >/dev/null 2>&1 && [[ ! -x "${WIN_CARGO}" ]]; then
    return 1
  fi
  return 0
}

select_windows_builder() {
  local requested="${CC_SWITCH_WIN_BUILDER:-}"
  case "${requested}" in
    xwin)
      if ! command -v cargo-xwin >/dev/null 2>&1; then
        echo "CC_SWITCH_WIN_BUILDER=xwin but cargo-xwin is not on PATH." >&2
        exit 1
      fi
      win_builder=xwin
      ;;
    cargo.exe)
      if ! resolve_win_cargo; then
        echo "CC_SWITCH_WIN_BUILDER=cargo.exe but Windows cargo was not found; set WIN_CARGO or PUBLISH_WINDOWS=0." >&2
        exit 1
      fi
      win_builder=cargo.exe
      ;;
    "")
      if command -v cargo-xwin >/dev/null 2>&1; then
        win_builder=xwin
      elif resolve_win_cargo; then
        win_builder=cargo.exe
      else
        echo "Windows builder not found; install cargo-xwin, set WIN_CARGO, or PUBLISH_WINDOWS=0." >&2
        exit 1
      fi
      ;;
    *)
      echo "Unknown CC_SWITCH_WIN_BUILDER='${requested}'. Expected xwin or cargo.exe." >&2
      exit 1
      ;;
  esac
}

if [[ "${need_windows}" == "1" ]]; then
  win_builder=""
  select_windows_builder
  echo "==> Windows release binary (${win_builder})"
  (
    cd "${ROOT}/src-tauri"
    export RUSTUP_TOOLCHAIN=1.91.1
    if [[ "${win_builder}" == "xwin" ]]; then
      echo "cargo xwin build --release --target x86_64-pc-windows-msvc"
      cargo xwin build --release --target x86_64-pc-windows-msvc
    else
      echo "${WIN_CARGO} build --release --target x86_64-pc-windows-msvc"
      "${WIN_CARGO}" build --release --target x86_64-pc-windows-msvc
    fi
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
  : > checksums.txt
  if [[ -f cc-switch-cli-"${TAG}"-linux-x64.tar.gz ]]; then
    sha256sum cc-switch-cli-"${TAG}"-linux-x64.tar.gz >> checksums.txt
  fi
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

if [[ "${need_upload}" != "1" ]]; then
  echo "PUBLISH_UPLOAD=0: skipped GitHub upload"
  echo "Packaged ${TAG} in ${OUT}"
  exit 0
fi

assets=(
  "${OUT}/install.sh"
  "${OUT}/install.ps1"
  "${OUT}/checksums.txt"
)
if [[ -f "${OUT}/cc-switch-cli-${TAG}-linux-x64.tar.gz" ]]; then
  assets+=("${OUT}/cc-switch-cli-${TAG}-linux-x64.tar.gz")
fi
if [[ -f "${OUT}/cc-switch-cli-${TAG}-windows-x64.zip" ]]; then
  assets+=("${OUT}/cc-switch-cli-${TAG}-windows-x64.zip")
fi

# Windows gh.exe cannot read WSL /mnt paths; convert when needed.
gh_path() {
  local p="$1"
  if [[ "${GH}" == *.exe || "${GH}" == *.EXE ]] && command -v wslpath >/dev/null 2>&1; then
    wslpath -w "$p"
  else
    printf '%s' "$p"
  fi
}

gh_assets=()
for asset in "${assets[@]}"; do
  gh_assets+=("$(gh_path "${asset}")")
done

echo "==> GitHub release ${TAG}"
if "${GH}" release view "${TAG}" --repo "${REPO}" >/dev/null 2>&1; then
  "${GH}" release upload "${TAG}" --repo "${REPO}" --clobber "${gh_assets[@]}"
else
  "${GH}" release create "${TAG}" --repo "${REPO}" --title "CC Switch CLI ${TAG}" --notes-file "$(gh_path "${NOTES}")" "${gh_assets[@]}"
fi

echo "Published ${TAG}"
echo "https://github.com/${REPO}/releases/tag/${TAG}"
