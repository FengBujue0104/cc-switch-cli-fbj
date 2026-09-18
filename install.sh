#!/usr/bin/env bash
# Install cc-switch from GitHub Releases. Verifies SHA-256 from checksums.txt.
set -Eeuo pipefail

REPO="${CC_SWITCH_REPO:-FengBujue0104/cc-switch-cli-fbj}"
BIN_NAME="cc-switch"
INSTALL_DIR="${CC_SWITCH_INSTALL_DIR:-$HOME/.local/bin}"
TARGET="${INSTALL_DIR}/${BIN_NAME}"
FORCE_OVERWRITE="${CC_SWITCH_FORCE:-0}"
VERSION="${1:-latest}"
[[ "${VERSION}" == "latest" || "${VERSION}" =~ ^v ]] || VERSION="v${VERSION}"

API="https://api.github.com/repos/${REPO}/releases"
RELEASES_URL="https://github.com/${REPO}/releases"
TMP_DIR=""

info() { printf '  \033[1;32minfo\033[0m: %s\n' "$*"; }
warn() { printf '  \033[1;33mwarn\033[0m: %s\n' "$*" >&2; }
err()  { printf '  \033[1;31merror\033[0m: %s\n' "$*" >&2; }

cleanup() {
  if [[ -n "${TMP_DIR}" && -d "${TMP_DIR}" ]]; then
    rm -rf "${TMP_DIR}"
  fi
}

http_get() {
  local url="$1"
  local dest="${2:-}"
  if command -v curl >/dev/null 2>&1; then
    if [[ -n "${dest}" ]]; then
      curl --fail --location --silent --show-error -A "cc-switch-install" -o "${dest}" "${url}"
    else
      curl --fail --location --silent --show-error -A "cc-switch-install" "${url}"
    fi
  elif command -v wget >/dev/null 2>&1; then
    if [[ -n "${dest}" ]]; then
      wget --quiet --user-agent="cc-switch-install" --output-document="${dest}" "${url}"
    else
      wget --quiet --user-agent="cc-switch-install" --output-document="-" "${url}"
    fi
  else
    err "Neither curl nor wget found."
    exit 1
  fi
}

file_sha256() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    err "Need sha256sum or shasum to verify the download."
    exit 1
  fi
}

os="$(uname -s 2>/dev/null || true)"
arch="$(uname -m 2>/dev/null || true)"
case "${os}" in
  Linux) ;;
  *)
    err "This script is for Linux. Detected: ${os:-unknown}"
    err "Windows: irm https://raw.githubusercontent.com/${REPO}/main/install.ps1 | iex"
    exit 1
    ;;
esac
case "${arch}" in
  x86_64|amd64) ;;
  *)
    err "This script currently ships x86_64 Linux only. Detected: ${arch}"
    err "See ${RELEASES_URL}"
    exit 1
    ;;
esac

if [[ -e "${TARGET}" && "${FORCE_OVERWRITE}" != "1" ]]; then
  if ! exec 3<> /dev/tty 2>/dev/null; then
    err "Already installed at ${TARGET}. Re-run interactively, or set CC_SWITCH_FORCE=1."
    exit 1
  fi
  printf '  Existing install: %s\n  [U]pdate or [C]ancel? [U/c] ' "${TARGET}" >&3
  IFS= read -r reply <&3 || true
  exec 3>&-
  case "${reply:-u}" in
    u|U|update|UPDATE|"") ;;
    c|C|cancel|CANCEL) info "Canceled."; exit 0 ;;
    *) err "Unrecognized choice."; exit 1 ;;
  esac
fi

if [[ "${VERSION}" == "latest" ]]; then
  release_json="$(http_get "${API}/latest")"
else
  release_json="$(http_get "${API}/tags/${VERSION}")"
fi

tag_name="$(printf '%s' "${release_json}" | grep -oE '"tag_name":[[:space:]]*"[^"]+"' | head -n 1 | sed -E 's/.*"tag_name":[[:space:]]*"([^"]+)".*/\1/')"
if [[ -z "${tag_name}" || "${tag_name}" == *'/'* || "${tag_name}" == *'..'* ]]; then
  err "Could not read a safe tag_name from ${REPO} ${VERSION}."
  exit 1
fi

asset_name="cc-switch-cli-${tag_name}-linux-x64.tar.gz"
asset_url="https://github.com/${REPO}/releases/download/${tag_name}/${asset_name}"
checksum_url="https://github.com/${REPO}/releases/download/${tag_name}/checksums.txt"

trap cleanup EXIT
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/cc-switch-install.XXXXXX")"
archive="${TMP_DIR}/${asset_name}"
checksums="${TMP_DIR}/checksums.txt"

info "Downloading ${asset_url}"
http_get "${asset_url}" "${archive}"
info "Downloading ${checksum_url}"
http_get "${checksum_url}" "${checksums}"

expected="$(awk -v name="${asset_name}" '
  $2 == name || $2 == ("*" name) { print $1; found=1 }
  END { if (!found) exit 1 }
' "${checksums}")" || {
  err "checksums.txt does not list ${asset_name}."
  exit 1
}
actual="$(file_sha256 "${archive}")"
if [[ "${actual}" != "${expected}" ]]; then
  err "Checksum mismatch for ${asset_name}."
  err "expected ${expected}"
  err "got      ${actual}"
  exit 1
fi
info "Checksum OK"

tar -xzf "${archive}" -C "${TMP_DIR}"
bin="${TMP_DIR}/${BIN_NAME}"
if [[ ! -f "${bin}" ]]; then
  err "Archive did not contain ${BIN_NAME} at the top level."
  exit 1
fi

mkdir -p "${INSTALL_DIR}"
cp "${bin}" "${TARGET}.new"
chmod 755 "${TARGET}.new"
mv -f "${TARGET}.new" "${TARGET}"
chmod 755 "${TARGET}"

info "Installed ${TARGET} (${tag_name})"
if ! command -v "${BIN_NAME}" >/dev/null 2>&1 || [[ "$(command -v "${BIN_NAME}")" != "${TARGET}" ]]; then
  case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
      warn "${INSTALL_DIR} is not in PATH. Add:"
      printf '    export PATH="%s:$PATH"\n' "${INSTALL_DIR}"
      ;;
  esac
fi
printf '  Run \033[1m%s --version\033[0m to verify.\n' "${BIN_NAME}"
