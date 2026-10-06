#!/bin/bash
# 링링이 릴리스 빌드 — 검사, 버전 반영, .app 과 .dmg 만들기. 서명과 notarization 은 하지 않는다. Apple Silicon 전용
#
# Usage:
#   ./scripts/build.sh <version>            build, verify → release/<version>/
#   ./scripts/build.sh <version> --dry-run  every check and a release compile, but nothing is
#                                           bundled or written to the version files
#   ./scripts/build.sh <version> --smoke    like --dry-run, but the binary is compiled with the dev
#                                           identifier and the dev-agent, so the production frontend
#                                           (embedded assets, CSP) can be run and looked at. It uses
#                                           the dev data folder, never the installed app's
#
# What it does NOT do: code signing, notarization, auto-update artifacts, uploads, git commits, tags.
#
# The app is not signed with a Developer ID and is not notarized. Gatekeeper stops a downloaded copy
# the first time it is opened.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
TAURI_DIR="$PROJECT_DIR/src-tauri"
TARGET="aarch64-apple-darwin"
# Must match tauri.conf.json bundle.macOS.minimumSystemVersion.
DEPLOYMENT_TARGET="13.0"
DEV_AGENT_PORT=9797

RED="\033[0;31m"
GREEN="\033[0;32m"
YELLOW="\033[1;33m"
NC="\033[0m"

step() { echo -e "${GREEN}▶${NC} $1"; }
ok() { echo -e "${GREEN}✓${NC} $1"; }
warn() { echo -e "${YELLOW}!${NC} $1"; }
die() {
	echo -e "${RED}✗${NC} $1" >&2
	exit 1
}

# ── 1. Arguments ─────────────────────────────────────────────────────────────

VERSION=""
DRY_RUN=0
SMOKE=0
for arg in "$@"; do
	case "$arg" in
	--dry-run) DRY_RUN=1 ;;
	--smoke)
		DRY_RUN=1
		SMOKE=1
		;;
	-h | --help)
		sed -n '2,16p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
		exit 0
		;;
	-*) die "Unknown option: $arg" ;;
	*)
		[ -z "$VERSION" ] || die "Only one version may be given."
		VERSION="$arg"
		;;
	esac
done

[ -n "$VERSION" ] || die "Version is required. Usage: $0 <version> [--dry-run]"
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "Invalid version: $VERSION (expected X.Y.Z)"

if [ "$DRY_RUN" -eq 1 ]; then
	warn "Dry run: nothing is bundled or written to the version files."
fi

# ── 2. Preconditions ─────────────────────────────────────────────────────────

[ "$(uname -s)" = "Darwin" ] || die "This script builds on macOS only."
[ "$(uname -m)" = "arm64" ] || die "This script builds on Apple Silicon only."

for tool in bun cargo jq plutil hdiutil ditto shasum; do
	command -v "$tool" >/dev/null 2>&1 || die "$tool is not installed."
done
cargo tauri --version >/dev/null 2>&1 || die "cargo-tauri is not installed (cargo install tauri-cli)."

# A dev build watches dist/ and holds the cargo lock on the same target/. Stop it first.
if lsof -nP -iTCP:"$DEV_AGENT_PORT" -sTCP:LISTEN >/dev/null 2>&1 ||
	pgrep -f "$TAURI_DIR/target/debug/ringring" >/dev/null 2>&1; then
	die "A dev build of RingRing is running. Stop it before a release build."
fi
ok "Tools present, no dev build running"

# Tauri's bundler signs and notarizes by itself when it finds these in the environment.
# This build never does either, whatever the shell has exported.
unset APPLE_SIGNING_IDENTITY APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID \
	APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD \
	APPLE_API_KEY APPLE_API_ISSUER APPLE_API_KEY_PATH

# ── 3. Checks ────────────────────────────────────────────────────────────────
#
# Before anything is written. They run against the current version.

cd "$PROJECT_DIR"

step "Installing dependencies..."
bun install --frozen-lockfile

step "Type check, lint, tests..."
bun run typecheck
bun run lint
bun run test
(cd "$TAURI_DIR" && cargo fmt --check && cargo test --quiet)
ok "tsc, biome, bun test, rustfmt and cargo test pass"

# ── 4. Version in the three files ────────────────────────────────────────────
#
# Only after the checks pass: a failed check must not leave edited version files behind.
# Cargo.lock follows on the next cargo command (the build below) — it carries the version too.

CURRENT_VERSION="$(jq -r .version "$TAURI_DIR/tauri.conf.json")"
if [ "$DRY_RUN" -eq 1 ]; then
	ok "Would set the version: $CURRENT_VERSION → $VERSION (package.json, Cargo.toml, tauri.conf.json; Cargo.lock follows)"
elif [ "$CURRENT_VERSION" != "$VERSION" ] ||
	[ "$(jq -r .version package.json)" != "$VERSION" ] ||
	! grep -q "^version = \"$VERSION\"$" "$TAURI_DIR/Cargo.toml"; then
	step "Setting the version to $VERSION..."
	TMP_JSON="$(mktemp)"
	jq --arg v "$VERSION" '.version = $v' package.json >"$TMP_JSON" && cat "$TMP_JSON" >package.json
	jq --arg v "$VERSION" '.version = $v' "$TAURI_DIR/tauri.conf.json" >"$TMP_JSON" &&
		cat "$TMP_JSON" >"$TAURI_DIR/tauri.conf.json"
	rm -f "$TMP_JSON"
	# Only the [package] version: the first `version = "…"` line of Cargo.toml.
	sed -i '' "1,/^version = \".*\"$/s/^version = \".*\"$/version = \"$VERSION\"/" "$TAURI_DIR/Cargo.toml"
	ok "Version set in package.json, Cargo.toml and tauri.conf.json"
else
	ok "Version is already $VERSION"
fi

# CHANGELOG: promote [Unreleased] when the version has no section yet.
TODAY="$(date +%Y-%m-%d)"
if grep -q "^## \[$VERSION\]" CHANGELOG.md; then
	ok "CHANGELOG.md has a [$VERSION] section"
elif grep -q "^## \[Unreleased\]" CHANGELOG.md; then
	if [ "$DRY_RUN" -eq 1 ]; then
		ok "Would promote [Unreleased] to [$VERSION] - $TODAY in CHANGELOG.md"
	else
		sed -i '' "s/^## \[Unreleased\]/## [Unreleased]\\
\\
## [$VERSION] - $TODAY/" CHANGELOG.md
		ok "Promoted [Unreleased] to [$VERSION] - $TODAY"
	fi
else
	die "CHANGELOG.md has neither a [$VERSION] nor an [Unreleased] section."
fi

export MACOSX_DEPLOYMENT_TARGET="$DEPLOYMENT_TARGET"
# rustc writes the absolute path of every source file into the binary (panic locations).
# That path holds the builder's home folder, so the account name would ship. Remap both roots.
# The separator is 0x1f, so a path with a space still works.
export CARGO_ENCODED_RUSTFLAGS="--remap-path-prefix=$HOME=/home"$'\x1f'"--remap-path-prefix=$PROJECT_DIR=/ringring"

# ── 5. Dry run stops here ────────────────────────────────────────────────────

if [ "$DRY_RUN" -eq 1 ]; then
	SMOKE_ARGS=()
	if [ "$SMOKE" -eq 1 ]; then
		# The dev identifier keeps this binary's data, log and single-instance socket away from an
		# installed copy. The dev-agent lets it be driven without OS input.
		SMOKE_ARGS=(--features dev-agent --config "$TAURI_DIR/tauri.dev.conf.json")
	fi
	step "Compiling the release binary without bundling..."
	cargo tauri build --target "$TARGET" --no-bundle "${SMOKE_ARGS[@]+"${SMOKE_ARGS[@]}"}"
	ok "Release binary compiled: src-tauri/target/$TARGET/release/ringring"
	if [ "$SMOKE" -eq 1 ]; then
		echo ""
		ok "Smoke build finished. It is NOT the binary that ships (dev identifier, dev-agent)."
		echo "  Run it with:"
		echo "    RINGRING_DEV_AGENT=1 RINGRING_DEV_QUIET=1 src-tauri/target/$TARGET/release/ringring"
		echo "  It registers the global shortcuts stored in the dev data folder. Clear them first"
		echo "  if they could shadow a shortcut you use."
		exit 0
	fi
	echo ""
	ok "Dry run finished. A real build would now run:"
	echo "    cargo tauri build --target $TARGET --bundles app,dmg"
	echo "  then check the bundle's version and LSUIElement,"
	echo "  and copy RingRing.app (zipped) and the .dmg to release/$VERSION/."
	exit 0
fi

# ── 6. Build ─────────────────────────────────────────────────────────────────

BUNDLE_DIR="$TAURI_DIR/target/$TARGET/release/bundle"
rm -rf "$BUNDLE_DIR"

step "Building the app and the DMG..."
# CI=true makes Tauri's DMG step skip the Finder layout script. That script drives Finder through
# AppleScript and would ask for Automation permission.
CI=true cargo tauri build --target "$TARGET" --bundles app,dmg

APP="$BUNDLE_DIR/macos/RingRing.app"
DMG="$(find "$BUNDLE_DIR/dmg" -maxdepth 1 -name '*.dmg' -print -quit)"
[ -d "$APP" ] || die "The app bundle was not produced: $APP"
[ -n "$DMG" ] && [ -f "$DMG" ] || die "The DMG was not produced."

# ── 7. Verify ────────────────────────────────────────────────────────────────

step "Verifying the bundle..."
[ "$(plutil -extract CFBundleShortVersionString raw "$APP/Contents/Info.plist")" = "$VERSION" ] ||
	die "The bundle version is not $VERSION."
[ "$(plutil -extract LSUIElement raw "$APP/Contents/Info.plist")" = "true" ] ||
	die "LSUIElement is missing — the app would show a Dock icon."
ok "Version, LSUIElement"

# ── 8. Collect ───────────────────────────────────────────────────────────────

OUT_DIR="$PROJECT_DIR/release/$VERSION"
rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"
# ditto zips the bundle the way Finder does: symlinks and extended attributes survive.
ditto -c -k --keepParent "$APP" "$OUT_DIR/RingRing-$VERSION-arm64.zip"
cp "$DMG" "$OUT_DIR/RingRing-$VERSION-arm64.dmg"
(cd "$OUT_DIR" && shasum -a 256 ./*.zip ./*.dmg >SHA256SUMS)

echo ""
ok "RingRing $VERSION is in release/$VERSION/"
ls -lh "$OUT_DIR"
echo ""
warn "Nothing was committed. Commit package.json, src-tauri/Cargo.toml, src-tauri/Cargo.lock,"
warn "src-tauri/tauri.conf.json and CHANGELOG.md yourself."
