#!/usr/bin/env bash
set -euo pipefail

# Usage: scripts/chat_insight.sh <id> <summary> <details> [tags_csv] [source_event_cid] [path=dialog-dag.json]

ID=${1:-}
SUMMARY=${2:-}
DETAILS=${3:-}
TAGS=${4:-}
SRC=${5:-}
PATH_JSON=${6:-dialog-dag.json}

if [[ -z "$ID" || -z "$SUMMARY" || -z "$DETAILS" ]]; then
  echo "Usage: $0 <id> <summary> <details> [tags_csv] [source_event_cid] [path]" >&2
  exit 2
fi

ARGS=( --file "$PATH_JSON" --id "$ID" --summary "$SUMMARY" --details "$DETAILS" )
if [[ -n "${TAGS:-}" ]]; then ARGS+=( --tags "$TAGS" ); fi
if [[ -n "${SRC:-}" ]]; then ARGS+=( --source "$SRC" ); fi

cargo run -q -p dialog_dag_tools --bin log_insight -- "${ARGS[@]}" || {
  echo "Failed to log insight" >&2
  exit 1
}

