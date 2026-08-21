#!/usr/bin/env bash
# build-artifacts.sh — raccpack release artifacts (mirror of MVP 0.1.0 set).
#
# Builds inside Docker (rust:1.85-bookworm = MSRV), cross-compiles aarch64,
# packs portable .tar.zst + deb/rpm/archlinux via nFPM, writes SHA256SUMS.
#
# Usage: bash dist/build-artifacts.sh          (from repo root)
# Requires: docker; zstd on host for tarballs.
set -euo pipefail

VERSION="0.3.0"
OUT="dist/out"
IMAGE="rust:1.85-bookworm"

rm -rf "$OUT"
mkdir -p "$OUT"

CARGO_ENV=(
  -e CARGO_TARGET_DIR=/tmp/target
  -e CARGO_NET_RETRY=5
  -e RUSTUP_TOOLCHAIN=1.85
)
CACHE_VOLS=(
  -v racc-cargo-registry:/usr/local/cargo/registry
  -v racc-cargo-git:/usr/local/cargo/git
)

echo "==> [1/5] x86_64-unknown-linux-gnu ($IMAGE)"
docker run --rm -v "$PWD":/src -w /src "${CACHE_VOLS[@]}" "${CARGO_ENV[@]}" "$IMAGE" \
  bash -c 'export PATH=/usr/local/cargo/bin:$PATH && cargo build --release -p raccpack-cli &&
            cp /tmp/target/release/racc /src/dist/out/racc-x86_64 &&
            chmod 755 /src/dist/out/racc-x86_64'

echo "==> [2/5] aarch64-unknown-linux-gnu (cross in $IMAGE)"
docker run --rm -v "$PWD":/src -w /src "${CACHE_VOLS[@]}" "${CARGO_ENV[@]}" "$IMAGE" \
  bash -c 'export PATH=/usr/local/cargo/bin:$PATH && apt-get update -qq && apt-get install -y -qq gcc-aarch64-linux-gnu >/dev/null &&
            rustup target add aarch64-unknown-linux-gnu &&
            export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
                   CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc \
                   AR_aarch64_unknown_linux_gnu=aarch64-linux-gnu-ar &&
            cargo build --release -p raccpack-cli --target aarch64-unknown-linux-gnu &&
            cp /tmp/target/aarch64-unknown-linux-gnu/release/racc /src/dist/out/racc-aarch64 &&
            chmod 755 /src/dist/out/racc-aarch64'

echo "==> [3/5] portable tar.zst"
for pair in "x86_64:racc-x86_64" "aarch64:racc-aarch64"; do
  arch="${pair%%:*}"; bin="${pair##*:}"
  stage=$(mktemp -d)
  install -m 0755 "$OUT/$bin" "$stage/racc"
  cp LICENSE-MIT LICENSE-APACHE "$stage/"
  tar -C "$stage" -cf - racc LICENSE-MIT LICENSE-APACHE \
    | zstd -19 -q -o "$OUT/raccpack-${VERSION}-linux-${arch}.tar.zst"
  rm -rf "$stage"
done

echo "==> [4/5] nFPM packages (deb/rpm/archlinux) from x86_64 binary"
for pkg in deb rpm archlinux; do
  docker run --rm -v "$PWD":/work -w /work goreleaser/nfpm package \
    --config dist/nfpm.yaml --packager "$pkg" --target "$OUT"
done
# normalize to MVP-style names
mv -f "$OUT/raccpack_${VERSION}-1_amd64.deb"      "$OUT/raccpack-${VERSION}-1-x86_64.deb"      2>/dev/null || true
mv -f "$OUT/raccpack-0.3.0-1.x86_64.rpm"          "$OUT/raccpack-${VERSION}-1.x86_64.rpm"      2>/dev/null || true
mv -f "$OUT/raccpack-0.3.0-1-x86_64.pkg.tar.zst"  "$OUT/raccpack-${VERSION}-1-x86_64.pkg.tar.zst" 2>/dev/null || true

echo "==> [5/5] SHA256SUMS"
(
  cd "$OUT"
  sha256sum \
    "raccpack-${VERSION}-linux-x86_64.tar.zst" \
    "raccpack-${VERSION}-linux-aarch64.tar.zst" \
    "raccpack-${VERSION}-1-x86_64.deb" \
    "raccpack-${VERSION}-1.x86_64.rpm" \
    "raccpack-${VERSION}-1-x86_64.pkg.tar.zst" \
    > SHA256SUMS
)

echo "==> done:"; ls -la "$OUT"
