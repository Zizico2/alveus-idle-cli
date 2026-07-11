#!/usr/bin/env bash
# Run headless Python validation drivers against a wrapper-owned server.
# Tracks only the PID it starts (no broad pkill). Backs up/restores save.ron.
set -u
cd "$(dirname "$0")/.."
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$(pwd)/target}"

# Never skip audits for missing servers in this wrapper.
unset ALLOW_MISSING_SERVER
unset REQUIRE_SERVER

SCRIPTS=(
  scripts/headless_care_framework_smoke.py
  scripts/headless_typepath_smoke.py
  scripts/headless_nutrition_house_demo.py
  scripts/headless_playtest.py
  scripts/headless_feed_push_pop_demo.py
  scripts/headless_polly_care_demo.py
  scripts/headless_clean_push_pop_demo.py
  "scripts/headless_care_feedback_audit.py feed"
  "scripts/headless_care_feedback_audit.py clean"
  "scripts/headless_care_feedback_audit.py enrich"
  scripts/headless_satchel_card_clarity_audit.py
)

RESULTS=/tmp/py_validation_results.txt
: > "$RESULTS"

SERVER_PID=""
USER_SAVE_BACKUP=""
SAVE_FILE="$(pwd)/save.ron"

backup_user_save() {
  if [[ -z "$USER_SAVE_BACKUP" ]]; then
    if [[ -f "$SAVE_FILE" ]]; then
      USER_SAVE_BACKUP=$(mktemp /tmp/alveus_save_backup.XXXXXX.ron)
      cp "$SAVE_FILE" "$USER_SAVE_BACKUP"
    else
      USER_SAVE_BACKUP="__none__"
    fi
  fi
}

restore_user_save() {
  if [[ "$USER_SAVE_BACKUP" == "__none__" ]]; then
    rm -f "$SAVE_FILE"
  elif [[ -n "$USER_SAVE_BACKUP" && -f "$USER_SAVE_BACKUP" ]]; then
    mv -f "$USER_SAVE_BACKUP" "$SAVE_FILE"
    USER_SAVE_BACKUP=""
  fi
}

kill_tree() {
  local pid=$1
  local child
  for child in $(pgrep -P "$pid" 2>/dev/null || true); do
    kill_tree "$child"
  done
  kill "$pid" 2>/dev/null || true
}

stop_owned_server() {
  if [[ -n "${SERVER_PID:-}" ]]; then
    kill_tree "$SERVER_PID"
    wait "$SERVER_PID" 2>/dev/null || true
    SERVER_PID=""
    sleep 1
  fi
}

cleanup() {
  stop_owned_server
  restore_user_save
}
trap cleanup EXIT INT TERM

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
  stop_owned_server
  backup_user_save
  rm -f "$SAVE_FILE"
  cargo run --features headless -- --headless --realtime --port 15702 --no-stdio \
    >/tmp/headless_validation.log 2>&1 &
  SERVER_PID=$!
  if ! wait_ready; then
    echo "FAIL: server did not become ready (pid=${SERVER_PID:-none})" >&2
    tail -30 /tmp/headless_validation.log >&2 || true
    stop_owned_server
    return 1
  fi
  return 0
}

fail_count=0
pass_count=0

for entry in "${SCRIPTS[@]}"; do
  # shellcheck disable=SC2206
  cmd=($entry)
  label=$entry
  echo ""
  echo "========== RUNNING $label =========="
  if ! restart_server; then
    echo "$label FAIL server" | tee -a "$RESULTS"
    fail_count=$((fail_count + 1))
    continue
  fi
  if python3 "${cmd[@]}"; then
    echo "$label PASS" | tee -a "$RESULTS"
    pass_count=$((pass_count + 1))
  else
    echo "$label FAIL exit=$?" | tee -a "$RESULTS"
    fail_count=$((fail_count + 1))
  fi
done

echo ""
echo "===== SUMMARY ====="
cat "$RESULTS"
echo "pass=$pass_count fail=$fail_count"

# Explicit stop before exit trap restore; trap still runs restore_user_save.
stop_owned_server
if pgrep -af 'alveus-idle-cli.*--headless' >/dev/null 2>&1; then
  echo "WARN: a headless process is still running (not started by this wrapper?)" >&2
  pgrep -af 'alveus-idle-cli.*--headless' || true
else
  echo "headless stopped"
fi

exit "$fail_count"
