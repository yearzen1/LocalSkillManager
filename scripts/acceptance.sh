#!/usr/bin/env bash
#
# End-to-end acceptance script for lsm. Mirrors docs/impl/quality.md "1.3"
# step by step; the script is the executable copy, the doc explains the why.
#
# Usage (Windows): "D:/Git/bin/bash.exe" scripts/acceptance.sh
# Usage (Linux/macOS): bash scripts/acceptance.sh
#
# This script is intentionally ASCII-only and asserts on ASCII markers
# (`NAME`, `Usage:`) instead of localized strings, so a wrong file encoding
# can never turn a failed assertion into a pass.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

step="init"
fail() {
  printf 'FAIL [%s]: %s\n' "$step" "$1" >&2
  exit 1
}
ok() {
  printf 'ok   %s\n' "$step"
}

# Paths handed to the binary as environment variables must be native Windows
# paths; paths used by this shell stay POSIX.
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*)
    win() { cygpath -w "$1"; }
    ;;
  Linux)
    if grep -qi microsoft /proc/version 2>/dev/null; then
      printf 'run this with Git Bash, not WSL: "D:/Git/bin/bash.exe" scripts/acceptance.sh\n' >&2
      exit 2
    fi
    win() { printf '%s' "$1"; }
    ;;
  *)
    win() { printf '%s' "$1"; }
    ;;
esac

step="1/9 cargo test"
cargo test --quiet
ok

step="2/9 cargo build --release"
cargo build --release --quiet
if [[ -x "$root/target/release/lsm.exe" ]]; then
  lsm="$root/target/release/lsm.exe"
elif [[ -x "$root/target/release/lsm" ]]; then
  lsm="$root/target/release/lsm"
else
  fail "release binary not found under target/release"
fi
ok

# Isolate every piece of state: skills root, config dir, language.
export LSM_HOME="$(win "$tmp/agents")"
export LSM_CONFIG_DIR="$(win "$tmp/config")"
unset LSM_LANG LSM_UPGRADE_API

step="3/9 add sample to global"
"$lsm" add "$root/docs/sample.md" global >/dev/null
[[ -f "$tmp/agents/skills/sample/SKILL.md" ]] \
  || fail "missing $tmp/agents/skills/sample/SKILL.md"
ok

step="4/9 list global"
out=$("$lsm" list global)
[[ "$out" == *"sample"* ]] || fail "list global does not mention sample, got: $out"
ok

step="5/9 localized help"
out=$("$lsm" --lang zh --help)
[[ "$out" != *"Usage:"* ]] || fail "--lang zh --help is still English: $out"
ok

step="6/9 persist language"
"$lsm" config set lang zh >/dev/null
out=$("$lsm" config get lang)
[[ "$out" == "zh" ]] || fail "config get lang expected zh, got: $out"
ok

step="7/9 copy into a project"
out=$("$lsm" list global)
# The English header must be gone: proves config.toml took effect.
[[ "$out" != *"NAME"* ]] || fail "list global is still English after config set lang zh: $out"

mkdir -p "$tmp/demo"
cd "$tmp/demo"
"$lsm" copy sample >/dev/null
out=$("$lsm" list project)
[[ "$out" == *"sample"* ]] || fail "list project does not mention sample, got: $out"
ok

step="8/9 copy --all skips existing"
out=$(LSM_LANG=en "$lsm" copy --all)
[[ "$out" == *"skipped 1"* ]] || fail "copy --all did not skip the existing skill, got: $out"
ok

step="9/9 remove from project"
LSM_LANG=en "$lsm" remove sample project >/dev/null
[[ ! -d "$tmp/demo/.agents/skills/sample" ]] \
  || fail "remove left $tmp/demo/.agents/skills/sample behind"
out=$("$lsm" list project)
[[ "$out" != *"sample"* ]] || fail "list project still mentions sample, got: $out"
ok

printf 'PASS\n'
