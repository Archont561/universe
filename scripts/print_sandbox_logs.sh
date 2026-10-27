#!/usr/bin/env bash
# Surface the tail of pixi-sandbox's failure logs as GitHub annotations. The
# artifact remains the complete diagnostic record; this gives CI-only users a
# concise clue without depending on artifact-download access.
set -euo pipefail

log_dir="${1:?usage: print_sandbox_logs.sh LOG_DIR}"
if [[ ! -d "$log_dir" ]]; then
  echo "::notice::No pixi-sandbox diagnostic directory was created."
  exit 0
fi

shopt -s nullglob
logs=("$log_dir"/*)
if ((${#logs[@]} == 0)); then
  echo "::notice::No pixi-sandbox diagnostic files were created."
  exit 0
fi

for log in "${logs[@]}"; do
  [[ -f "$log" ]] || continue
  while IFS= read -r line || [[ -n "$line" ]]; do
    line="${line//'%'/'%25'}"
    line="${line//$'\r'/'%0D'}"
    line="${line//':'/'%3A'}"
    line="${line//','/'%2C'}"
    printf '::error title=Pixi sandbox diagnostics::%s\n' "$line"
  done < <(tail -n 80 "$log")
done
