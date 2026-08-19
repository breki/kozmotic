#!/usr/bin/env bash
#
# Write a SHA256SUMS over the release assets in a directory.
#
# Covers both the archives and the binary inside each one:
# downstream provisioning scripts pin the digest of the
# extracted binary, not of the archive it came in, so a file
# listing only archives would leave their pin unverifiable.
#
# Run by .github/workflows/release.yml, and runnable by hand
# against a directory of downloaded release assets:
#
#   scripts/checksum-assets.sh ./artifacts
#
set -euo pipefail

dir=${1:-}
if [ -z "$dir" ] || [ ! -d "$dir" ]; then
  echo "usage: $0 <directory of release assets>" >&2
  exit 2
fi

# Unmatched globs expand to nothing rather than to
# themselves, so a dropped build target cannot hand the
# literal string "kozmotic-*.zip" to unzip and fail the
# release with a confusing error.
shopt -s nullglob
cd "$dir"

# Globbed on the "kozmotic-" prefix rather than "*", so a
# SHA256SUMS left behind by an earlier run is not
# checksummed into the new one. Regular files only: a run
# that failed partway leaves its extracted kozmotic-*/ trees
# behind, and hashing a directory only fails with "Is a
# directory".
archives=()
for entry in kozmotic-*; do
  [ -f "$entry" ] && archives+=("$entry")
done
if [ "${#archives[@]}" -eq 0 ]; then
  echo "::error::no release archives in $dir; nothing to checksum"
  exit 1
fi
sha256sum "${archives[@]}" > SHA256SUMS

# Each archive is unpacked on its own and must yield
# exactly one binary. Counting across all of them instead
# would let an archive shipping two entries cover for one
# shipping none, and SHA256SUMS would omit a platform while
# looking complete.
#
# Each archive unpacks to a directory named exactly as the
# archive is, so `sha256sum -c SHA256SUMS --ignore-missing`
# verifies the archives before extraction and the binaries
# after it, from this one file.
for archive in "${archives[@]}"; do
  case "$archive" in
    *.tar.gz) tar -xzf "$archive"; dir="${archive%.tar.gz}" ;;
    *.zip)    unzip -q -o "$archive"; dir="${archive%.zip}" ;;
    *)
      echo "::error::$archive is neither .tar.gz nor .zip"
      exit 1
      ;;
  esac
  # "$dir" rather than a glob: an empty glob would leave
  # find with no starting point, and it would silently
  # search the whole directory instead.
  mapfile -d "" -t found < <(
    find "$dir" -type f \
      \( -name kozmotic -o -name kozmotic.exe \) -print0 |
      sort -z
  )
  if [ "${#found[@]}" -ne 1 ]; then
    echo "::error::$archive holds ${#found[@]} binaries," \
         "expected exactly one"
    exit 1
  fi
  sha256sum "${found[0]}" >> SHA256SUMS
done

# Delete the unpacked trees: the publish step passes
# artifacts/* to gh, which would upload each directory.
rm -rf kozmotic-*/
cat SHA256SUMS
