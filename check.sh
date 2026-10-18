#!/usr/bin/env bash
# The one gate: everything the project can check, in dependency order.
# Any code/data change should end with `./check.sh` (fast) and, before
# calling anything done, `./check.sh --full` (adds the exhaustive audits).
#
#   fmt + clippy (pedantic deny)
#   cargo test (unit + integration + sampled audits)
#   [--full] cargo test --release -- --ignored   (exhaustive: ~3 min)
#   wasm-pack build ×3 (browser, node, served dist)  → drift impossible
#   refresh dist shell (web/*)
#   node --test web/tests/                        (render contracts)
#   node web/e2e-dist.check.cjs                   (36 portal-flow checks vs dist)
#   node web/smoke.cjs                            (every wasm export the app calls)
#   browser self-test (headless Firefox)          (DOM flows that regressed)
#
# Not covered here (manual, documented in PROGRESS.md): rebuilding the heavy
# artifacts (`build-site`, shards, blobs) after engine/data changes, and
# Sanskrit-correctness judgments that only you can confirm.
set -euo pipefail
cd "$(dirname "$0")"
full=0
[ "${1:-}" = "--full" ] && full=1

echo "== fmt =="
cargo fmt --check

echo "== clippy (pedantic deny via Cargo.toml [lints]; no CLI -D so vendored deps stay capped) =="
cargo clippy --all-targets

echo "== cargo test (unit + integration + sampled audits) =="
cargo test

if [ "$full" = 1 ]; then
  echo "== exhaustive audits (release, ~3 min) =="
  cargo test --release -- --ignored
fi

echo "== wasm builds (all targets, so pkg/ can never drift) =="
wasm-pack build --target web --out-dir pkg >/dev/null
wasm-pack build --target nodejs --out-dir pkg-node >/dev/null
wasm-pack build --target web --out-dir dist/pkg >/dev/null

echo "== dist shell refresh =="
cp web/app.html dist/index.html
cp web/app.js web/render.js web/sw.js dist/
cp web/selftest.html web/selftest.js dist/
# Served शब्द oracle (word view): rebuild from data/shabda_data2.json when the
# dump is present; the script keeps an already-built file otherwise.
node web/build-shabda.cjs
# Stray duplicate from an old manual copy: app.html is not a dist artifact
# (the shell ships as index.html); a stale copy served the old upasarga box.
rm -f dist/app.html

echo "== node render tests =="
node --test web/tests/

echo "== portal-flow e2e (dist artifacts) =="
node web/e2e-dist.check.cjs

echo "== wasm export smoke =="
node web/smoke.cjs

echo "== browser self-test (headless Chromium via chromedriver) =="
port=8124
node web/serve.cjs dist "$port" >/dev/null 2>&1 &
server=$!
trap 'kill "$server" 2>/dev/null || true' EXIT
sleep 1
node web/browser_check.cjs "http://127.0.0.1:$port/selftest.html"

echo
echo "ALL CHECKS PASSED"
