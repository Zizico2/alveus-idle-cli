#!/usr/bin/env bash
set -u
cd "$(dirname "$0")/.."
export CARGO_TARGET_DIR=/home/bernardo/Projects/alveus-idle-cli/target

SCRIPTS=(
  scripts/headless_care_framework_smoke.py
  scripts/headless_typepath_smoke.py
  scripts/headless_nutrition_house_demo.py
  scripts/headless_playtest.py
  scripts/headless_feed_push_pop_demo.py
  scripts/headless_polly_care_demo.py
  scripts/headless_clean_push_pop_demo.py
  scripts/headless_care_feedback_audit.py
  scripts/headless_satchel_card_clarity_audit.py
)

RESULTS=/tmp/py_validation_results.txt
: > "$RESULTS"

kill_server() {
  pkill -9 -f 'alveus-idle-cli --headless' 2>/dev/null || true
  pkill -9 -f 'cargo run --features headless' 2>/dev/null || true
  sleep 1
}

wait_ready() {
  local i
  for i in $(seq 1 90); do
    if python3 -c "
import json, urllib.request
urllib.request.urlopen(
    urllib.request.Request(
        'http://127.0.0.1:15702/',
        data=json.dumps({'jsonrpc':'2.0','id':1,'method':'rpc.discover'}).encode(),
        headers={'Content-Type':'application/json'},
    ),
    timeout=2,
).read(10)
" 2>/dev/null; then
      sleep 2
      return 0
    fi
    sleep 1
  done
  return 1
}

restart_server() {
  kill_server
  rm -f save.ron
  cargo run --features headless -- --headless --realtime --port 15702 --no-stdio \
    >/tmp/headless_validation.log 2>&1 &
  if ! wait_ready; then
    echo "FAIL: server did not become ready" >&2
    tail -30 /tmp/headless_validation.log >&2 || true
    return 1
  fi
  return 0
}

for s in "${SCRIPTS[@]}"; do
  echo ""
  echo "========== RUNNING $s =========="
  if ! restart_server; then
    echo "$s FAIL server" | tee -a "$RESULTS"
    continue
  fi
  if python3 "$s"; then
    echo "$s PASS" | tee -a "$RESULTS"
  else
    echo "$s FAIL exit=$?" | tee -a "$RESULTS"
  fi
done

echo ""
echo "===== SUMMARY ====="
cat "$RESULTS"
fail_count=$(rg -c ' FAIL' "$RESULTS" 2>/dev/null || echo 0)
pass_count=$(rg -c ' PASS' "$RESULTS" 2>/dev/null || echo 0)
echo "pass=$pass_count fail=$fail_count"
kill_server
pgrep -af 'alveus-idle-cli --headless' || echo 'headless stopped'
exit "$fail_count"
