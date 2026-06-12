#!/usr/bin/env bash
# Local CI — mirrors .github/workflows/ci.yml so green here = green there.
#
#   scripts/ci-local.sh            run all gates
#   scripts/ci-local.sh --fast     check + unit tests only (pre-push speed)
#
# Exit code is non-zero if any gate fails; a summary prints at the end.
set -u

FAST=0
[ "${1:-}" = "--fast" ] && FAST=1

declare -a NAMES RESULTS
run_gate() {
    local name="$1"; shift
    echo ""
    echo "==> ${name}: $*"
    local start=$SECONDS
    if "$@"; then
        RESULTS+=("ok"); NAMES+=("${name} ($((SECONDS - start))s)")
    else
        RESULTS+=("FAIL"); NAMES+=("${name} ($((SECONDS - start))s)")
    fi
}

# CI_WORKDIR lets the post-receive hook point at its own checkout
cd "${CI_WORKDIR:-$(dirname "$0")/..}"

# No --all-features: the `ebpf` feature needs nightly + bpf-linker and the
# crush-ebpf-progs crate still uses the pre-rename aya-bpf deps (unbuildable
# from clean). Re-add once the crate is ported to aya-ebpf.
run_gate "check" cargo check --workspace --all-targets
run_gate "test"  cargo test --workspace --lib --exclude crush-ebpf-progs

if [ "$FAST" -eq 0 ]; then
    run_gate "clippy" cargo clippy --workspace --all-targets --exclude crush-ebpf-progs -- -D warnings
    run_gate "fmt"    cargo fmt --all -- --check
fi

echo ""
echo "================ CI summary ================"
rc=0
for i in "${!NAMES[@]}"; do
    if [ "${RESULTS[$i]}" = "ok" ]; then
        printf "  \033[32mPASS\033[0m  %s\n" "${NAMES[$i]}"
    else
        printf "  \033[31mFAIL\033[0m  %s\n" "${NAMES[$i]}"
        rc=1
    fi
done
echo "============================================"
exit $rc
