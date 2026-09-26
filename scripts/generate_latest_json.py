#!/usr/bin/env python3
"""This fork does not ship latest.json.

Release publishing uses scripts/publish-release.sh, which uploads tagged
linux-x64 / windows-x64 archives plus checksums.txt. The updater resolves
GitHub Releases and verifies SHA-256 from checksums.txt (or the GitHub asset
digest). Do not generate a minisign latest.json with upstream asset names.
"""

import sys


def main() -> int:
    print(
        "generate_latest_json.py is disabled in this fork. "
        "Publish with scripts/publish-release.sh (checksums.txt only).",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
