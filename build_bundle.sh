#!/usr/bin/env bash
# build_bundle.sh — bundle EXTRACTIBLE d'un module Kubuno pour l'installation à
# l'exécution par le CORE (marketplace). Produit une archive PLATE
#   <id>/{module.toml, kubuno-<id>[.exe], frontend/, config.toml.example}
# que le core télécharge SELON SON OS/ARCH, vérifie (SHA-256) et extrait dans son
# store (`modules_install_dir`). Distinct des installeurs manuels .deb/.exe/.pkg.
#
# Nommage attendu par le core (résolveur `os_artifact_suffixes`) :
#   dist/kubuno-<id>-<version>-<os>-<arch>.<zip|tar.gz>
#     os   ∈ {linux, windows, macos}
#     arch : windows→x64/arm64 · macos→arm64/x86_64 · linux→x86_64/aarch64
#   → zip pour Windows (extrait par le core sans outil, crate `zip`), tar.gz sinon.
#
# Ce script est GÉNÉRIQUE (id lu dans module.toml) : copiable tel quel dans chaque
# module (`bash _tools/forall.sh cp <path>/build_bundle.sh .`).
#
# Usage :
#   bash build_bundle.sh                                   # auto-détecte l'hôte
#   TARGET=x86_64-pc-windows-msvc bash build_bundle.sh     # CI Windows (runner)
#   TARGET=aarch64-apple-darwin  bash build_bundle.sh      # CI macOS (runner)
set -euo pipefail

MODULE="$(grep -m1 '^id'      module.toml | sed -E 's/.*"([^"]+)".*/\1/')"
VERSION="$(grep -m1 '^version' Cargo.toml  | sed -E 's/.*"([^"]+)".*/\1/')"
[[ -n "$MODULE" && -n "$VERSION" ]] || { echo "Erreur : id/version introuvables." >&2; exit 1; }

# ── OS / arch cible (hôte par défaut ; surchargeables pour la CI) ────────────
detect_os()   { case "$(uname -s)" in Linux) echo linux;; Darwin) echo macos;; MINGW*|MSYS*|CYGWIN*) echo windows;; *) echo linux;; esac; }
detect_arch() { case "$(uname -m)" in x86_64|amd64) echo x86_64;; arm64|aarch64) echo arm64;; *) uname -m;; esac; }
BUNDLE_OS="${BUNDLE_OS:-$(detect_os)}"
RAW_ARCH="${BUNDLE_ARCH:-$(detect_arch)}"

# Étiquette d'arch normalisée pour coller aux suffixes attendus par le core.
case "$BUNDLE_OS-$RAW_ARCH" in
  windows-x86_64|windows-x64)     ARCH_LABEL=x64 ;;
  windows-arm64|windows-aarch64)  ARCH_LABEL=arm64 ;;
  macos-arm64|macos-aarch64)      ARCH_LABEL=arm64 ;;
  macos-x86_64)                   ARCH_LABEL=x86_64 ;;
  linux-arm64|linux-aarch64)      ARCH_LABEL=aarch64 ;;
  *)                              ARCH_LABEL="$RAW_ARCH" ;;
esac

EXT="tar.gz"; BIN_EXT=""
if [[ "$BUNDLE_OS" == "windows" ]]; then EXT="zip"; BIN_EXT=".exe"; fi

# ── Binaire (natif ou cross via $TARGET), compilé au besoin ──────────────────
if [[ -n "${TARGET:-}" ]]; then BIN="target/${TARGET}/release/kubuno-${MODULE}${BIN_EXT}"
else                            BIN="target/release/kubuno-${MODULE}${BIN_EXT}"; fi
if [[ ! -f "$BIN" ]]; then
  echo "→ Compilation de ${MODULE} (${TARGET:-natif})…"
  if [[ -n "${TARGET:-}" ]]; then SQLX_OFFLINE=true cargo build --release --target "$TARGET"
  else                            SQLX_OFFLINE=true cargo build --release; fi
fi
[[ -f "$BIN" ]] || { echo "Erreur : binaire introuvable ($BIN)." >&2; exit 1; }

# ── Frontend (buildé si présent et pas encore compilé) ───────────────────────
if [[ -f frontend/package.json && ! -f frontend/dist/entry.js ]]; then
  echo "→ Build du frontend…"; ( cd frontend && npm ci && npm run build )
fi

# ── Stage PLAT : <id>/… ──────────────────────────────────────────────────────
STAGE="$(mktemp -d)"; trap 'rm -rf "$STAGE"' EXIT
MODDIR="$STAGE/$MODULE"; mkdir -p "$MODDIR"
install -m 755 "$BIN" "$MODDIR/kubuno-${MODULE}${BIN_EXT}"
cp module.toml "$MODDIR/module.toml"
[[ -f config.toml.example ]] && cp config.toml.example "$MODDIR/config.toml.example"
[[ -d frontend/dist ]]       && cp -r frontend/dist "$MODDIR/frontend"

# ── Archive ──────────────────────────────────────────────────────────────────
mkdir -p dist
OUT="dist/kubuno-${MODULE}-${VERSION}-${BUNDLE_OS}-${ARCH_LABEL}.${EXT}"
rm -f "$OUT"
if [[ "$EXT" == "zip" ]]; then
  if   command -v zip >/dev/null; then ( cd "$STAGE" && zip -qr bundle.zip "$MODULE" ) && mv "$STAGE/bundle.zip" "$OUT"
  elif command -v 7z  >/dev/null; then ( cd "$STAGE" && 7z a -tzip -bso0 -bsp0 bundle.zip "$MODULE" >/dev/null ) && mv "$STAGE/bundle.zip" "$OUT"
  else echo "Erreur : ni 'zip' ni '7z' pour créer $OUT." >&2; exit 1; fi
else
  tar -czf "$OUT" -C "$STAGE" "$MODULE"
fi

# ── Empreinte SHA-256 (à publier à côté ; GitHub expose aussi asset.digest) ──
if   command -v sha256sum >/dev/null; then sha256sum "$OUT" | awk '{print $1}' > "$OUT.sha256"
elif command -v shasum    >/dev/null; then shasum -a 256 "$OUT" | awk '{print $1}' > "$OUT.sha256"; fi

echo "  ✓ $OUT"
[[ -f "$OUT.sha256" ]] && echo "  ✓ SHA-256 : $(cat "$OUT.sha256")"
