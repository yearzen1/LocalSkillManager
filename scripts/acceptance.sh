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

step="1/18 cargo test"
cargo test --quiet
ok

step="2/18 cargo build --release"
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

step="3/18 add sample to global"
"$lsm" add "$root/docs/sample.md" global >/dev/null
[[ -f "$tmp/agents/skills/sample/SKILL.md" ]] \
  || fail "missing $tmp/agents/skills/sample/SKILL.md"
ok

step="4/18 list global"
out=$("$lsm" list global)
[[ "$out" == *"sample"* ]] || fail "list global does not mention sample, got: $out"
ok

step="5/18 localized help"
out=$("$lsm" --lang zh --help)
[[ "$out" != *"Usage:"* ]] || fail "--lang zh --help is still English: $out"
ok

step="6/18 persist language"
"$lsm" config set lang zh >/dev/null
out=$("$lsm" config get lang)
[[ "$out" == "zh" ]] || fail "config get lang expected zh, got: $out"
ok

step="7/18 pull into a project and search"
out=$("$lsm" list global)
# The English header must be gone: proves config.toml took effect.
[[ "$out" != *"NAME"* ]] || fail "list global is still English after config set lang zh: $out"

mkdir -p "$tmp/demo"
cd "$tmp/demo"
"$lsm" pull sample >/dev/null
out=$(LSM_LANG=en "$lsm" list project)
[[ "$out" == *"sample"* ]] || fail "list project does not mention sample, got: $out"
out=$(LSM_LANG=en "$lsm" search sample project)
[[ "$out" == *"sample"* ]] || fail "search did not find sample, got: $out"
ok

step="8/18 pull multiple names"
printf -- '---\nname: beta\ndescription: Second skill.\n---\n\n# Body\n' >"$tmp/beta.md"
"$lsm" add "$tmp/beta.md" global >/dev/null
out=$(LSM_LANG=en "$lsm" pull beta sample)
[[ "$out" == *"pulled 1, skipped 1"* ]] || fail "multi pull summary unexpected, got: $out"
[[ -d "$tmp/demo/.agents/skills/beta" ]] || fail "multi pull did not install beta"
ok

step="9/18 pull --all skips existing"
out=$(LSM_LANG=en "$lsm" pull --all)
[[ "$out" == *"skipped 2"* ]] || fail "pull --all did not skip existing skills, got: $out"
ok

step="10/18 push a project skill"
printf -- '---\nname: gamma\ndescription: Project only.\n---\n\n# Body\n' >"$tmp/gamma.md"
"$lsm" add "$tmp/gamma.md" project >/dev/null
"$lsm" push gamma >/dev/null
[[ -f "$tmp/agents/skills/gamma/SKILL.md" ]] || fail "push did not install gamma globally"
out=$(LSM_LANG=en "$lsm" list global)
[[ "$out" == *"gamma"* ]] || fail "list global does not mention pushed gamma, got: $out"
ok

step="11/18 push --all skips existing"
out=$(LSM_LANG=en "$lsm" push --all)
[[ "$out" == *"skipped 3"* ]] || fail "push --all did not skip existing global skills, got: $out"
ok

step="12/18 remove multiple names"
LSM_LANG=en "$lsm" remove sample beta gamma project >/dev/null
out=$(LSM_LANG=en "$lsm" list project)
[[ "$out" != *"sample"* && "$out" != *"beta"* && "$out" != *"gamma"* ]] \
  || fail "remove left skills behind, got: $out"
ok

step="13/18 default scope is project"
printf -- '---\nname: delta\ndescription: Default scope.\n---\n\n# Body\n' >"$tmp/delta.md"
"$lsm" add "$tmp/delta.md" >/dev/null
[[ -f "$tmp/demo/.agents/skills/delta/SKILL.md" ]] \
  || fail "add without scope did not install into project"
out=$(LSM_LANG=en "$lsm" list)
[[ "$out" == *"delta"* ]] || fail "list without scope did not read project, got: $out"
ok

step="14/18 unique name prefix"
LSM_LANG=en "$lsm" pull samp >/dev/null
[[ -d "$tmp/demo/.agents/skills/sample" ]] || fail "pull samp did not resolve to sample"
LSM_LANG=en "$lsm" remove samp project >/dev/null
[[ ! -d "$tmp/demo/.agents/skills/sample" ]] || fail "remove samp did not resolve to sample"
ok

step="15/18 search field filters"
out=$(LSM_LANG=en "$lsm" search second --description global)
[[ "$out" == *"beta"* ]] || fail "description filter did not find beta, got: $out"
out=$(LSM_LANG=en "$lsm" search second --name global)
[[ "$out" != *"beta"* ]] || fail "name filter should not match the description, got: $out"
ok

step="16/18 localized possible values"
out=$("$lsm" --lang zh add --help)
[[ "$out" != *"possible values"* ]] || fail "zh add --help still contains English possible values: $out"
[[ "$out" == *"global, project"* ]] || fail "zh add --help lost the allowed scope values: $out"
ok

# Offline only: the real GitHub round-trip is a manual release step. Exercises
# the new subcommand without touching the network.
step="17/18 upgrade help and unknown flags (offline)"
out=$("$lsm" upgrade --help)
[[ "$out" == *"--check"* ]] || fail "upgrade --help does not list --check: $out"
code=0
"$lsm" upgrade --bogus >/dev/null 2>&1 || code=$?
[[ "$code" -eq 2 ]] || fail "upgrade --bogus exited $code, expected 2"
ok

step="18/18 copy command and -c flag are removed"
code=0
LSM_LANG=en "$lsm" copy sample >/dev/null 2>&1 || code=$?
[[ "$code" -eq 2 ]] || fail "copy exited $code, expected 2"
code=0
LSM_LANG=en "$lsm" -c sample >/dev/null 2>&1 || code=$?
[[ "$code" -eq 2 ]] || fail "-c exited $code, expected 2"
ok

printf 'PASS\n'
